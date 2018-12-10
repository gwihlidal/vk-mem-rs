extern crate ash;

pub mod ffi;

pub struct Allocator {
    pub(crate) internal: ffi::VmaAllocator,
}

impl Allocator {
    pub fn new() -> Self {
        let internal: ffi::VmaAllocator = unsafe { std::mem::zeroed() };
        Allocator {
            internal,
        }
    }
}