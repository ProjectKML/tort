use std::ffi::c_void;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

#[inline]
pub unsafe fn allocate(size: usize) -> *mut c_void {
    libmimalloc_sys::mi_malloc(size)
}

#[inline]
pub unsafe fn allocate_aligned(size: usize, alignment: usize) -> *mut c_void {
    libmimalloc_sys::mi_malloc_aligned(size, alignment)
}

#[inline]
pub unsafe fn reallocate(p: *mut c_void, new_size: usize) -> *mut c_void {
    libmimalloc_sys::mi_realloc(p, new_size)
}

#[inline]
pub unsafe fn reallocate_aligned(p: *mut c_void, new_size: usize, alignment: usize) -> *mut c_void {
    libmimalloc_sys::mi_realloc_aligned(p, new_size, alignment)
}

#[inline]
pub unsafe fn deallocate(p: *mut c_void) {
    libmimalloc_sys::mi_free(p)
}

#[inline]
pub unsafe fn deallocate_aligned(p: *mut c_void) {
    libmimalloc_sys::mi_free(p)
}
