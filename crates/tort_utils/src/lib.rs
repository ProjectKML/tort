pub use bevy_utils::*;

pub mod smallvec {
    pub use smallvec::*;

    pub type SmallVec4<T> = SmallVec<[T; 4]>;
    pub type SmallVec8<T> = SmallVec<[T; 8]>;
    pub type SmallVec16<T> = SmallVec<[T; 16]>;
}
