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

impl Allocator {
    pub fn new(create_info: &AllocatorCreateInfo) -> Self {
        use ash::vk::Handle;
        let mut ffi_create_info: ffi::VmaAllocatorCreateInfo = unsafe { std::mem::zeroed() };
        ffi_create_info.physicalDevice =
            create_info.physical_device.as_raw() as ffi::VkPhysicalDevice;
        ffi_create_info.device = create_info.device.as_raw() as ffi::VkDevice;
        let mut internal: ffi::VmaAllocator = unsafe { std::mem::zeroed() };
        let result = unsafe {
            ffi::vmaCreateAllocator(
                &ffi_create_info as *const ffi::VmaAllocatorCreateInfo,
                &mut internal,
            )
        };

        println!("result is {}", result);
        Allocator { internal }
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

        let result = unsafe {
            ffi::vmaCreateBuffer(
                self.internal,
                &ffi_buffer_create_info,
                &ffi_allocation_create_info,
                &mut ffi_buffer,
                &mut allocation.internal,
                &mut allocation.info,
            )
        };
        println!("result2 is {}", result);
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
