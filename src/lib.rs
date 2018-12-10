extern crate ash;

pub mod ffi;

#[derive(Debug)]
pub struct Allocator {
    pub(crate) internal: ffi::VmaAllocator,
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub(crate) internal: ffi::VmaAllocation,
    pub(crate) info: ffi::VmaAllocationInfo,
}

#[derive(Debug, Clone)]
pub struct AllocatorCreateInfo {
    pub physical_device: ash::vk::PhysicalDevice,
    pub device: ash::vk::Device,
}

#[inline]
fn ffi_to_result(result: ffi::VkResult) -> ash::vk::Result {
    ash::vk::Result::from_raw(result)
}

impl Allocator {
    pub fn new(create_info: &AllocatorCreateInfo) -> Self {
        use ash::vk::Handle;
        let mut ffi_create_info: ffi::VmaAllocatorCreateInfo = unsafe { std::mem::zeroed() };
        ffi_create_info.physicalDevice =
            create_info.physical_device.as_raw() as ffi::VkPhysicalDevice;
        ffi_create_info.device = create_info.device.as_raw() as ffi::VkDevice;
        let mut internal: ffi::VmaAllocator = unsafe { std::mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaCreateAllocator(
                &ffi_create_info as *const ffi::VmaAllocatorCreateInfo,
                &mut internal,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => {
                // Success
            }
            _ => {
                panic!(format!("new - error occurred! {}", result));
            }
        }

        Allocator { internal }
    }

    // TODO: vmaGetPhysicalDeviceProperties
    // TODO: vmaGetMemoryProperties
    // TODO: vmaGetMemoryTypeProperties
    // TODO: vmaSetCurrentFrameIndex

    pub fn calculate_stats(&self) -> ffi::VmaStats {
        let mut vma_stats: ffi::VmaStats = unsafe { std::mem::zeroed() };
        unsafe {
            ffi::vmaCalculateStats(
                self.internal,
                &mut vma_stats,
            );
        }
        vma_stats
    }

    pub fn build_stats_string(&self, detailed_map: bool) -> String {
        let mut stats_string: *mut ::std::os::raw::c_char = ::std::ptr::null_mut();
        unsafe {
            ffi::vmaBuildStatsString(self.internal, &mut stats_string, if detailed_map { 1 } else { 0 });
        }
        if stats_string.is_null() {
            String::new()
        } else {
            let result = unsafe { std::ffi::CStr::from_ptr(stats_string).to_string_lossy().into_owned() };
            unsafe { ffi::vmaFreeStatsString(self.internal, stats_string); }
            result
        }
    }

    // TODO: vmaFindMemoryTypeIndex
    // TODO: vmaFindMemoryTypeIndexForBufferInfo
    // TODO: vmaFindMemoryTypeIndexForImageInfo
    // TODO: vmaCreatePool
    // TODO: vmaDestroyPool
    // TODO: vmaGetPoolStats
    // TODO: vmaMakePoolAllocationsLost
    // TODO: vmaCheckPoolCorruption
    // TODO: vmaAllocateMemory
    // TODO: vmaAllocateMemoryForBuffer
    // TODO: vmaAllocateMemoryForImage

    pub fn free_memory(&mut self, allocation: &Allocation) {
        unsafe {
            ffi::vmaFreeMemory(
                self.internal,
                allocation.internal,
            );
        }
    }

    pub fn resize_allocation(&mut self, allocation: &Allocation, new_size: usize) {
        let result = ffi_to_result(unsafe {
            ffi::vmaResizeAllocation(
                self.internal,
                allocation.internal,
                new_size as ffi::VkDeviceSize,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => {
                // Success
            }
            _ => {
                panic!(format!("resize_allocation - error occurred! {}", result));
            }
        }
    }

    pub fn get_allocation_info(&mut self, allocation: &mut Allocation) {
        unsafe {
            ffi::vmaGetAllocationInfo(
                self.internal,
                allocation.internal,
                &mut allocation.info
            )
        }
    }

    pub fn touch_allocation(&mut self, allocation: &Allocation) -> bool {
        let result = unsafe {
            ffi::vmaTouchAllocation(
                self.internal,
                allocation.internal,
            )
        };
        if result == 1 {
            true
        } else {
            false
        }
    }

    pub fn set_allocation_user_data(&mut self, allocation: &Allocation, user_data: *mut ::std::os::raw::c_void) {
        unsafe {
            ffi::vmaSetAllocationUserData(
                self.internal,
                allocation.internal,
                user_data,
            );
        }
    }

    pub fn create_lost_allocation(&mut self) -> Allocation {
        let mut allocation: Allocation = unsafe { std::mem::zeroed() };
        unsafe {
            ffi::vmaCreateLostAllocation(
                self.internal,
                &mut allocation.internal,
            );
        }
        allocation
    }

    // TODO: vmaMapMemory

    pub fn unmap_memory(&mut self, allocation: &Allocation) {
        unsafe {
            ffi::vmaUnmapMemory(
                self.internal,
                allocation.internal,
            );
        }
    }

    pub fn flush_allocation(&mut self, allocation: &Allocation, offset: usize, size: usize) {
        unsafe {
            ffi::vmaFlushAllocation(
                self.internal,
                allocation.internal,
                offset as ffi::VkDeviceSize,
                size as ffi::VkDeviceSize,
            );
        }
    }

    pub fn invalidate_allocation(&mut self, allocation: &Allocation, offset: usize, size: usize) {
        unsafe {
            ffi::vmaInvalidateAllocation(
                self.internal,
                allocation.internal,
                offset as ffi::VkDeviceSize,
                size as ffi::VkDeviceSize,
            );
        }
    }

    pub fn check_corruption(&self, memory_types: ash::vk::MemoryPropertyFlags) {
        let result =
            ffi_to_result(unsafe { ffi::vmaCheckCorruption(self.internal, memory_types.as_raw()) });
        match result {
            ash::vk::Result::SUCCESS => {
                // Success
            }
            _ => {
                panic!(format!("check_corruption - error occurred! {}", result));
            }
        }
    }

    // TODO: vmaDefragment

    pub fn bind_buffer_memory(&mut self, buffer: ash::vk::Buffer, allocation: &Allocation) {
        use ash::vk::Handle;
        let result = ffi_to_result(unsafe {
            ffi::vmaBindBufferMemory(
                self.internal,
                allocation.internal,
                buffer.as_raw() as ffi::VkBuffer,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => {
                // Success
            }
            _ => {
                panic!(format!("bind_buffer_memory - error occurred! {}", result));
            }
        }
    }

    pub fn bind_image_memory(&mut self, image: ash::vk::Image, allocation: &Allocation) {
        use ash::vk::Handle;
        let result = ffi_to_result(unsafe {
            ffi::vmaBindImageMemory(
                self.internal,
                allocation.internal,
                image.as_raw() as ffi::VkImage,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => {
                // Success
            }
            _ => {
                panic!(format!("bind_image_memory - error occurred! {}", result));
            }
        }
    }

    pub fn create_buffer(
        &mut self,
        create_info: ash::vk::BufferCreateInfo,
    ) -> (ash::vk::Buffer, Allocation) {
        use ash::vk::Handle;
        let ffi_buffer_create_info: ffi::VkBufferCreateInfo = unsafe {
            std::mem::transmute::<ash::vk::BufferCreateInfo, ffi::VkBufferCreateInfo>(create_info)
        };
        let mut ffi_allocation_create_info: ffi::VmaAllocationCreateInfo =
            unsafe { std::mem::zeroed() };
        ffi_allocation_create_info.usage = ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_ONLY;

        let mut ffi_buffer: ffi::VkBuffer = unsafe { std::mem::zeroed() };
        let mut allocation: Allocation = unsafe { std::mem::zeroed() };

        let result = ffi_to_result(unsafe {
            ffi::vmaCreateBuffer(
                self.internal,
                &ffi_buffer_create_info,
                &ffi_allocation_create_info,
                &mut ffi_buffer,
                &mut allocation.internal,
                &mut allocation.info,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => {
                // Success
            }
            _ => {
                panic!(format!("create_buffer - error occurred! {}", result));
            }
        }
        (ash::vk::Buffer::from_raw(ffi_buffer as u64), allocation)
    }

    pub fn destroy_buffer(&mut self, buffer: ash::vk::Buffer, allocation: &Allocation) {
        use ash::vk::Handle;
        unsafe {
            ffi::vmaDestroyBuffer(
                self.internal,
                buffer.as_raw() as ffi::VkBuffer,
                allocation.internal,
            );
        }
    }

    // TODO: vmaCreateImage

    pub fn destroy_image(&mut self, image: ash::vk::Image, allocation: &Allocation) {
        use ash::vk::Handle;
        unsafe {
            ffi::vmaDestroyImage(
                self.internal,
                image.as_raw() as ffi::VkImage,
                allocation.internal,
            );
        }
    }
}

impl Drop for Allocator {
    fn drop(&mut self) {
        if !self.internal.is_null() {
            unsafe {
                ffi::vmaDestroyAllocator(self.internal);
            }
        }
    }
}
