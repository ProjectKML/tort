use std::{borrow::Cow, sync::Arc};

use anyhow::Result;
use ash::vk;
use tort_asset::{AssetLoader, AssetPath, BoxedFuture, Handle, LoadContext, LoadedAsset};
use tort_reflect::{self as bevy_reflect, TypeUuid};

#[derive(Debug)]
struct Inner {
    source: ShaderSource,
    path: AssetPath<'static>,
}

#[derive(Clone, Debug, TypeUuid)]
#[uuid = "d09ec4a9-f995-429d-8924-d3cf6ddbc1bc"]
pub struct Shader(Arc<Inner>);

impl Shader {
    #[inline]
    pub fn from_spirv(
        path: impl Into<AssetPath<'static>>,
        source: impl Into<Cow<'static, [u32]>>,
    ) -> Self {
        Self(Arc::new(Inner {
            source: ShaderSource::SpirV(source.into()),
            path: path.into(),
        }))
    }

    #[inline]
    pub fn source(&self) -> &ShaderSource {
        &self.0.source
    }

    #[inline]
    pub fn path(&self) -> &AssetPath<'static> {
        &self.0.path
    }
}

#[derive(Clone, Debug)]
pub enum ShaderSource {
    SpirV(Cow<'static, [u32]>),
}

#[derive(Default)]
pub struct ShaderLoader;

impl AssetLoader for ShaderLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let path = tort_utils::normalize_path(load_context.path());
            let ext = path.extension().unwrap().to_str().unwrap();

            let shader = match ext {
                "spv" => {
                    Shader::from_spirv(
                        path.to_owned(),
                        Vec::from(tort_utils::bytemuck::try_cast_slice(bytes)?),
                    )
                }
                _ => panic!("Unhandled extension: {ext}"),
            };

            load_context.set_default_asset(LoadedAsset::new(shader));

            Ok(())
        })
    }

    #[inline]
    fn extensions(&self) -> &[&str] {
        &["spv"]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct SpecializationMapEntry {
    pub constant_id: u32,
    pub offset: u32,
    pub size: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpecializationInfo {
    pub map_entries: Vec<SpecializationMapEntry>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ShaderStageDesc {
    pub flags: vk::PipelineShaderStageCreateFlags,
    pub shader: Handle<Shader>,
    pub stage: vk::ShaderStageFlags,
    pub entry_point: Cow<'static, str>,
    pub defines: Vec<(Cow<'static, str>, Option<Cow<'static, str>>)>,
    pub specialization_info: Option<SpecializationInfo>,
}

impl From<&ShaderStageDesc> for ShaderStageDesc {
    #[inline]
    fn from(desc: &ShaderStageDesc) -> Self {
        desc.clone()
    }
}
