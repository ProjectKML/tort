use ash::vk;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Offset2D {
    pub x: i32,
    pub y: i32,
}

impl Offset2D {
    #[inline]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl From<Offset2D> for vk::Offset2D {
    #[inline]
    fn from(val: Offset2D) -> Self {
        vk::Offset2D { x: val.x, y: val.y }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

impl Extent2D {
    #[inline]
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl From<Extent2D> for vk::Extent2D {
    #[inline]
    fn from(val: Extent2D) -> Self {
        vk::Extent2D {
            width: val.width,
            height: val.height,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Rect2D {
    pub offset: Offset2D,
    pub extent: Extent2D,
}

impl Rect2D {
    #[inline]
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            offset: Offset2D { x, y },
            extent: Extent2D { width, height },
        }
    }
}

impl From<Rect2D> for vk::Rect2D {
    #[inline]
    fn from(val: Rect2D) -> Self {
        vk::Rect2D {
            offset: val.offset.into(),
            extent: val.extent.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Offset3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Offset3D {
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl From<Offset3D> for vk::Offset3D {
    #[inline]
    fn from(val: Offset3D) -> Self {
        vk::Offset3D {
            x: val.x,
            y: val.y,
            z: val.z,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl Extent3D {
    #[inline]
    pub fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }
}

impl From<Extent3D> for vk::Extent3D {
    #[inline]
    fn from(val: Extent3D) -> Self {
        vk::Extent3D {
            width: val.width,
            height: val.height,
            depth: val.depth,
        }
    }
}
