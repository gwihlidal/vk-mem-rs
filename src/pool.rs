use std::sync::Arc;

use crate::ffi;
use crate::Allocator;
use crate::PoolCreateInfo;
use ash::prelude::VkResult;

/// Represents custom memory pool handle.
pub struct AllocatorPool {
    allocator: Arc<Allocator>,
    pub(crate) raw: ffi::VmaPool,
}

impl Allocator {
    /// Allocates Vulkan device memory and creates `AllocatorPool` object.
    pub fn create_pool(self: &Arc<Self>, create_info: &PoolCreateInfo) -> VkResult<AllocatorPool> {
        unsafe {
            let mut ffi_pool: ffi::VmaPool = std::mem::zeroed();
            ffi::vmaCreatePool(self.internal, &create_info.inner, &mut ffi_pool).result()?;
            Ok(AllocatorPool {
                raw: ffi_pool,
                allocator: self.clone(),
            })
        }
    }
}

impl Drop for AllocatorPool {
    fn drop(&mut self) {
        unsafe {
            ffi::vmaDestroyPool(self.allocator.internal, self.raw);
        }
    }
}

impl AllocatorPool {
    /// Retrieves statistics of existing `AllocatorPool` object.
    pub fn get_statistics(&self) -> VkResult<ffi::VmaStatistics> {
        unsafe {
            let mut pool_stats: ffi::VmaStatistics = std::mem::zeroed();
            ffi::vmaGetPoolStatistics(self.allocator.internal, self.raw, &mut pool_stats);
            Ok(pool_stats)
        }
    }

    /// Retrieves statistics of existing `AllocatorPool` object.
    pub fn calculate_statistics(&self) -> VkResult<ffi::VmaDetailedStatistics> {
        unsafe {
            let mut pool_stats: ffi::VmaDetailedStatistics = std::mem::zeroed();
            ffi::vmaCalculatePoolStatistics(self.allocator.internal, self.raw, &mut pool_stats);
            Ok(pool_stats)
        }
    }

    /// Checks magic number in margins around all allocations in given memory pool in search for corruptions.
    ///
    /// Corruption detection is enabled only when `VMA_DEBUG_DETECT_CORRUPTION` macro is defined to nonzero,
    /// `VMA_DEBUG_MARGIN` is defined to nonzero and the pool is created in memory type that is
    /// `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` and `ash::vk::MemoryPropertyFlags::HOST_COHERENT`.
    ///
    /// Possible error values:
    ///
    /// - `ash::vk::Result::ERROR_FEATURE_NOT_PRESENT` - corruption detection is not enabled for specified pool.
    /// - `ash::vk::Result::ERROR_VALIDATION_FAILED_EXT` - corruption detection has been performed and found memory corruptions around one of the allocations.
    ///   `VMA_ASSERT` is also fired in that case.
    /// - Other value: Error returned by Vulkan, e.g. memory mapping failure.
    pub unsafe fn check_corruption(&self) -> VkResult<()> {
        ffi::vmaCheckPoolCorruption(self.allocator.internal, self.raw).result()
    }
}
