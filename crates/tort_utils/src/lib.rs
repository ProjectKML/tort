use std::path::{Component, Path, PathBuf};

pub use bevy_utils::*;

pub mod bytemuck {
    pub use bytemuck::*;
}

pub use ordered_float::*;

pub mod slices;

pub mod smallvec {
    pub use smallvec::*;

    pub type SmallVec4<T> = SmallVec<[T; 4]>;
    pub type SmallVec8<T> = SmallVec<[T; 8]>;
    pub type SmallVec16<T> = SmallVec<[T; 16]>;
}

//From https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub trait PlainUnwrap<T> {
    fn plain_unwrap(self) -> T;
}

impl<T, E> PlainUnwrap<T> for Result<T, E> {
    #[inline]
    fn plain_unwrap(self) -> T {
        match self {
            Ok(t) => t,
            Err(_) => panic!("called `PlainUnwrap::plain_unwrap()` on an `Err` value"),
        }
    }
}
