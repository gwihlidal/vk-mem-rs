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
    // TODO: vmaCalculateStats
    // TODO: vmaBuildStatsString
    // TODO: vmaFreeStatsString
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
    // TODO: vmaFreeMemory
    // TODO: vmaResizeAllocation
    // TODO: vmaGetAllocationInfo
    // TODO: vmaTouchAllocation
    // TODO: vmaSetAllocationUserData
    // TODO: vmaCreateLostAllocation
    // TODO: vmaMapMemory
    // TODO: vmaUnmapMemory
    // TODO: vmaFlushAllocation
    // TODO: vmaInvalidateAllocation

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
    // TODO: vmaBindBufferMemory
    // TODO: vmaBindImageMemory

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

    pub fn destroy_buffer(&mut self, buffer: ash::vk::Buffer, allocation: Allocation) {
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

    pub fn destroy_image(&mut self, image: ash::vk::Image, allocation: Allocation) {
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
