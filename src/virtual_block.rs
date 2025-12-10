use crate::ffi;
use crate::RawVirtualAllocationHandle;
use crate::RawVirtualBlockHandle;
use ash::prelude::VkResult;
use std::mem;

use crate::definitions::*;

/// Handle to a virtual block object that allows to use core allocation algorithm without allocating any real GPU memory.
///
/// For more info: <https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/virtual_allocator.html>
pub struct VirtualBlock {
    internal: RawVirtualBlockHandle,
}

/// Represents single memory allocation done inside VirtualBlock.
///
/// # Use with foreign code
///
/// The layout of this type is compatible with
/// [`VmaVirtualAllocation`](https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/struct_vma_virtual_allocation.html)
/// in C.
#[derive(Debug)]
pub struct VirtualAllocation(RawVirtualAllocationHandle);
unsafe impl Send for VirtualAllocation {}
unsafe impl Sync for VirtualAllocation {}

impl VirtualAllocation {
    /// Returns the raw handle of this virtual allocation
    pub fn get_raw(&self) -> RawVirtualAllocationHandle {
        self.0
    }

    /// Imports a virtual allocation from a raw handle
    ///
    /// # Safety
    ///
    /// The handle must be a valid virtual allocation
    pub unsafe fn from_raw(handle: RawVirtualAllocationHandle) -> Self {
        VirtualAllocation(handle)
    }
}

impl VirtualBlock {
    /// Creates new VirtualBlock object.
    pub fn new(create_info: VirtualBlockCreateInfo) -> VkResult<Self> {
        unsafe {
            let mut internal: ffi::VmaVirtualBlock = mem::zeroed();
            let raw_info = ffi::VmaVirtualBlockCreateInfo {
                size: create_info.size,
                flags: create_info.flags.bits(),
                pAllocationCallbacks: create_info
                    .allocation_callbacks
                    .map(|a| std::mem::transmute(a))
                    .unwrap_or(std::ptr::null()),
            };
            ffi::vmaCreateVirtualBlock(&raw_info, &mut internal).result()?;

            Ok(VirtualBlock { internal })
        }
    }

    /// Consumes the virtual block without dropping it and returns the underlying handle.
    ///
    /// Ownership is transferred to the caller.
    pub fn into_raw(self) -> RawVirtualBlockHandle {
        let handle = self.get_raw();
        mem::forget(self);
        handle
    }

    /// Gets the underlying raw handle
    pub fn get_raw(&self) -> RawVirtualBlockHandle {
        self.internal
    }

    /// Imports a virtual block from a raw handle.
    ///
    /// # Safety
    ///
    /// `handle` is a valid virtual block handle.
    ///
    /// Either the ownership of the virtual block needs to be transferred,
    /// or the caller must make sure that the returned value never gets dropped.
    pub unsafe fn from_raw(handle: RawVirtualBlockHandle) -> Self {
        Self { internal: handle }
    }

    /// Allocates new virtual allocation inside given VirtualBlock.
    ///
    /// Possible error values:
    ///
    /// - `ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY` - Allocation failed due to not enough free space in the virtual block.
    ///     (despite the function doesn't ever allocate actual GPU memory)
    pub unsafe fn allocate(
        &mut self,
        allocation_info: VirtualAllocationCreateInfo,
    ) -> VkResult<(VirtualAllocation, u64)> {
        let create_info: ffi::VmaVirtualAllocationCreateInfo = allocation_info.into();
        let mut allocation: ffi::VmaVirtualAllocation = std::mem::zeroed();
        let mut offset = 0;
        ffi::vmaVirtualAllocate(self.internal, &create_info, &mut allocation, &mut offset)
            .result()?;
        Ok((VirtualAllocation(allocation), offset))
    }

    /// Frees virtual allocation inside given VirtualBlock.
    ///
    /// It is correct to call this function with `allocation == VK_NULL_HANDLE` - it does nothing.
    pub unsafe fn free(&mut self, allocation: &mut VirtualAllocation) {
        ffi::vmaVirtualFree(self.internal, allocation.0);
    }

    /// Frees all virtual allocations inside given VirtualBlock.
    ///
    /// You must either call this function or free each virtual allocation individually with vmaVirtualFree()
    /// before destroying a virtual block. Otherwise, an assert is called.
    ///
    /// If you keep pointer to some additional metadata associated with your virtual allocation in its `user_data`,
    /// don't forget to free it as well.
    ///
    /// Any VirtualAllocations created previously in the VirtualBlock will no longer be valid!
    pub unsafe fn clear(&mut self) {
        ffi::vmaClearVirtualBlock(self.internal);
    }

    /// Returns information about a specific virtual allocation within a virtual block, like its size and user_data pointer.
    pub unsafe fn get_allocation_info(
        &self,
        allocation: &VirtualAllocation,
    ) -> VkResult<VirtualAllocationInfo> {
        let mut allocation_info: ffi::VmaVirtualAllocationInfo = mem::zeroed();
        ffi::vmaGetVirtualAllocationInfo(self.internal, allocation.0, &mut allocation_info);
        Ok(allocation_info.into())
    }

    /// Changes custom pointer associated with given virtual allocation.
    pub unsafe fn set_allocation_user_data(
        &self,
        allocation: &mut VirtualAllocation,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        ffi::vmaSetVirtualAllocationUserData(self.internal, allocation.0, user_data);
    }
}

/// Custom `Drop` implementation to clean up internal VirtualBlock instance
impl Drop for VirtualBlock {
    fn drop(&mut self) {
        unsafe {
            ffi::vmaDestroyVirtualBlock(self.internal);
            self.internal = std::ptr::null_mut();
        }
    }
}
