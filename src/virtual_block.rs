use crate::ffi;
use ash::prelude::VkResult;
use std::mem;

use crate::definitions::*;

/// Handle to a virtual block object that allows to use core allocation algorithm without allocating any real GPU memory.
///
/// For more info: <https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/virtual_allocator.html>
pub struct VirtualBlock {
    internal: ffi::VmaVirtualBlock,
}

/// Represents single memory allocation done inside VirtualBlock.
#[derive(Debug)]
pub struct VirtualAllocation(ffi::VmaVirtualAllocation);
unsafe impl Send for VirtualAllocation {}
unsafe impl Sync for VirtualAllocation {}

impl VirtualBlock {
    /// Creates new VirtualBlock object.
    pub fn new(create_info: VirtualBlockCreateInfo) -> VkResult<Self> {
        unsafe {
            let mut internal: ffi::VmaVirtualBlock = mem::zeroed();
            ffi::vmaCreateVirtualBlock(&create_info.inner as *const _, &mut internal).result()?;

            Ok(VirtualBlock { internal })
        }
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
