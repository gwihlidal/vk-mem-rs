extern crate ash;
#[macro_use]
extern crate bitflags;
extern crate failure;

pub mod error;
pub mod ffi;
pub use crate::error::{Error, ErrorKind, Result};
use ash::vk::Handle;
use std::mem;

#[derive(Clone)]
pub struct Allocator {
    pub(crate) internal: ffi::VmaAllocator,
    pub(crate) instance: ash::Instance,
    pub(crate) device: ash::Device,
}

#[derive(Debug, Clone)]
pub struct AllocatorPool {
    pub(crate) internal: ffi::VmaPool,
}

impl Default for AllocatorPool {
    fn default() -> Self {
        AllocatorPool {
            internal: unsafe { mem::zeroed() },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub(crate) internal: ffi::VmaAllocation,
    pub(crate) info: ffi::VmaAllocationInfo,
}

impl Allocation {
    #[inline(always)]
    pub fn get_memory_type(&self) -> u32 {
        self.info.memoryType
    }

    #[inline(always)]
    pub fn get_device_memory(&self) -> ash::vk::DeviceMemory {
        ash::vk::DeviceMemory::from_raw(self.info.deviceMemory as u64)
    }

    #[inline(always)]
    pub fn get_offset(&self) -> usize {
        self.info.offset as usize
    }

    #[inline(always)]
    pub fn get_size(&self) -> usize {
        self.info.size as usize
    }

    #[inline(always)]
    pub fn get_mapped_data(&self) -> *mut u8 {
        self.info.pMappedData as *mut u8
    }

    /*#[inline(always)]
    pub fn get_mapped_slice(&self) -> Option<&mut &[u8]> {
        if self.info.pMappedData.is_null() {
            None
        } else {
            Some(unsafe { &mut ::std::slice::from_raw_parts(self.info.pMappedData as *mut u8, self.get_size()) })
        }
    }*/

    #[inline(always)]
    pub fn get_user_data(&self) -> *mut ::std::os::raw::c_void {
        self.info.pUserData
    }
}

/// Description of an allocator to be created.
#[derive(Clone)]
pub struct AllocatorCreateInfo {
    pub physical_device: ash::vk::PhysicalDevice,
    pub device: ash::Device,
    pub instance: ash::Instance,
}

/// Converts a raw result into an ash result.
#[inline]
fn ffi_to_result(result: ffi::VkResult) -> ash::vk::Result {
    ash::vk::Result::from_raw(result)
}

/// Converts an `AllocationCreateInfo` struct into the raw representation.
fn allocation_create_info_to_ffi(info: &AllocationCreateInfo) -> ffi::VmaAllocationCreateInfo {
    let mut create_info: ffi::VmaAllocationCreateInfo = unsafe { mem::zeroed() };
    create_info.usage = match &info.usage {
        MemoryUsage::Unknown => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_UNKNOWN,
        MemoryUsage::GpuOnly => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_ONLY,
        MemoryUsage::CpuOnly => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_CPU_ONLY,
        MemoryUsage::CpuToGpu => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_CPU_TO_GPU,
        MemoryUsage::GpuToCpu => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_TO_CPU,
    };
    create_info.flags = info.flags.bits();
    create_info.requiredFlags = info.required_flags.as_raw();
    create_info.preferredFlags = info.preferred_flags.as_raw();
    create_info.memoryTypeBits = info.memory_type_bits;
    create_info.pool = match &info.pool {
        Some(pool) => pool.internal,
        None => unsafe { mem::zeroed() },
    };
    create_info.pUserData = info.user_data.unwrap_or(::std::ptr::null_mut());
    create_info
}

/// Converts an `AllocatorPoolCreateInfo` struct into the raw representation.
fn pool_create_info_to_ffi(info: &AllocatorPoolCreateInfo) -> ffi::VmaPoolCreateInfo {
    let mut create_info: ffi::VmaPoolCreateInfo = unsafe { mem::zeroed() };
    create_info.memoryTypeIndex = info.memory_type_index;
    create_info.flags = info.flags.bits();
    create_info.blockSize = info.block_size as ffi::VkDeviceSize;
    create_info.minBlockCount = info.min_block_count;
    create_info.maxBlockCount = info.max_block_count;
    create_info.frameInUseCount = info.frame_in_use_count;
    create_info
}

#[derive(Debug, Clone)]
pub enum MemoryUsage {
    Unknown,
    GpuOnly,
    CpuOnly,
    CpuToGpu,
    GpuToCpu,
}

bitflags! {
    pub struct AllocatorPoolCreateFlags: u32 {
        const NONE = 0x0000_0000;
        const IGNORE_BUFFER_IMAGE_GRANULARITY = 0x0000_0002;
        const LINEAR_ALGORITHM = 0x0000_0004;
        const BUDDY_ALGORITHM = 0x0000_0008;
        const ALGORITHM_MASK = 0x0000_0004 | 0x0000_0008;
    }
}

bitflags! {
    pub struct AllocationCreateFlags: u32 {
        const NONE = 0x0000_0000;
        const DEDICATED_MEMORY = 0x0000_0001;
        const NEVER_ALLOCATE = 0x0000_0002;
        const MAPPED = 0x0000_0004;
        const CAN_BECOME_LOST = 0x0000_0008;
        const CAN_MAKE_OTHER_LOST = 0x0000_0010;
        const USER_DATA_COPY_STRING = 0x0000_0020;
        const UPPER_ADDRESS = 0x0000_0040;
        const STRATEGY_BEST_FIT = 0x0001_0000;
        const STRATEGY_WORST_FIT = 0x0002_0000;
        const STRATEGY_FIRST_FIT = 0x0004_0000;
        const STRATEGY_MIN_MEMORY = 0x0001_0000;
        const STRATEGY_MIN_TIME = 0x0004_0000;
        const STRATEGY_MIN_FRAGMENTATION = 0x0002_0000;
        const STRATEGY_MASK = 0x0001_0000 | 0x0002_0000 | 0x0004_0000;
    }
}

#[derive(Debug, Clone)]
pub struct AllocationCreateInfo {
    pub usage: MemoryUsage,
    pub flags: AllocationCreateFlags,
    pub required_flags: ash::vk::MemoryPropertyFlags,
    pub preferred_flags: ash::vk::MemoryPropertyFlags,
    pub memory_type_bits: u32,
    pub pool: Option<AllocatorPool>,
    pub user_data: Option<*mut ::std::os::raw::c_void>
}

impl Default for AllocationCreateInfo {
    fn default() -> Self {
        AllocationCreateInfo {
            usage: MemoryUsage::Unknown,
            flags: AllocationCreateFlags::NONE,
            required_flags: ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
            preferred_flags: ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
            memory_type_bits: 0,
            pool: None,
            user_data: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AllocatorPoolCreateInfo {
    pub memory_type_index: u32,
    pub flags: AllocatorPoolCreateFlags,
    pub block_size: usize,
    pub min_block_count: usize,
    pub max_block_count: usize,

    /// Maximum number of additional frames that are in use at the same time as current frame.
    /// This value is used only when you make allocations with VMA_ALLOCATION_CREATE_CAN_BECOME_LOST_BIT flag.
    /// Such allocations cannot become lost if allocation.lastUseFrameIndex >= allocator.currentFrameIndex - frameInUseCount.
    /// 
    /// For example, if you double-buffer your command buffers, so resources used for rendering
    /// in previous frame may still be in use by the GPU at the moment you allocate resources
    /// needed for the current frame, set this value to 1.
    /// 
    /// If you want to allow any allocations other than used in the current frame to become lost,
    /// set this value to 0.
    pub frame_in_use_count: u32,
}

impl Default for AllocatorPoolCreateInfo {
    fn default() -> Self {
        AllocatorPoolCreateInfo {
            memory_type_index: 0,
            flags: AllocatorPoolCreateFlags::NONE,
            block_size: 0,
            min_block_count: 0,
            max_block_count: 0,
            frame_in_use_count: 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DefragmentationInfo {
    pub max_bytes_to_move: usize,
    pub max_allocations_to_move: u32,
}

impl Default for DefragmentationInfo {
    fn default() -> Self {
        DefragmentationInfo {
            max_bytes_to_move: ash::vk::WHOLE_SIZE as usize,
            max_allocations_to_move: std::u32::MAX,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DefragmentationStats {
    pub bytes_moved: usize,
    pub bytes_freed: usize,
    pub allocations_moved: u32,
    pub device_memory_blocks_freed: u32,
}

impl Allocator {
    pub fn new(create_info: &AllocatorCreateInfo) -> Result<Self> {
        use ash::version::{DeviceV1_0, DeviceV1_1, InstanceV1_0};
        let instance = create_info.instance.clone();
        let device = create_info.device.clone();
        let mut ffi_create_info: ffi::VmaAllocatorCreateInfo = unsafe { mem::zeroed() };
        ffi_create_info.physicalDevice =
            create_info.physical_device.as_raw() as ffi::VkPhysicalDevice;
        ffi_create_info.device = create_info.device.handle().as_raw() as ffi::VkDevice;
        let routed_functions = unsafe {
            ffi::VmaVulkanFunctions {
                vkGetPhysicalDeviceProperties: mem::transmute::<
                    _,
                    ffi::PFN_vkGetPhysicalDeviceProperties,
                >(Some(
                    instance.fp_v1_0().get_physical_device_properties,
                )),
                vkGetPhysicalDeviceMemoryProperties: mem::transmute::<
                    _,
                    ffi::PFN_vkGetPhysicalDeviceMemoryProperties,
                >(Some(
                    instance.fp_v1_0().get_physical_device_memory_properties,
                )),
                vkAllocateMemory: mem::transmute::<_, ffi::PFN_vkAllocateMemory>(Some(
                    device.fp_v1_0().allocate_memory,
                )),
                vkFreeMemory: mem::transmute::<_, ffi::PFN_vkFreeMemory>(Some(
                    device.fp_v1_0().free_memory,
                )),
                vkMapMemory: mem::transmute::<_, ffi::PFN_vkMapMemory>(Some(
                    device.fp_v1_0().map_memory,
                )),
                vkUnmapMemory: mem::transmute::<_, ffi::PFN_vkUnmapMemory>(Some(
                    device.fp_v1_0().unmap_memory,
                )),
                vkFlushMappedMemoryRanges: mem::transmute::<_, ffi::PFN_vkFlushMappedMemoryRanges>(
                    Some(device.fp_v1_0().flush_mapped_memory_ranges),
                ),
                vkInvalidateMappedMemoryRanges: mem::transmute::<
                    _,
                    ffi::PFN_vkInvalidateMappedMemoryRanges,
                >(Some(
                    device.fp_v1_0().invalidate_mapped_memory_ranges,
                )),
                vkBindBufferMemory: mem::transmute::<_, ffi::PFN_vkBindBufferMemory>(Some(
                    device.fp_v1_0().bind_buffer_memory,
                )),
                vkBindImageMemory: mem::transmute::<_, ffi::PFN_vkBindImageMemory>(Some(
                    device.fp_v1_0().bind_image_memory,
                )),
                vkGetBufferMemoryRequirements: mem::transmute::<
                    _,
                    ffi::PFN_vkGetBufferMemoryRequirements,
                >(Some(
                    device.fp_v1_0().get_buffer_memory_requirements,
                )),
                vkGetImageMemoryRequirements: mem::transmute::<
                    _,
                    ffi::PFN_vkGetImageMemoryRequirements,
                >(Some(
                    device.fp_v1_0().get_image_memory_requirements,
                )),
                vkCreateBuffer: mem::transmute::<_, ffi::PFN_vkCreateBuffer>(Some(
                    device.fp_v1_0().create_buffer,
                )),
                vkDestroyBuffer: mem::transmute::<_, ffi::PFN_vkDestroyBuffer>(Some(
                    device.fp_v1_0().destroy_buffer,
                )),
                vkCreateImage: mem::transmute::<_, ffi::PFN_vkCreateImage>(Some(
                    device.fp_v1_0().create_image,
                )),
                vkDestroyImage: mem::transmute::<_, ffi::PFN_vkDestroyImage>(Some(
                    device.fp_v1_0().destroy_image,
                )),
                vkGetBufferMemoryRequirements2KHR: mem::transmute::<
                    _,
                    ffi::PFN_vkGetBufferMemoryRequirements2KHR,
                >(Some(
                    device.fp_v1_1().get_buffer_memory_requirements2,
                )),
                vkGetImageMemoryRequirements2KHR: mem::transmute::<
                    _,
                    ffi::PFN_vkGetImageMemoryRequirements2KHR,
                >(Some(
                    device.fp_v1_1().get_image_memory_requirements2,
                )),
            }
        };
        ffi_create_info.pVulkanFunctions = &routed_functions;
        let mut internal: ffi::VmaAllocator = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaCreateAllocator(
                &ffi_create_info as *const ffi::VmaAllocatorCreateInfo,
                &mut internal,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(Allocator {
                internal,
                instance,
                device,
            }),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn get_physical_device_properties(&self) -> Result<ash::vk::PhysicalDeviceProperties> {
        let mut ffi_properties: *const ffi::VkPhysicalDeviceProperties = unsafe { mem::zeroed() };
        Ok(unsafe {
            ffi::vmaGetPhysicalDeviceProperties(self.internal, &mut ffi_properties);
            mem::transmute::<ffi::VkPhysicalDeviceProperties, ash::vk::PhysicalDeviceProperties>(
                *ffi_properties,
            )
        })
    }

    pub fn get_memory_properties(&self) -> Result<ash::vk::PhysicalDeviceMemoryProperties> {
        let mut ffi_properties: *const ffi::VkPhysicalDeviceMemoryProperties =
            unsafe { mem::zeroed() };
        Ok(unsafe {
            ffi::vmaGetMemoryProperties(self.internal, &mut ffi_properties);
            mem::transmute::<
                ffi::VkPhysicalDeviceMemoryProperties,
                ash::vk::PhysicalDeviceMemoryProperties,
            >(*ffi_properties)
        })
    }

    pub fn get_memory_type_properties(
        &self,
        memory_type_index: u32,
    ) -> Result<ash::vk::MemoryPropertyFlags> {
        let mut ffi_properties: ffi::VkMemoryPropertyFlags = unsafe { mem::zeroed() };
        Ok(unsafe {
            ffi::vmaGetMemoryTypeProperties(self.internal, memory_type_index, &mut ffi_properties);
            mem::transmute::<ffi::VkMemoryPropertyFlags, ash::vk::MemoryPropertyFlags>(
                ffi_properties,
            )
        })
    }

    pub fn set_current_frame_index(&self, frame_index: u32) -> Result<()> {
        unsafe {
            ffi::vmaSetCurrentFrameIndex(self.internal, frame_index);
        }
        Ok(())
    }

    pub fn calculate_stats(&self) -> Result<ffi::VmaStats> {
        let mut vma_stats: ffi::VmaStats = unsafe { mem::zeroed() };
        unsafe {
            ffi::vmaCalculateStats(self.internal, &mut vma_stats);
        }
        Ok(vma_stats)
    }

    pub fn build_stats_string(&self, detailed_map: bool) -> Result<String> {
        let mut stats_string: *mut ::std::os::raw::c_char = ::std::ptr::null_mut();
        unsafe {
            ffi::vmaBuildStatsString(
                self.internal,
                &mut stats_string,
                if detailed_map { 1 } else { 0 },
            );
        }
        Ok(if stats_string.is_null() {
            String::new()
        } else {
            let result = unsafe {
                std::ffi::CStr::from_ptr(stats_string)
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe {
                ffi::vmaFreeStatsString(self.internal, stats_string);
            }
            result
        })
    }

    pub fn find_memory_type_index(
        &self,
        memory_type_bits: u32,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<u32> {
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut memory_type_index: u32 = 0;
        let result = ffi_to_result(unsafe {
            ffi::vmaFindMemoryTypeIndex(
                self.internal,
                memory_type_bits,
                &create_info,
                &mut memory_type_index,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(memory_type_index),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn find_memory_type_index_for_buffer_info(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<u32> {
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let buffer_create_info = unsafe {
            mem::transmute::<ash::vk::BufferCreateInfo, ffi::VkBufferCreateInfo>(*buffer_info)
        };
        let mut memory_type_index: u32 = 0;
        let result = ffi_to_result(unsafe {
            ffi::vmaFindMemoryTypeIndexForBufferInfo(
                self.internal,
                &buffer_create_info,
                &allocation_create_info,
                &mut memory_type_index,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(memory_type_index),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn find_memory_type_index_for_image_info(
        &self,
        image_info: &ash::vk::ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<u32> {
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let image_create_info = unsafe {
            mem::transmute::<ash::vk::ImageCreateInfo, ffi::VkImageCreateInfo>(*image_info)
        };
        let mut memory_type_index: u32 = 0;
        let result = ffi_to_result(unsafe {
            ffi::vmaFindMemoryTypeIndexForImageInfo(
                self.internal,
                &image_create_info,
                &allocation_create_info,
                &mut memory_type_index,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(memory_type_index),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn create_pool(&mut self, pool_info: &AllocatorPoolCreateInfo) -> Result<AllocatorPool> {
        let mut ffi_pool: ffi::VmaPool = unsafe { mem::zeroed() };
        let create_info = pool_create_info_to_ffi(&pool_info);
        let result = ffi_to_result(unsafe {
            ffi::vmaCreatePool(self.internal, &create_info, &mut ffi_pool)
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(AllocatorPool { internal: ffi_pool }),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn destroy_pool(&mut self, pool: &AllocatorPool) -> Result<()> {
        unsafe {
            ffi::vmaDestroyPool(self.internal, pool.internal);
        }
        Ok(())
    }

    pub fn get_pool_stats(&self, pool: &AllocatorPool) -> Result<ffi::VmaPoolStats> {
        let mut pool_stats: ffi::VmaPoolStats = unsafe { mem::zeroed() };
        unsafe {
            ffi::vmaGetPoolStats(self.internal, pool.internal, &mut pool_stats);
        }
        Ok(pool_stats)
    }

    pub fn make_pool_allocations_lost(&mut self, pool: &mut AllocatorPool) -> Result<usize> {
        let mut lost_count: usize = 0;
        unsafe {
            ffi::vmaMakePoolAllocationsLost(self.internal, pool.internal, &mut lost_count);
        }
        Ok(lost_count)
    }

    pub fn check_pool_corruption(&self, pool: &AllocatorPool) -> Result<()> {
        let result =
            ffi_to_result(unsafe { ffi::vmaCheckPoolCorruption(self.internal, pool.internal) });
        match result {
            ash::vk::Result::SUCCESS => Ok(()),
            _ => Err(Error::vulkan(result)),
        }
    }

    #[doc = "General purpose memory allocation."]
    #[doc = ""]
    #[doc = "You should free the memory using `free_memory`."]
    #[doc = ""]
    #[doc = "It is recommended to use `allocate_memory_for_buffer`, `allocate_memory_for_image`,"]
    #[doc = "`create_buffer`, `create_image` instead whenever possible."]
    pub fn allocate_memory(
        &mut self,
        memory_requirements: &ash::vk::MemoryRequirements,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<Allocation> {
        let ffi_requirements = unsafe {
            mem::transmute::<ash::vk::MemoryRequirements, ffi::VkMemoryRequirements>(
                *memory_requirements,
            )
        };
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaAllocateMemory(
                self.internal,
                &ffi_requirements,
                &create_info,
                &mut allocation.internal,
                &mut allocation.info,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(allocation),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn allocate_memory_for_buffer(
        &mut self,
        buffer: ash::vk::Buffer,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<Allocation> {
        let ffi_buffer = buffer.as_raw() as ffi::VkBuffer;
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaAllocateMemoryForBuffer(
                self.internal,
                ffi_buffer,
                &create_info,
                &mut allocation.internal,
                &mut allocation.info,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(allocation),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn allocate_memory_for_image(
        &mut self,
        image: ash::vk::Image,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<Allocation> {
        let ffi_image = image.as_raw() as ffi::VkImage;
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaAllocateMemoryForImage(
                self.internal,
                ffi_image,
                &create_info,
                &mut allocation.internal,
                &mut allocation.info,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(allocation),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn free_memory(&mut self, allocation: &Allocation) -> Result<()> {
        unsafe {
            ffi::vmaFreeMemory(self.internal, allocation.internal);
        }
        Ok(())
    }

    pub fn resize_allocation(&mut self, allocation: &Allocation, new_size: usize) -> Result<()> {
        let result = ffi_to_result(unsafe {
            ffi::vmaResizeAllocation(
                self.internal,
                allocation.internal,
                new_size as ffi::VkDeviceSize,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(()),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn get_allocation_info(&mut self, allocation: &mut Allocation) -> Result<()> {
        unsafe {
            ffi::vmaGetAllocationInfo(self.internal, allocation.internal, &mut allocation.info)
        }
        Ok(())
    }

    pub fn touch_allocation(&mut self, allocation: &Allocation) -> Result<bool> {
        let result = unsafe { ffi::vmaTouchAllocation(self.internal, allocation.internal) };
        Ok(result == ash::vk::TRUE)
    }

    pub unsafe fn set_allocation_user_data(
        &mut self,
        allocation: &Allocation,
        user_data: *mut ::std::os::raw::c_void,
    ) -> Result<()> {
        ffi::vmaSetAllocationUserData(self.internal, allocation.internal, user_data);
        Ok(())
    }

    pub fn create_lost_allocation(&mut self) -> Result<Allocation> {
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        unsafe {
            ffi::vmaCreateLostAllocation(self.internal, &mut allocation.internal);
        }
        Ok(allocation)
    }

    pub fn map_memory(&mut self, allocation: &Allocation) -> Result<*mut u8> {
        let mut mapped_data: *mut ::std::os::raw::c_void = ::std::ptr::null_mut();
        let result = ffi_to_result(unsafe {
            ffi::vmaMapMemory(self.internal, allocation.internal, &mut mapped_data)
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(mapped_data as *mut u8),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn unmap_memory(&mut self, allocation: &Allocation) -> Result<()> {
        unsafe {
            ffi::vmaUnmapMemory(self.internal, allocation.internal);
        }
        Ok(())
    }

    pub fn flush_allocation(
        &mut self,
        allocation: &Allocation,
        offset: usize,
        size: usize,
    ) -> Result<()> {
        unsafe {
            ffi::vmaFlushAllocation(
                self.internal,
                allocation.internal,
                offset as ffi::VkDeviceSize,
                size as ffi::VkDeviceSize,
            );
        }
        Ok(())
    }

    pub fn invalidate_allocation(
        &mut self,
        allocation: &Allocation,
        offset: usize,
        size: usize,
    ) -> Result<()> {
        unsafe {
            ffi::vmaInvalidateAllocation(
                self.internal,
                allocation.internal,
                offset as ffi::VkDeviceSize,
                size as ffi::VkDeviceSize,
            );
        }
        Ok(())
    }

    pub fn check_corruption(&self, memory_types: ash::vk::MemoryPropertyFlags) -> Result<()> {
        let result =
            ffi_to_result(unsafe { ffi::vmaCheckCorruption(self.internal, memory_types.as_raw()) });
        match result {
            ash::vk::Result::SUCCESS => Ok(()),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn defragment(
        &mut self,
        allocations: &[Allocation],
        defrag_info: &DefragmentationInfo,
    ) -> Result<(DefragmentationStats, Vec<bool>)> {
        let mut ffi_allocations: Vec<ffi::VmaAllocation> = allocations
            .iter()
            .map(|allocation| allocation.internal)
            .collect();
        let mut ffi_change_list: Vec<ffi::VkBool32> = vec![0; ffi_allocations.len()];
        let ffi_info = ffi::VmaDefragmentationInfo {
            maxBytesToMove: defrag_info.max_bytes_to_move as ffi::VkDeviceSize,
            maxAllocationsToMove: defrag_info.max_allocations_to_move,
        };
        let mut ffi_stats: ffi::VmaDefragmentationStats = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaDefragment(
                self.internal,
                ffi_allocations.as_mut_ptr(),
                ffi_allocations.len(),
                ffi_change_list.as_mut_ptr(),
                &ffi_info,
                &mut ffi_stats,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => {
                let change_list: Vec<bool> = ffi_change_list
                    .iter()
                    .map(|change| *change == ash::vk::TRUE)
                    .collect();
                Ok((
                    DefragmentationStats {
                        bytes_moved: 0,
                        bytes_freed: 0,
                        allocations_moved: 0,
                        device_memory_blocks_freed: 0,
                    },
                    change_list,
                ))
            }
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn bind_buffer_memory(
        &mut self,
        buffer: ash::vk::Buffer,
        allocation: &Allocation,
    ) -> Result<()> {
        let result = ffi_to_result(unsafe {
            ffi::vmaBindBufferMemory(
                self.internal,
                allocation.internal,
                buffer.as_raw() as ffi::VkBuffer,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(()),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn bind_image_memory(
        &mut self,
        image: ash::vk::Image,
        allocation: &Allocation,
    ) -> Result<()> {
        let result = ffi_to_result(unsafe {
            ffi::vmaBindImageMemory(
                self.internal,
                allocation.internal,
                image.as_raw() as ffi::VkImage,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(()),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn create_buffer(
        &mut self,
        buffer_info: &ash::vk::BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<(ash::vk::Buffer, Allocation)> {
        let buffer_create_info = unsafe {
            mem::transmute::<ash::vk::BufferCreateInfo, ffi::VkBufferCreateInfo>(*buffer_info)
        };
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut buffer: ffi::VkBuffer = unsafe { mem::zeroed() };
        let mut allocation: Allocation = unsafe { mem::zeroed() };

        let result = ffi_to_result(unsafe {
            ffi::vmaCreateBuffer(
                self.internal,
                &buffer_create_info,
                &allocation_create_info,
                &mut buffer,
                &mut allocation.internal,
                &mut allocation.info,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok((ash::vk::Buffer::from_raw(buffer as u64), allocation)),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn destroy_buffer(
        &mut self,
        buffer: ash::vk::Buffer,
        allocation: &Allocation,
    ) -> Result<()> {
        unsafe {
            ffi::vmaDestroyBuffer(
                self.internal,
                buffer.as_raw() as ffi::VkBuffer,
                allocation.internal,
            );
        }
        Ok(())
    }

    pub fn create_image(
        &mut self,
        image_info: &ash::vk::ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<(ash::vk::Image, Allocation)> {
        let image_create_info = unsafe {
            mem::transmute::<ash::vk::ImageCreateInfo, ffi::VkImageCreateInfo>(*image_info)
        };
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut image: ffi::VkImage = unsafe { mem::zeroed() };
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaCreateImage(
                self.internal,
                &image_create_info,
                &allocation_create_info,
                &mut image,
                &mut allocation.internal,
                &mut allocation.info,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok((ash::vk::Image::from_raw(image as u64), allocation)),
            _ => Err(Error::vulkan(result)),
        }
    }

    pub fn destroy_image(&mut self, image: ash::vk::Image, allocation: &Allocation) -> Result<()> {
        unsafe {
            ffi::vmaDestroyImage(
                self.internal,
                image.as_raw() as ffi::VkImage,
                allocation.internal,
            );
        }
        Ok(())
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
