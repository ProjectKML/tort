use std::{borrow::Cow, path::Path, sync::Arc};

use anyhow::Result;
use ash::vk;
use once_cell::sync::Lazy;
use regex::Regex;
use tort_asset::{AssetLoader, AssetPath, BoxedFuture, Handle, LoadContext, LoadedAsset};
use tort_reflect::{self as bevy_reflect, TypeUuid};

static INCLUDE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("^\\s*#include\\s+\"(.+)\"\\s*$").unwrap());

fn parse_includes(parent_path: &Path, source: &str) -> Vec<AssetPath<'static>> {
    let mut includes = Vec::new();

    for line in source.lines() {
        if let Some(captures) = INCLUDE_REGEX.captures(line) {
            let name = captures.get(1).unwrap().as_str();

            let mut path = parent_path.to_owned();
            path.push(name);

            includes.push(AssetPath::from(tort_utils::normalize_path(&path)));
        }
    }

    includes
}

#[derive(Debug)]
struct Inner {
    source: ShaderSource,
    path: AssetPath<'static>,
    includes: Vec<AssetPath<'static>>,
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
            includes: Vec::new(),
        }))
    }

    #[inline]
    pub fn from_glsl(
        path: impl Into<AssetPath<'static>>,
        source: impl Into<Cow<'static, str>>,
    ) -> Self {
        let source = source.into();
        let path = path.into();
        let includes = parse_includes(path.path().parent().unwrap(), &source);

        Self(Arc::new(Inner {
            source: ShaderSource::Glsl(source),
            path,
            includes,
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
    Glsl(Cow<'static, str>),
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
                "glsl" => Shader::from_glsl(path.to_owned(), String::from_utf8(Vec::from(bytes))?),
                "spv" => {
                    Shader::from_spirv(
                        path.to_owned(),
                        Vec::from(tort_utils::bytemuck::try_cast_slice(bytes)?),
                    )
                }
                _ => panic!("Unhandled extension: {ext}"),
            };

            let includes = shader.0.includes.clone();
            load_context.set_default_asset(LoadedAsset::new(shader).with_dependencies(includes));

            Ok(())
        })
    }

    #[inline]
    fn extensions(&self) -> &[&str] {
        &["spv", "glsl"]
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
