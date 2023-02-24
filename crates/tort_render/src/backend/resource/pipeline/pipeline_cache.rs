use std::{sync::Arc};


use concurrent_queue::ConcurrentQueue;
use parking_lot::{Mutex, RwLock};
use tort_asset::{AssetEvent, AssetPath, Assets, Handle};
use tort_ecs::{
    self as bevy_ecs,
    event::EventReader,
    system::{Res, ResMut, Resource},
};
use tort_tasks::AsyncComputeTaskPool;
use tort_utils::{smallvec::SmallVec4, HashMap, HashSet, PlainUnwrap, Uuid};

use crate::{
    backend::{
        resource::{
            descriptor::{DescriptorSetLayout, DescriptorSetLayoutDesc},
            pipeline::{
                ComputePipeline, ComputePipelineDesc, ComputePipelineId, GraphicsPipeline,
                GraphicsPipelineDesc, GraphicsPipelineId, Pipeline, PipelineLayout,
                PipelineLayoutDesc, Shader, ShaderModule, ShaderModuleDesc, ShaderSource,
                ShaderStageDesc,
            },
            Sampler, SamplerDesc,
        },
        utils::BackendError,
        Device,
    },
    Extract,
};

struct Inner {
    immutable_samplers: Mutex<HashMap<SamplerDesc, Arc<Sampler>>>,
    descriptor_set_layouts: Mutex<HashMap<DescriptorSetLayoutDesc, Arc<DescriptorSetLayout>>>,
    pipeline_layouts: Mutex<HashMap<PipelineLayoutDesc, Arc<PipelineLayout>>>,

    shaders: RwLock<HashMap<Handle<Shader>, Shader>>,
    shader_paths: RwLock<HashMap<AssetPath<'static>, Handle<Shader>>>,

    spirv_modules: Mutex<HashMap<Handle<Shader>, Arc<ShaderModule>>>,

    ready_graphics_pipelines: ConcurrentQueue<GraphicsPipeline>,
    ready_compute_pipelines: ConcurrentQueue<ComputePipeline>,

    device: Device,
}

impl Inner {
    fn new(device: Device) -> Self {
        Self {
            immutable_samplers: Mutex::new(HashMap::new()),
            descriptor_set_layouts: Mutex::new(HashMap::new()),
            pipeline_layouts: Mutex::new(HashMap::new()),

            shaders: RwLock::new(HashMap::new()),
            shader_paths: RwLock::new(HashMap::new()),

            spirv_modules: Mutex::new(HashMap::new()),

            ready_graphics_pipelines: ConcurrentQueue::unbounded(),
            ready_compute_pipelines: ConcurrentQueue::unbounded(),

            device,
        }
    }

    fn get_immutable_sampler(&self, desc: &SamplerDesc) -> Result<Arc<Sampler>, BackendError> {
        if let Some(sampler) = {
            let immutable_samplers = self.immutable_samplers.lock();
            immutable_samplers.get(desc).cloned()
        } {
            Ok(sampler)
        } else {
            let sampler = Arc::new(Sampler::new(self.device.clone(), desc)?);

            self.immutable_samplers
                .lock()
                .insert(desc.clone(), sampler.clone());
            Ok(sampler)
        }
    }

    fn get_descriptor_set_layout(
        &self,
        desc: &DescriptorSetLayoutDesc,
    ) -> Result<Arc<DescriptorSetLayout>, BackendError> {
        if let Some(descriptor_set_layout) = {
            let descriptor_set_layouts = self.descriptor_set_layouts.lock();
            descriptor_set_layouts.get(desc).cloned()
        } {
            Ok(descriptor_set_layout)
        } else {
            let descriptor_set_layout = Arc::new(DescriptorSetLayout::new(
                self.device.clone(),
                desc,
                |sampler_desc| self.get_immutable_sampler(sampler_desc),
            )?);

            self.descriptor_set_layouts
                .lock()
                .insert(desc.clone(), descriptor_set_layout.clone());
            Ok(descriptor_set_layout)
        }
    }

    fn get_pipeline_layout(
        &self,
        desc: &PipelineLayoutDesc,
    ) -> Result<Arc<PipelineLayout>, BackendError> {
        if let Some(pipeline_layout) = {
            let pipeline_layouts = self.pipeline_layouts.lock();
            pipeline_layouts.get(desc).cloned()
        } {
            Ok(pipeline_layout)
        } else {
            let pipeline_layout = Arc::new(PipelineLayout::new(
                self.device.clone(),
                desc,
                |set_layout_desc| self.get_descriptor_set_layout(set_layout_desc),
            )?);

            self.pipeline_layouts
                .lock()
                .insert(desc.clone(), pipeline_layout.clone());
            Ok(pipeline_layout)
        }
    }

    fn get_shader_module(
        &self,
        stage_desc: &ShaderStageDesc,
        shader: &Shader,
    ) -> Result<Arc<ShaderModule>, BackendError> {
        match shader.source() {
            ShaderSource::SpirV(spirv) => {
                if let Some(shader_module) =
                    self.spirv_modules.lock().get(&stage_desc.shader).cloned()
                {
                    Ok(shader_module)
                } else {
                    let shader_module = Arc::new(ShaderModule::new(
                        self.device.clone(),
                        &ShaderModuleDesc {
                            label: Some(shader.path().path().to_str().unwrap()),
                            code: spirv,
                            ..Default::default()
                        },
                    )?);

                    self.spirv_modules
                        .lock()
                        .insert(stage_desc.shader.clone_weak(), shader_module.clone());
                    Ok(shader_module)
                }
            }
        }
    }

    #[inline]
    fn create_shader(&self, handle: &Handle<Shader>, shader: &Shader) {
        self.shaders.write().insert(handle.clone(), shader.clone());
        self.shader_paths
            .write()
            .insert(shader.path().clone(), handle.clone_weak());
    }

    #[inline]
    fn modify_shader(&self, handle: &Handle<Shader>, shader: &Shader) {
        self.shaders
            .write()
            .entry_ref(handle)
            .and_modify(|s| *s = shader.clone());
    }

    fn remove_shader(&self, handle: &Handle<Shader>) {
        self.shaders.write().remove(handle);
    }
}

struct Pipelines<P: Pipeline> {
    pipelines: HashMap<P::Id, P>,
    ids: HashMap<P::Desc, P::Id>,
    queued: Vec<P::Desc>,

    shader_to_pipeline: HashMap<Handle<Shader>, HashSet<P::Id>>,
}

impl<P: Pipeline> Pipelines<P> {
    fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
            ids: HashMap::new(),
            queued: Vec::new(),

            shader_to_pipeline: HashMap::new(),
        }
    }

    fn queue(&mut self, desc: &P::Desc) -> P::Id {
        *self.ids.entry_ref(desc).or_insert_with(|| {
            let mut id = P::Id::from(Uuid::new_v4());
            while self.pipelines.contains_key(&id) {
                id = P::Id::from(Uuid::new_v4());
            }

            self.queued.push(desc.clone());
            id
        })
    }

    #[inline]
    fn get(&self, id: &P::Id) -> Option<&P> {
        self.pipelines.get(id)
    }
}

#[derive(Resource)]
pub struct PipelineCache {
    inner: Arc<Inner>,

    modified_shaders: Vec<Handle<Shader>>,

    graphics_pipelines: Pipelines<GraphicsPipeline>,
    compute_pipelines: Pipelines<ComputePipeline>,
}

impl PipelineCache {
    pub fn new(device: Device) -> Self {
        Self {
            inner: Arc::new(Inner::new(device)),

            modified_shaders: Vec::new(),

            graphics_pipelines: Pipelines::new(),
            compute_pipelines: Pipelines::new(),
        }
    }

    #[inline]
    fn create_shader(&self, handle: &Handle<Shader>, shader: &Shader) {
        self.inner.create_shader(handle, shader);
    }

    #[inline]
    fn modify_shader(&mut self, handle: &Handle<Shader>, shader: &Shader) {
        self.inner.modify_shader(handle, shader);
        self.modified_shaders.push(handle.clone_weak());
    }

    #[inline]
    fn remove_shader(&self, handle: &Handle<Shader>) {
        self.inner.remove_shader(handle);
    }

    fn process_graphics_pipelines(&mut self) {
        let mut waited_idle = false;

        while let Ok(graphics_pipeline) = self.inner.ready_graphics_pipelines.pop() {
            if self
                .graphics_pipelines
                .pipelines
                .contains_key(graphics_pipeline.id())
            {
                if !waited_idle {
                    unsafe { self.inner.device.loader().device_wait_idle() }.unwrap();
                    waited_idle = true;
                }

                self.graphics_pipelines
                    .pipelines
                    .entry(*graphics_pipeline.id())
                    .and_modify(|p| *p = graphics_pipeline);
            } else {
                self.graphics_pipelines
                    .pipelines
                    .insert(*graphics_pipeline.id(), graphics_pipeline);
            }
        }

        for modified_shader in &self.modified_shaders {
            if let Some(graphics_pipelines) = self
                .graphics_pipelines
                .shader_to_pipeline
                .get(modified_shader)
            {
                for graphics_pipeline in graphics_pipelines {
                    let desc = self.graphics_pipelines.pipelines[graphics_pipeline].desc();
                    self.graphics_pipelines.queued.push(desc.clone());
                }
            }
        }

        self.graphics_pipelines.queued.retain(|desc| {
            let Some(shaders) = ({
                let shaders = self.inner.shaders.read();
                desc.stages.iter().map(|stage_desc| shaders.get(&stage_desc.shader).cloned()).collect::<Option<SmallVec4<_>>>()
            }) else {
                return true;
            };

            let desc = desc.clone();
            let id = self.graphics_pipelines.ids[&desc];
            let inner = self.inner.clone();

            for stage_desc in &desc.stages {
                let pipelines = self.graphics_pipelines.shader_to_pipeline.entry(stage_desc.shader.clone_weak()).or_insert_with(HashSet::new);
                pipelines.insert(id);
            }

            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let shader_modules = desc
                        .stages
                        .iter()
                        .zip(shaders.into_iter())
                        .map(|(stage_desc, shader)| inner.get_shader_module(stage_desc, &shader))
                        .collect::<Result<SmallVec4<_>, _>>()
                        .unwrap(); //TODO:

                    let graphics_pipeline = GraphicsPipeline::new(inner.device.clone(), &desc, id, &shader_modules, |layout_desc| inner.get_pipeline_layout(layout_desc)).unwrap(); //TODO:
                    inner.ready_graphics_pipelines.push(graphics_pipeline).plain_unwrap();
                })
                .detach();

            false
        });
    }

    fn process_compute_pipelines(&mut self) {
        let mut waited_idle = false;

        while let Ok(compute_pipeline) = self.inner.ready_compute_pipelines.pop() {
            if self
                .compute_pipelines
                .pipelines
                .contains_key(compute_pipeline.id())
                && !waited_idle
            {
                unsafe { self.inner.device.loader().device_wait_idle() }.unwrap();
                waited_idle = true;
            }

            self.compute_pipelines
                .pipelines
                .insert(*compute_pipeline.id(), compute_pipeline);
        }

        for modified_shader in &self.modified_shaders {
            if let Some(compute_pipelines) = self
                .compute_pipelines
                .shader_to_pipeline
                .get(modified_shader)
            {
                for compute_pipeline in compute_pipelines {
                    let desc = self.compute_pipelines.pipelines[compute_pipeline].desc();
                    self.compute_pipelines.queued.push(desc.clone());
                }
            }
        }

        self.compute_pipelines.queued.retain(|desc| {
            let Some(shader) = self.inner.shaders.read().get(&desc.stage.shader).cloned() else {
                return true;
            };

            let desc = desc.clone();
            let id = self.compute_pipelines.ids[&desc];
            let inner = self.inner.clone();

            let pipelines = self
                .compute_pipelines
                .shader_to_pipeline
                .entry(desc.stage.shader.clone_weak())
                .or_insert_with(HashSet::new);
            pipelines.insert(id);

            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let shader_module = inner.get_shader_module(&desc.stage, &shader).unwrap(); //TODO:
                    let compute_pipeline = ComputePipeline::new(
                        inner.device.clone(),
                        &desc,
                        id,
                        &shader_module,
                        |layout_desc| inner.get_pipeline_layout(layout_desc),
                    )
                    .unwrap(); //TODO:
                    inner
                        .ready_compute_pipelines
                        .push(compute_pipeline)
                        .plain_unwrap();
                })
                .detach();

            false
        })
    }

    #[inline]
    pub fn queue_graphics_pipeline(&mut self, desc: &GraphicsPipelineDesc) -> GraphicsPipelineId {
        self.graphics_pipelines.queue(desc)
    }

    #[inline]
    pub fn queue_compute_pipeline(&mut self, desc: &ComputePipelineDesc) -> ComputePipelineId {
        self.compute_pipelines.queue(desc)
    }

    #[inline]
    pub fn get_graphics_pipeline(&self, id: &GraphicsPipelineId) -> Option<&GraphicsPipeline> {
        self.graphics_pipelines.get(id)
    }

    #[inline]
    pub fn get_compute_pipeline(&self, id: &ComputePipelineId) -> Option<&ComputePipeline> {
        self.compute_pipelines.get(id)
    }

    pub fn extract_shaders_system(
        mut cache: ResMut<Self>,
        shaders: Extract<Res<Assets<Shader>>>,
        mut events: Extract<EventReader<AssetEvent<Shader>>>,
    ) {
        for event in events.iter() {
            match event {
                AssetEvent::Created { handle } => {
                    if let Some(shader) = shaders.get(handle) {
                        cache.create_shader(handle, shader);
                    }
                }
                AssetEvent::Modified { handle } => {
                    if let Some(shader) = shaders.get(handle) {
                        cache.modify_shader(handle, shader);
                    }
                }
                AssetEvent::Removed { handle } => cache.remove_shader(handle),
            }
        }
    }

    pub fn process_pipelines_system(mut cache: ResMut<Self>) {
        for modified_shader in &cache.modified_shaders {
            cache
                .inner
                .spirv_modules
                .lock()
                .retain(|k, _| k != modified_shader);
        }

        cache.process_graphics_pipelines();
        cache.process_compute_pipelines();

        cache.modified_shaders.clear();
    }
}
