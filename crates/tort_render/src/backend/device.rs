use std::{ops::Deref, os::raw::c_char, sync::Arc};

use anyhow::Result;
use ash::{
    extensions::{
        ext::MeshShader,
        khr::{DynamicRendering, Swapchain, Synchronization2},
    },
    prelude::VkResult,
    vk,
};
use tort_ecs::{self as bevy_ecs, system::Resource};
use vk_mem_alloc::{Allocator, AllocatorCreateFlags, AllocatorCreateInfo};

use crate::backend::Instance;

pub struct DeviceProperties {
    pub properties: vk::PhysicalDeviceProperties,
    pub properties_11: vk::PhysicalDeviceVulkan11Properties<'static>,
    pub properties_12: vk::PhysicalDeviceVulkan12Properties<'static>,
    pub properties_13: vk::PhysicalDeviceVulkan13Properties<'static>,
    pub mesh_shader_properties: vk::PhysicalDeviceMeshShaderPropertiesEXT<'static>,
}

impl DeviceProperties {
    #[inline]
    unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut properties_11 = vk::PhysicalDeviceVulkan11Properties::default();
        let mut properties_12 = vk::PhysicalDeviceVulkan12Properties::default();
        let mut properties_13 = vk::PhysicalDeviceVulkan13Properties::default();
        let mut mesh_shader_properties = vk::PhysicalDeviceMeshShaderPropertiesEXT::default();

        let mut properties = vk::PhysicalDeviceProperties2::default()
            .push_next(&mut properties_11)
            .push_next(&mut properties_12)
            .push_next(&mut properties_13)
            .push_next(&mut mesh_shader_properties);

        instance
            .loader()
            .get_physical_device_properties2(physical_device, &mut properties);

        Self {
            properties: properties.properties,
            properties_11,
            properties_12,
            properties_13,
            mesh_shader_properties,
        }
    }
}

unsafe impl Send for DeviceProperties {}
unsafe impl Sync for DeviceProperties {}

pub struct DeviceMemoryProperties {
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
}

impl DeviceMemoryProperties {
    #[inline]
    unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut memory_properties = vk::PhysicalDeviceMemoryProperties2::default();

        instance
            .loader()
            .get_physical_device_memory_properties2(physical_device, &mut memory_properties);

        Self {
            memory_properties: memory_properties.memory_properties,
        }
    }
}

pub struct DeviceQueueFamilyProperties {
    pub queue_family_properties: Vec<vk::QueueFamilyProperties>,
}

impl DeviceQueueFamilyProperties {
    #[inline]
    unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let instance_loader = instance.loader();

        let mut queue_family_properties: Vec<_> = (0..instance_loader
            .get_physical_device_queue_family_properties2_len(physical_device))
            .map(|_| vk::QueueFamilyProperties2::default())
            .collect();
        instance_loader.get_physical_device_queue_family_properties2(
            physical_device,
            &mut queue_family_properties,
        );

        Self {
            queue_family_properties: queue_family_properties
                .iter()
                .map(|queue_family_properties| queue_family_properties.queue_family_properties)
                .collect(),
        }
    }
}

#[derive(Default)]
pub struct DeviceFeatures {
    pub features: vk::PhysicalDeviceFeatures,
    pub features_11: vk::PhysicalDeviceVulkan11Features<'static>,
    pub features_12: vk::PhysicalDeviceVulkan12Features<'static>,
    pub dynamic_rendering_features: vk::PhysicalDeviceDynamicRenderingFeatures<'static>,
    pub mesh_shader_features: vk::PhysicalDeviceMeshShaderFeaturesEXT<'static>,
    pub synchronization2_features: vk::PhysicalDeviceSynchronization2Features<'static>,
}

impl DeviceFeatures {
    #[inline]
    unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut features_11 = vk::PhysicalDeviceVulkan11Features::default();
        let mut features_12 = vk::PhysicalDeviceVulkan12Features::default();

        let mut dynamic_rendering_features = vk::PhysicalDeviceDynamicRenderingFeatures::default();
        let mut mesh_shader_features = vk::PhysicalDeviceMeshShaderFeaturesEXT::default();
        let mut synchronization2_features = vk::PhysicalDeviceSynchronization2Features::default();

        let mut features = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut features_11)
            .push_next(&mut features_12)
            .push_next(&mut dynamic_rendering_features)
            .push_next(&mut mesh_shader_features)
            .push_next(&mut synchronization2_features);

        instance
            .loader()
            .get_physical_device_features2(physical_device, &mut features);

        Self {
            features: features.features,
            features_11,
            features_12,
            dynamic_rendering_features,
            mesh_shader_features,
            synchronization2_features,
        }
    }
}

unsafe impl Send for DeviceFeatures {}
unsafe impl Sync for DeviceFeatures {}

pub struct DeviceExtensions {
    supported: Vec<vk::ExtensionProperties>,
    enabled: Vec<*const c_char>,

    ext_mesh_shader: bool,

    khr_dynamic_rendering: bool,
    khr_portability_subset: bool,
    khr_swapchain: bool,
    khr_synchronization2: bool,
}

impl DeviceExtensions {
    pub unsafe fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> VkResult<Self> {
        let supported = instance
            .loader()
            .enumerate_device_extension_properties(physical_device)?;

        Ok(Self {
            supported,
            enabled: Vec::new(),

            ext_mesh_shader: false,

            khr_dynamic_rendering: false,
            khr_portability_subset: false,
            khr_swapchain: false,
            khr_synchronization2: false,
        })
    }

    #[inline]
    unsafe fn try_push(&mut self, name: *const c_char) -> bool {
        if self
            .supported
            .iter()
            .any(|e| libc::strcmp(e.extension_name.as_ptr(), name) == 0)
        {
            self.enabled.push(name);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn try_push_ext_mesh_shader(&mut self) -> bool {
        if unsafe { self.try_push(MeshShader::name().as_ptr()) } {
            self.ext_mesh_shader = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_ext_mesh_shader(&mut self) {
        assert!(self.try_push_ext_mesh_shader());
    }

    #[inline]
    pub fn try_push_khr_dynamic_rendering(&mut self) -> bool {
        if unsafe { self.try_push(DynamicRendering::name().as_ptr()) } {
            self.khr_dynamic_rendering = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_khr_dynamic_rendering(&mut self) {
        assert!(self.try_push_khr_dynamic_rendering());
    }

    #[inline]
    pub fn try_push_khr_portability_subset(&mut self) -> bool {
        if unsafe { self.try_push(b"VK_KHR_portability_subset\0".as_ptr().cast()) } {
            self.khr_portability_subset = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_khr_portability_subset(&mut self) {
        assert!(self.try_push_khr_portability_subset());
    }

    #[inline]
    pub fn khr_portability_subset(&self) -> bool {
        self.khr_portability_subset
    }

    #[inline]
    pub fn try_push_khr_swapchain(&mut self) -> bool {
        if unsafe { self.try_push(Swapchain::name().as_ptr()) } {
            self.khr_swapchain = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_khr_swapchain(&mut self) {
        assert!(self.try_push_khr_swapchain());
    }

    #[inline]
    pub fn try_push_khr_synchronization2(&mut self) -> bool {
        if unsafe { self.try_push(Synchronization2::name().as_ptr()) } {
            self.khr_synchronization2 = true;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn push_khr_synchronization2(&mut self) {
        assert!(self.try_push_khr_synchronization2());
    }
}

unsafe impl Send for DeviceExtensions {}
unsafe impl Sync for DeviceExtensions {}

pub struct Queue {
    queue: vk::Queue,
    family_index: u32,
    index: u32,
}

impl Deref for Queue {
    type Target = vk::Queue;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}

impl Queue {
    unsafe fn new(device: &ash::Device, family_index: u32, index: u32) -> Self {
        let queue = device.get_device_queue(family_index, index);

        Self {
            queue,
            family_index,
            index,
        }
    }

    #[inline]
    pub fn family_index(&self) -> u32 {
        self.family_index
    }

    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }
}

struct Inner {
    physical_device: vk::PhysicalDevice,

    device: vk::Device,
    loader: ash::Device,
    dynamic_rendering_loader: DynamicRendering,
    mesh_shader_loader: MeshShader,
    swapchain_loader: Swapchain,
    synchronization2_loader: Synchronization2,
    allocator: Allocator,

    extensions: DeviceExtensions,

    properties: DeviceProperties,
    memory_properties: DeviceMemoryProperties,
    queue_family_properties: DeviceQueueFamilyProperties,

    supported_features: DeviceFeatures,
    enabled_features: DeviceFeatures,

    queues: [Queue; 3],

    instance: Instance,
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            vk_mem_alloc::destroy_allocator(self.allocator);
            self.loader.destroy_device(None);
        }
    }
}

#[derive(Clone, Resource)]
pub struct Device(Arc<Inner>);

unsafe fn find_direct_queue_family_index(
    _instance: &Instance,
    _physical_device: vk::PhysicalDevice,
    properties: &[vk::QueueFamilyProperties],
) -> Option<u32> {
    let mut queue_count: u32 = 0;
    let mut family_index: u32 = 0;

    let direct_flags: vk::QueueFlags =
        vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER;

    for (i, properties) in properties.iter().enumerate() {
        let i = i as u32;

        if (properties.queue_flags & direct_flags) == direct_flags
            && properties.queue_count > queue_count
        {
            queue_count = properties.queue_count;
            family_index = i;
        }
    }

    if queue_count > 0 {
        Some(family_index)
    } else {
        None
    }
}

unsafe fn find_queue_family_index(
    properties: &[vk::QueueFamilyProperties],
    desired_flags: vk::QueueFlags,
    undesired_flags: vk::QueueFlags,
) -> Option<u32> {
    let mut queue_count: u32 = 0;
    let mut family_index: u32 = 0;

    for (i, properties) in properties.iter().enumerate() {
        let i = i as u32;

        if (properties.queue_flags & desired_flags) == desired_flags
            && (properties.queue_flags & undesired_flags) == vk::QueueFlags::empty()
            && properties.queue_count > queue_count
        {
            queue_count = properties.queue_count;
            family_index = i;
        }
    }

    if queue_count > 0 {
        Some(family_index)
    } else {
        None
    }
}

unsafe fn find_queue_family_indices(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    properties: &[vk::QueueFamilyProperties],
) -> Option<(u32, u32, u32)> {
    let direct_index = find_direct_queue_family_index(instance, physical_device, properties)?;
    let compute_index = find_queue_family_index(
        properties,
        vk::QueueFlags::COMPUTE,
        vk::QueueFlags::GRAPHICS | vk::QueueFlags::TRANSFER,
    )
    .or_else(|| {
        find_queue_family_index(
            properties,
            vk::QueueFlags::COMPUTE,
            vk::QueueFlags::GRAPHICS,
        )
    })
    .or_else(|| {
        find_queue_family_index(
            properties,
            vk::QueueFlags::COMPUTE,
            vk::QueueFlags::TRANSFER,
        )
    })
    .unwrap_or(direct_index);

    let transfer_index = find_queue_family_index(
        properties,
        vk::QueueFlags::TRANSFER,
        vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE,
    )
    .or_else(|| {
        find_queue_family_index(
            properties,
            vk::QueueFlags::TRANSFER,
            vk::QueueFlags::GRAPHICS,
        )
    })
    .or_else(|| {
        find_queue_family_index(
            properties,
            vk::QueueFlags::TRANSFER,
            vk::QueueFlags::COMPUTE,
        )
    })
    .unwrap_or(direct_index);

    Some((direct_index, compute_index, transfer_index))
}

impl Device {
    pub unsafe fn new(
        instance: Instance,
        physical_device: vk::PhysicalDevice,
        callback: impl FnOnce(
            &DeviceProperties,
            &DeviceMemoryProperties,
            &DeviceQueueFamilyProperties,
            &mut DeviceExtensions,
            &DeviceFeatures,
            &mut DeviceFeatures,
        ) -> Result<()>,
    ) -> Result<Self> {
        let mut extensions = DeviceExtensions::new(&instance, physical_device)?;

        let properties = DeviceProperties::new(&instance, physical_device);
        let memory_properties = DeviceMemoryProperties::new(&instance, physical_device);
        let queue_family_properties = DeviceQueueFamilyProperties::new(&instance, physical_device);

        let supported_features = DeviceFeatures::new(&instance, physical_device);
        let mut enabled_features = DeviceFeatures::default();

        callback(
            &properties,
            &memory_properties,
            &queue_family_properties,
            &mut extensions,
            &supported_features,
            &mut enabled_features,
        )?;

        //Queue families
        let (direct_queue_family_index, compute_queue_family_index, transfer_queue_family_index) =
            find_queue_family_indices(
                &instance,
                physical_device,
                &queue_family_properties.queue_family_properties,
            )
            .ok_or_else(|| anyhow::anyhow!("Failed to find queue family indices"))?;

        let queue_priorities = [1.0];

        let mut device_queue_create_infos = vec![vk::DeviceQueueCreateInfo::default()
            .queue_family_index(direct_queue_family_index)
            .queue_priorities(&queue_priorities)];

        if compute_queue_family_index != direct_queue_family_index {
            device_queue_create_infos.push(
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(compute_queue_family_index)
                    .queue_priorities(&queue_priorities),
            );
        }

        if transfer_queue_family_index != direct_queue_family_index {
            device_queue_create_infos.push(
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(transfer_queue_family_index)
                    .queue_priorities(&queue_priorities),
            );
        }

        let mut features_11 = enabled_features.features_11;
        let mut features_12 = enabled_features.features_12;
        let mut dynamic_rendering_features = enabled_features.dynamic_rendering_features;
        let mut mesh_shader_features = enabled_features.mesh_shader_features;
        let mut synchronization2_features = enabled_features.synchronization2_features;

        let mut features = vk::PhysicalDeviceFeatures2::default()
            .features(enabled_features.features)
            .push_next(&mut features_11)
            .push_next(&mut features_12)
            .push_next(&mut dynamic_rendering_features)
            .push_next(&mut mesh_shader_features)
            .push_next(&mut synchronization2_features);

        //Create device
        let device_create_info = vk::DeviceCreateInfo::default()
            .push_next(&mut features)
            .queue_create_infos(&device_queue_create_infos)
            .enabled_extension_names(&extensions.enabled);

        let instance_loader = instance.loader();
        let loader = instance_loader.create_device(physical_device, &device_create_info, None)?;
        let dynamic_rendering_loader = DynamicRendering::new(instance_loader, &loader);
        let mesh_shader_loader = MeshShader::new(instance_loader, &loader);
        let swapchain_loader = Swapchain::new(instance_loader, &loader);
        let synchronization2_loader = Synchronization2::new(instance_loader, &loader);

        let allocator = vk_mem_alloc::create_allocator(
            instance_loader,
            physical_device,
            &loader,
            Some(&AllocatorCreateInfo {
                flags: AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS,
                ..Default::default()
            }),
        )?;

        let queues = [
            Queue::new(&loader, direct_queue_family_index, 0),
            Queue::new(&loader, compute_queue_family_index, 0),
            Queue::new(&loader, transfer_queue_family_index, 0),
        ];

        Ok(Self(Arc::new(Inner {
            physical_device,

            device: loader.handle(),
            loader,
            dynamic_rendering_loader,
            mesh_shader_loader,
            swapchain_loader,
            synchronization2_loader,
            allocator,

            extensions,

            properties,
            memory_properties,
            queue_family_properties,

            supported_features,
            enabled_features,

            queues,

            instance,
        })))
    }

    #[inline]
    pub fn physical_device(&self) -> &vk::PhysicalDevice {
        &self.0.physical_device
    }

    #[inline]
    pub fn loader(&self) -> &ash::Device {
        &self.0.loader
    }

    #[inline]
    pub fn dynamic_rendering_loader(&self) -> &DynamicRendering {
        &self.0.dynamic_rendering_loader
    }

    #[inline]
    pub fn mesh_shader_loader(&self) -> &MeshShader {
        &self.0.mesh_shader_loader
    }

    #[inline]
    pub fn swapchain_loader(&self) -> &Swapchain {
        &self.0.swapchain_loader
    }

    #[inline]
    pub fn synchronization2_loader(&self) -> &Synchronization2 {
        &self.0.synchronization2_loader
    }

    #[inline]
    pub fn allocator(&self) -> &Allocator {
        &self.0.allocator
    }

    #[inline]
    pub fn extensions(&self) -> &DeviceExtensions {
        &self.0.extensions
    }

    #[inline]
    pub fn properties(&self) -> &DeviceProperties {
        &self.0.properties
    }

    #[inline]
    pub fn memory_properties(&self) -> &DeviceMemoryProperties {
        &self.0.memory_properties
    }

    #[inline]
    pub fn queue_family_properties(&self) -> &DeviceQueueFamilyProperties {
        &self.0.queue_family_properties
    }

    #[inline]
    pub fn supported_features(&self) -> &DeviceFeatures {
        &self.0.supported_features
    }

    #[inline]
    pub fn enabled_features(&self) -> &DeviceFeatures {
        &self.0.enabled_features
    }

    #[inline]
    pub fn direct_queue(&self) -> &Queue {
        &self.0.queues[0]
    }

    #[inline]
    pub fn compute_queue(&self) -> &Queue {
        &self.0.queues[1]
    }

    #[inline]
    pub fn transfer_queue(&self) -> &Queue {
        &self.0.queues[2]
    }

    #[inline]
    pub fn queue(&self, index: u32) -> &Queue {
        &self.0.queues[index as usize]
    }

    #[inline]
    pub fn queues(&self) -> &[Queue] {
        &self.0.queues
    }

    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.0.instance
    }
}

impl Deref for Device {
    type Target = vk::Device;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.device
    }
}
