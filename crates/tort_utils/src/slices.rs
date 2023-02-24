use std::{mem, slice};

use bytemuck::Pod;

#[inline]
pub fn bytes_of<T: Pod>(t: &[T]) -> &[u8] {
    if mem::size_of::<T>() == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(t.as_ptr().cast(), t.len() * mem::size_of::<T>()) }
    }
}

#[inline]
pub fn bytes_of_mut<T: Pod>(t: &mut [T]) -> &mut [u8] {
    if mem::size_of::<T>() == 0 {
        &mut []
    } else {
        unsafe { slice::from_raw_parts_mut(t.as_mut_ptr().cast(), t.len() * mem::size_of::<T>()) }
    }
}

#[inline]
pub unsafe fn cast_unsafe<T, U>(t: &[T]) -> &[U] {
    debug_assert_eq!(mem::size_of::<T>(), mem::size_of::<U>());
    debug_assert_eq!(mem::align_of::<T>(), mem::align_of::<U>());

    slice::from_raw_parts(t.as_ptr().cast(), t.len())
}

#[inline]
pub unsafe fn cast_mut_unsafe<T, U>(t: &mut [T]) -> &mut [U] {
    debug_assert_eq!(mem::size_of::<T>(), mem::size_of::<U>());
    debug_assert_eq!(mem::align_of::<T>(), mem::align_of::<U>());

    slice::from_raw_parts_mut(t.as_mut_ptr().cast(), t.len())
}
