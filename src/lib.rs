//! Easy to use, high performance memory manager for Vulkan.

mod definitions;
pub mod ffi;
pub use definitions::*;

use ash::prelude::VkResult;
use ash::vk;
use std::mem;

/// Main allocator object
pub struct Allocator {
    /// Pointer to internal VmaAllocator instance
    internal: ffi::VmaAllocator,
}

// Allocator is internally thread safe unless AllocatorCreateFlags::EXTERNALLY_SYNCHRONIZED is used (then you need to add synchronization!)
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

/// Represents custom memory pool handle.
///
/// Fill structure `AllocatorPoolCreateInfo` and call `Allocator::create_pool` to create it.
/// Call `Allocator::destroy_pool` to destroy it.
pub type AllocatorPool = ffi::VmaPool;

/// Represents single memory allocation.
///
/// It may be either dedicated block of `ash::vk::DeviceMemory` or a specific region of a
/// bigger block of this type plus unique offset.
///
/// Although the library provides convenience functions that create a Vulkan buffer or image,
/// allocate memory for it and bind them together, binding of the allocation to a buffer or an
/// image is out of scope of the allocation itself.
///
/// Allocation object can exist without buffer/image bound, binding can be done manually by
/// the user, and destruction of it can be done independently of destruction of the allocation.
///
/// The object also remembers its size and some other information. To retrieve this information,
/// use `Allocator::get_allocation_info`.
///
/// Some kinds allocations can be in lost state.
pub type Allocation = ffi::VmaAllocation;

/// Parameters of `Allocation` objects, that can be retrieved using `Allocator::get_allocation_info`.
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Pointer to internal VmaAllocationInfo instance
    internal: ffi::VmaAllocationInfo,
}

unsafe impl Send for AllocationInfo {}
unsafe impl Sync for AllocationInfo {}

impl AllocationInfo {
    #[inline(always)]
    // Gets the memory type index that this allocation was allocated from. (Never changes)
    pub fn get_memory_type(&self) -> u32 {
        self.internal.memoryType
    }

    /// Handle to Vulkan memory object.
    ///
    /// Same memory object can be shared by multiple allocations.
    ///
    /// It can change after call to `Allocator::defragment` if this allocation is passed
    /// to the function, or if allocation is lost.
    ///
    /// If the allocation is lost, it is equal to `ash::vk::DeviceMemory::null()`.
    #[inline(always)]
    pub fn get_device_memory(&self) -> ash::vk::DeviceMemory {
        self.internal.deviceMemory
    }

    /// Offset into device memory object to the beginning of this allocation, in bytes.
    /// (`self.get_device_memory()`, `self.get_offset()`) pair is unique to this allocation.
    ///
    /// It can change after call to `Allocator::defragment` if this allocation is passed
    /// to the function, or if allocation is lost.
    #[inline(always)]
    pub fn get_offset(&self) -> usize {
        self.internal.offset as usize
    }

    /// Size of this allocation, in bytes.
    ///
    /// It never changes, unless allocation is lost.
    #[inline(always)]
    pub fn get_size(&self) -> usize {
        self.internal.size as usize
    }

    /// Pointer to the beginning of this allocation as mapped data.
    ///
    /// If the allocation hasn't been mapped using `Allocator::map_memory` and hasn't been
    /// created with `AllocationCreateFlags::MAPPED` flag, this value is null.
    ///
    /// It can change after call to `Allocator::map_memory`, `Allocator::unmap_memory`.
    /// It can also change after call to `Allocator::defragment` if this allocation is
    /// passed to the function.
    #[inline(always)]
    pub fn get_mapped_data(&self) -> *mut u8 {
        self.internal.pMappedData as *mut u8
    }

    /*#[inline(always)]
    pub fn get_mapped_slice(&self) -> Option<&mut &[u8]> {
        if self.internal.pMappedData.is_null() {
            None
        } else {
            Some(unsafe { &mut ::std::slice::from_raw_parts(self.internal.pMappedData as *mut u8, self.get_size()) })
        }
    }*/

    /// Custom general-purpose pointer that was passed as `AllocationCreateInfo::user_data` or set using `Allocator::set_allocation_user_data`.
    ///
    /// It can change after a call to `Allocator::set_allocation_user_data` for this allocation.
    #[inline(always)]
    pub fn get_user_data(&self) -> *mut ::std::os::raw::c_void {
        self.internal.pUserData
    }
}

/// Construct `AllocatorCreateFlags` with default values
impl Default for AllocatorCreateFlags {
    fn default() -> Self {
        AllocatorCreateFlags::NONE
    }
}

/// Converts a raw result into an ash result.
#[inline]
fn ffi_to_result(result: vk::Result) -> VkResult<()> {
    match result {
        vk::Result::SUCCESS => Ok(()),
        _ => Err(result),
    }
}

impl Allocator {
    /// Constructor a new `Allocator` using the provided options.
    pub fn new(mut create_info: AllocatorCreateInfo) -> VkResult<Self> {
        unsafe extern "system" fn get_instance_proc_addr_stub(
            _instance: ash::vk::Instance,
            _p_name: *const ::std::os::raw::c_char,
        ) -> ash::vk::PFN_vkVoidFunction {
            panic!("VMA_DYNAMIC_VULKAN_FUNCTIONS is unsupported")
        }

        unsafe extern "system" fn get_get_device_proc_stub(
            _device: ash::vk::Device,
            _p_name: *const ::std::os::raw::c_char,
        ) -> ash::vk::PFN_vkVoidFunction {
            panic!("VMA_DYNAMIC_VULKAN_FUNCTIONS is unsupported")
        }

        #[cfg(feature = "loaded")]
        {
            let routed_functions = ffi::VmaVulkanFunctions {
                vkGetInstanceProcAddr: get_instance_proc_addr_stub,
                vkGetDeviceProcAddr: get_get_device_proc_stub,
                vkGetPhysicalDeviceProperties: create_info
                    .instance
                    .fp_v1_0()
                    .get_physical_device_properties,
                vkGetPhysicalDeviceMemoryProperties: create_info
                    .instance
                    .fp_v1_0()
                    .get_physical_device_memory_properties,
                vkAllocateMemory: create_info.device.fp_v1_0().allocate_memory,
                vkFreeMemory: create_info.device.fp_v1_0().free_memory,
                vkMapMemory: create_info.device.fp_v1_0().map_memory,
                vkUnmapMemory: create_info.device.fp_v1_0().unmap_memory,
                vkFlushMappedMemoryRanges: create_info.device.fp_v1_0().flush_mapped_memory_ranges,
                vkInvalidateMappedMemoryRanges: create_info
                    .device
                    .fp_v1_0()
                    .invalidate_mapped_memory_ranges,
                vkBindBufferMemory: create_info.device.fp_v1_0().bind_buffer_memory,
                vkBindImageMemory: create_info.device.fp_v1_0().bind_image_memory,
                vkGetBufferMemoryRequirements: create_info
                    .device
                    .fp_v1_0()
                    .get_buffer_memory_requirements,
                vkGetImageMemoryRequirements: create_info
                    .device
                    .fp_v1_0()
                    .get_image_memory_requirements,
                vkCreateBuffer: create_info.device.fp_v1_0().create_buffer,
                vkDestroyBuffer: create_info.device.fp_v1_0().destroy_buffer,
                vkCreateImage: create_info.device.fp_v1_0().create_image,
                vkDestroyImage: create_info.device.fp_v1_0().destroy_image,
                vkCmdCopyBuffer: create_info.device.fp_v1_0().cmd_copy_buffer,
                vkGetBufferMemoryRequirements2KHR: create_info
                    .device
                    .fp_v1_1()
                    .get_buffer_memory_requirements2,
                vkGetImageMemoryRequirements2KHR: create_info
                    .device
                    .fp_v1_1()
                    .get_image_memory_requirements2,
                vkBindBufferMemory2KHR: create_info.device.fp_v1_1().bind_buffer_memory2,
                vkBindImageMemory2KHR: create_info.device.fp_v1_1().bind_image_memory2,
                vkGetPhysicalDeviceMemoryProperties2KHR: create_info
                    .instance
                    .fp_v1_1()
                    .get_physical_device_memory_properties2,
                vkGetDeviceBufferMemoryRequirements: create_info
                    .device
                    .fp_v1_3()
                    .get_device_buffer_memory_requirements,
                vkGetDeviceImageMemoryRequirements: create_info
                    .device
                    .fp_v1_3()
                    .get_device_image_memory_requirements,
            };
            create_info.inner.pVulkanFunctions = &routed_functions;
        }
        unsafe {
            let mut internal: ffi::VmaAllocator = mem::zeroed();
            ffi_to_result(ffi::vmaCreateAllocator(
                &create_info.inner as *const _,
                &mut internal,
            ))?;

            Ok(Allocator { internal })
        }
    }

    /// The allocator fetches `ash::vk::PhysicalDeviceProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub unsafe fn get_physical_device_properties(&self) -> VkResult<vk::PhysicalDeviceProperties> {
        let mut properties = vk::PhysicalDeviceProperties::default();
        ffi::vmaGetPhysicalDeviceProperties(
            self.internal,
            &mut properties as *mut _ as *mut *const _,
        );

        Ok(properties)
    }

    /// The allocator fetches `ash::vk::PhysicalDeviceMemoryProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub unsafe fn get_memory_properties(&self) -> VkResult<vk::PhysicalDeviceMemoryProperties> {
        let mut properties = vk::PhysicalDeviceMemoryProperties::default();
        ffi::vmaGetMemoryProperties(self.internal, &mut properties as *mut _ as *mut *const _);

        Ok(properties)
    }

    /// Given a memory type index, returns `ash::vk::MemoryPropertyFlags` of this memory type.
    ///
    /// This is just a convenience function; the same information can be obtained using
    /// `Allocator::get_memory_properties`.
    pub unsafe fn get_memory_type_properties(
        &self,
        memory_type_index: u32,
    ) -> VkResult<vk::MemoryPropertyFlags> {
        let mut flags = vk::MemoryPropertyFlags::empty();
        ffi::vmaGetMemoryTypeProperties(self.internal, memory_type_index, &mut flags);

        Ok(flags)
    }

    /// Sets index of the current frame.
    ///
    /// This function must be used if you make allocations with `AllocationCreateFlags::CAN_BECOME_LOST` and
    /// `AllocationCreateFlags::CAN_MAKE_OTHER_LOST` flags to inform the allocator when a new frame begins.
    /// Allocations queried using `Allocator::get_allocation_info` cannot become lost
    /// in the current frame.
    pub unsafe fn set_current_frame_index(&self, frame_index: u32) {
        ffi::vmaSetCurrentFrameIndex(self.internal, frame_index);
    }

    /// Retrieves statistics from current state of the `Allocator`.
    pub unsafe fn calculate_stats(&self) -> VkResult<ffi::VmaTotalStatistics> {
        let mut vma_stats: ffi::VmaTotalStatistics = mem::zeroed();
        ffi::vmaCalculateStatistics(self.internal, &mut vma_stats);
        Ok(vma_stats)
    }

    /// Builds and returns statistics in `JSON` format.
    pub unsafe fn build_stats_string(&self, detailed_map: bool) -> VkResult<String> {
        let mut stats_string: *mut ::std::os::raw::c_char = ::std::ptr::null_mut();
        ffi::vmaBuildStatsString(
            self.internal,
            &mut stats_string,
            if detailed_map { 1 } else { 0 },
        );

        Ok(if stats_string.is_null() {
            String::new()
        } else {
            let result = std::ffi::CStr::from_ptr(stats_string)
                .to_string_lossy()
                .into_owned();
            ffi::vmaFreeStatsString(self.internal, stats_string);
            result
        })
    }

    /// Helps to find memory type index, given memory type bits and allocation info.
    ///
    /// This algorithm tries to find a memory type that:
    ///
    /// - Is allowed by memory type bits.
    /// - Contains all the flags from `allocation_info.required_flags`.
    /// - Matches intended usage.
    /// - Has as many flags from `allocation_info.preferred_flags` as possible.
    ///
    /// Returns ash::vk::Result::ERROR_FEATURE_NOT_PRESENT if not found. Receiving such a result
    /// from this function or any other allocating function probably means that your
    /// device doesn't support any memory type with requested features for the specific
    /// type of resource you want to use it for. Please check parameters of your
    /// resource, like image layout (OPTIMAL versus LINEAR) or mip level count.
    pub unsafe fn find_memory_type_index(
        &self,
        memory_type_bits: u32,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let mut memory_type_index: u32 = 0;
        ffi_to_result(ffi::vmaFindMemoryTypeIndex(
            self.internal,
            memory_type_bits,
            &allocation_info.inner,
            &mut memory_type_index,
        ))?;

        Ok(memory_type_index)
    }

    /// Helps to find memory type index, given buffer info and allocation info.
    ///
    /// It can be useful e.g. to determine value to be used as `AllocatorPoolCreateInfo::memory_type_index`.
    /// It internally creates a temporary, dummy buffer that never has memory bound.
    /// It is just a convenience function, equivalent to calling:
    ///
    /// - `ash::vk::Device::create_buffer`
    /// - `ash::vk::Device::get_buffer_memory_requirements`
    /// - `Allocator::find_memory_type_index`
    /// - `ash::vk::Device::destroy_buffer`
    pub unsafe fn find_memory_type_index_for_buffer_info(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let mut memory_type_index: u32 = 0;
        ffi_to_result(ffi::vmaFindMemoryTypeIndexForBufferInfo(
            self.internal,
            buffer_info,
            &allocation_info.inner,
            &mut memory_type_index,
        ))?;

        Ok(memory_type_index)
    }

    /// Helps to find memory type index, given image info and allocation info.
    ///
    /// It can be useful e.g. to determine value to be used as `AllocatorPoolCreateInfo::memory_type_index`.
    /// It internally creates a temporary, dummy image that never has memory bound.
    /// It is just a convenience function, equivalent to calling:
    ///
    /// - `ash::vk::Device::create_image`
    /// - `ash::vk::Device::get_image_memory_requirements`
    /// - `Allocator::find_memory_type_index`
    /// - `ash::vk::Device::destroy_image`
    pub unsafe fn find_memory_type_index_for_image_info(
        &self,
        image_info: ash::vk::ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let mut memory_type_index: u32 = 0;
        ffi_to_result(ffi::vmaFindMemoryTypeIndexForImageInfo(
            self.internal,
            &image_info,
            &allocation_info.inner,
            &mut memory_type_index,
        ))?;

        Ok(memory_type_index)
    }

    /// Allocates Vulkan device memory and creates `AllocatorPool` object.
    pub unsafe fn create_pool(&self, create_info: &PoolCreateInfo) -> VkResult<AllocatorPool> {
        let mut ffi_pool: ffi::VmaPool = mem::zeroed();
        ffi_to_result(ffi::vmaCreatePool(
            self.internal,
            &create_info.inner,
            &mut ffi_pool,
        ))?;
        Ok(ffi_pool)
    }

    /// Destroys `AllocatorPool` object and frees Vulkan device memory.
    pub unsafe fn destroy_pool(&self, pool: AllocatorPool) {
        ffi::vmaDestroyPool(self.internal, pool);
    }

    /// Retrieves statistics of existing `AllocatorPool` object.
    pub unsafe fn get_pool_stats(&self, pool: AllocatorPool) -> VkResult<ffi::VmaStatistics> {
        let mut pool_stats: ffi::VmaStatistics = mem::zeroed();
        ffi::vmaGetPoolStatistics(self.internal, pool, &mut pool_stats);
        Ok(pool_stats)
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
    pub unsafe fn check_pool_corruption(&self, pool: AllocatorPool) -> VkResult<()> {
        ffi_to_result(ffi::vmaCheckPoolCorruption(self.internal, pool))
    }

    /// General purpose memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    ///
    /// It is recommended to use `Allocator::allocate_memory_for_buffer`, `Allocator::allocate_memory_for_image`,
    /// `Allocator::create_buffer`, `Allocator::create_image` instead whenever possible.
    pub unsafe fn allocate_memory(
        &self,
        memory_requirements: &ash::vk::MemoryRequirements,
        create_info: &AllocationCreateInfo,
    ) -> VkResult<(Allocation, AllocationInfo)> {
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaAllocateMemory(
            self.internal,
            memory_requirements,
            &create_info.inner,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((allocation, allocation_info))
    }

    /// General purpose memory allocation for multiple allocation objects at once.
    ///
    /// You should free the memory using `Allocator::free_memory` or `Allocator::free_memory_pages`.
    ///
    /// Word "pages" is just a suggestion to use this function to allocate pieces of memory needed for sparse binding.
    /// It is just a general purpose allocation function able to make multiple allocations at once.
    /// It may be internally optimized to be more efficient than calling `Allocator::allocate_memory` `allocations.len()` times.
    ///
    /// All allocations are made using same parameters. All of them are created out of the same memory pool and type.
    pub unsafe fn allocate_memory_pages(
        &self,
        memory_requirements: &ash::vk::MemoryRequirements,
        create_info: &AllocationCreateInfo,
        allocation_count: usize,
    ) -> VkResult<Vec<(Allocation, AllocationInfo)>> {
        let mut allocations: Vec<ffi::VmaAllocation> = vec![mem::zeroed(); allocation_count];
        let mut allocation_info: Vec<ffi::VmaAllocationInfo> =
            vec![mem::zeroed(); allocation_count];
        ffi_to_result(ffi::vmaAllocateMemoryPages(
            self.internal,
            memory_requirements,
            &create_info.inner,
            allocation_count,
            allocations.as_mut_ptr(),
            allocation_info.as_mut_ptr(),
        ))?;

        let it = allocations.iter().zip(allocation_info.iter());
        let allocations: Vec<(Allocation, AllocationInfo)> = it
            .map(|(alloc, info)| (*alloc, AllocationInfo { internal: *info }))
            .collect();

        Ok(allocations)
    }

    /// Buffer specialized memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    pub unsafe fn allocate_memory_for_buffer(
        &self,
        buffer: ash::vk::Buffer,
        create_info: &AllocationCreateInfo,
    ) -> VkResult<(Allocation, AllocationInfo)> {
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaAllocateMemoryForBuffer(
            self.internal,
            buffer,
            &create_info.inner,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((allocation, allocation_info))
    }

    /// Image specialized memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    pub unsafe fn allocate_memory_for_image(
        &self,
        image: ash::vk::Image,
        create_info: &AllocationCreateInfo,
    ) -> VkResult<(Allocation, AllocationInfo)> {
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaAllocateMemoryForImage(
            self.internal,
            image,
            &create_info.inner,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((allocation, allocation_info))
    }

    /// Frees memory previously allocated using `Allocator::allocate_memory`,
    /// `Allocator::allocate_memory_for_buffer`, or `Allocator::allocate_memory_for_image`.
    pub unsafe fn free_memory(&self, allocation: Allocation) {
        ffi::vmaFreeMemory(self.internal, allocation);
    }

    /// Frees memory and destroys multiple allocations.
    ///
    /// Word "pages" is just a suggestion to use this function to free pieces of memory used for sparse binding.
    /// It is just a general purpose function to free memory and destroy allocations made using e.g. `Allocator::allocate_memory',
    /// 'Allocator::allocate_memory_pages` and other functions.
    ///
    /// It may be internally optimized to be more efficient than calling 'Allocator::free_memory` `allocations.len()` times.
    ///
    /// Allocations in 'allocations' slice can come from any memory pools and types.
    pub unsafe fn free_memory_pages(&self, allocations: &[Allocation]) {
        ffi::vmaFreeMemoryPages(
            self.internal,
            allocations.len(),
            allocations.as_ptr() as *mut _,
        );
    }

    /// Returns current information about specified allocation and atomically marks it as used in current frame.
    ///
    /// Current parameters of given allocation are returned in the result object, available through accessors.
    ///
    /// This function also atomically "touches" allocation - marks it as used in current frame,
    /// just like `Allocator::touch_allocation`.
    ///
    /// If the allocation is in lost state, `allocation.get_device_memory` returns `ash::vk::DeviceMemory::null()`.
    ///
    /// Although this function uses atomics and doesn't lock any mutex, so it should be quite efficient,
    /// you can avoid calling it too often.
    ///
    /// If you just want to check if allocation is not lost, `Allocator::touch_allocation` will work faster.
    pub unsafe fn get_allocation_info(&self, allocation: Allocation) -> VkResult<AllocationInfo> {
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi::vmaGetAllocationInfo(self.internal, allocation, &mut allocation_info.internal);
        Ok(allocation_info)
    }

    /// Sets user data in given allocation to new value.
    ///
    /// If the allocation was created with `AllocationCreateFlags::USER_DATA_COPY_STRING`,
    /// `user_data` must be either null, or pointer to a null-terminated string. The function
    /// makes local copy of the string and sets it as allocation's user data. String
    /// passed as user data doesn't need to be valid for whole lifetime of the allocation -
    /// you can free it after this call. String previously pointed by allocation's
    /// user data is freed from memory.
    ///
    /// If the flag was not used, the value of pointer `user_data` is just copied to
    /// allocation's user data. It is opaque, so you can use it however you want - e.g.
    /// as a pointer, ordinal number or some handle to you own data.
    pub unsafe fn set_allocation_user_data(
        &self,
        allocation: Allocation,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        ffi::vmaSetAllocationUserData(self.internal, allocation, user_data);
    }

    /// Maps memory represented by given allocation and returns pointer to it.
    ///
    /// Maps memory represented by given allocation to make it accessible to CPU code.
    /// When succeeded, result is a pointer to first byte of this memory.
    ///
    /// If the allocation is part of bigger `ash::vk::DeviceMemory` block, the pointer is
    /// correctly offseted to the beginning of region assigned to this particular
    /// allocation.
    ///
    /// Mapping is internally reference-counted and synchronized, so despite raw Vulkan
    /// function `ash::vk::Device::MapMemory` cannot be used to map same block of
    /// `ash::vk::DeviceMemory` multiple times simultaneously, it is safe to call this
    /// function on allocations assigned to the same memory block. Actual Vulkan memory
    /// will be mapped on first mapping and unmapped on last unmapping.
    ///
    /// If the function succeeded, you must call `Allocator::unmap_memory` to unmap the
    /// allocation when mapping is no longer needed or before freeing the allocation, at
    /// the latest.
    ///
    /// It also safe to call this function multiple times on the same allocation. You
    /// must call `Allocator::unmap_memory` same number of times as you called
    /// `Allocator::map_memory`.
    ///
    /// It is also safe to call this function on allocation created with
    /// `AllocationCreateFlags::MAPPED` flag. Its memory stays mapped all the time.
    /// You must still call `Allocator::unmap_memory` same number of times as you called
    /// `Allocator::map_memory`. You must not call `Allocator::unmap_memory` additional
    /// time to free the "0-th" mapping made automatically due to `AllocationCreateFlags::MAPPED` flag.
    ///
    /// This function fails when used on allocation made in memory type that is not
    /// `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`.
    ///
    /// This function always fails when called for allocation that was created with
    /// `AllocationCreateFlags::CAN_BECOME_LOST` flag. Such allocations cannot be mapped.
    pub unsafe fn map_memory(&self, allocation: Allocation) -> VkResult<*mut u8> {
        let mut mapped_data: *mut ::std::os::raw::c_void = ::std::ptr::null_mut();
        ffi_to_result(ffi::vmaMapMemory(
            self.internal,
            allocation,
            &mut mapped_data,
        ))?;

        Ok(mapped_data as *mut u8)
    }

    /// Unmaps memory represented by given allocation, mapped previously using `Allocator::map_memory`.
    pub unsafe fn unmap_memory(&self, allocation: Allocation) {
        ffi::vmaUnmapMemory(self.internal, allocation);
    }

    /// Flushes memory of given allocation.
    ///
    /// Calls `ash::vk::Device::FlushMappedMemoryRanges` for memory associated with given range of given allocation.
    ///
    /// - `offset` must be relative to the beginning of allocation.
    /// - `size` can be `ash::vk::WHOLE_SIZE`. It means all memory from `offset` the the end of given allocation.
    /// - `offset` and `size` don't have to be aligned; hey are internally rounded down/up to multiple of `nonCoherentAtomSize`.
    /// - If `size` is 0, this call is ignored.
    /// - If memory type that the `allocation` belongs to is not `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` or it is `ash::vk::MemoryPropertyFlags::HOST_COHERENT`, this call is ignored.
    pub unsafe fn flush_allocation(
        &self,
        allocation: Allocation,
        offset: usize,
        size: usize,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaFlushAllocation(
            self.internal,
            allocation,
            offset as vk::DeviceSize,
            size as vk::DeviceSize,
        ))
    }

    /// Invalidates memory of given allocation.
    ///
    /// Calls `ash::vk::Device::invalidate_mapped_memory_ranges` for memory associated with given range of given allocation.
    ///
    /// - `offset` must be relative to the beginning of allocation.
    /// - `size` can be `ash::vk::WHOLE_SIZE`. It means all memory from `offset` the the end of given allocation.
    /// - `offset` and `size` don't have to be aligned. They are internally rounded down/up to multiple of `nonCoherentAtomSize`.
    /// - If `size` is 0, this call is ignored.
    /// - If memory type that the `allocation` belongs to is not `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` or it is `ash::vk::MemoryPropertyFlags::HOST_COHERENT`, this call is ignored.
    pub unsafe fn invalidate_allocation(
        &self,
        allocation: Allocation,
        offset: usize,
        size: usize,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaInvalidateAllocation(
            self.internal,
            allocation,
            offset as vk::DeviceSize,
            size as vk::DeviceSize,
        ))
    }

    /// Checks magic number in margins around all allocations in given memory types (in both default and custom pools) in search for corruptions.
    ///
    /// `memory_type_bits` bit mask, where each bit set means that a memory type with that index should be checked.
    ///
    /// Corruption detection is enabled only when `VMA_DEBUG_DETECT_CORRUPTION` macro is defined to nonzero,
    /// `VMA_DEBUG_MARGIN` is defined to nonzero and only for memory types that are `HOST_VISIBLE` and `HOST_COHERENT`.
    ///
    /// Possible error values:
    ///
    /// - `ash::vk::Result::ERROR_FEATURE_NOT_PRESENT` - corruption detection is not enabled for any of specified memory types.
    /// - `ash::vk::Result::ERROR_VALIDATION_FAILED_EXT` - corruption detection has been performed and found memory corruptions around one of the allocations.
    ///   `VMA_ASSERT` is also fired in that case.
    /// - Other value: Error returned by Vulkan, e.g. memory mapping failure.
    pub unsafe fn check_corruption(
        &self,
        memory_types: ash::vk::MemoryPropertyFlags,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaCheckCorruption(
            self.internal,
            memory_types.as_raw(),
        ))
    }

    /// Begins defragmentation process.
    ///
    /// ## Returns
    /// `VK_SUCCESS` if defragmentation can begin.
    /// `VK_ERROR_FEATURE_NOT_PRESENT` if defragmentation is not supported.
    pub unsafe fn begin_defragmentation(
        &self,
        info: &ffi::VmaDefragmentationInfo,
    ) -> VkResult<ffi::VmaDefragmentationContext> {
        let mut context: ffi::VmaDefragmentationContext = std::ptr::null_mut();

        ffi::vmaBeginDefragmentation(self.internal, info, &mut context).result()?;

        Ok(context)
    }

    /// Ends defragmentation process.
    ///
    /// Use this function to finish defragmentation started by `Allocator::begin_defragmentation`.
    pub unsafe fn end_defragmentation(
        &self,
        context: ffi::VmaDefragmentationContext,
    ) -> ffi::VmaDefragmentationStats {
        let mut stats = ffi::VmaDefragmentationStats {
            bytesMoved: 0,
            bytesFreed: 0,
            allocationsMoved: 0,
            deviceMemoryBlocksFreed: 0,
        };
        ffi::vmaEndDefragmentation(self.internal, context, &mut stats);

        stats
    }

    /// Ends single defragmentation pass.
    /// * `context` - Context object that has been created by `begin_defragmentation`
    /// * `pass_info` - Coputed informations for current pass filled by `begin_defragmentation_pass` and possibly modified by you.
    ///
    /// Returns:
    /// - `false` if no more moves are possible. Then you can omit call to vmaEndDefragmentationPass() and simply end whole defragmentation.
    /// - `true` if there are pending moves returned in pPassInfo. You need to perform them, call vmaEndDefragmentationPass(), and then
    /// preferably try another pass with vmaBeginDefragmentationPass().
    pub unsafe fn begin_defragmentation_pass(
        &self,
        context: ffi::VmaDefragmentationContext,
        pass_info: &mut ffi::VmaDefragmentationPassMoveInfo,
    ) -> bool {
        let result = ffi::vmaBeginDefragmentationPass(self.internal, context, pass_info);
        return result == vk::Result::INCOMPLETE;
    }

    /// Ends single defragmentation pass.
    /// * `context` - Context object that has been created by `begin_defragmentation`
    /// * `pass_info` - Coputed informations for current pass filled by `begin_defragmentation_pass` and possibly modified by you.
    ///
    /// Returns false if no more moves are possible or true if more defragmentations are possible.
    pub unsafe fn end_defragmentation_pass(
        &self,
        context: ffi::VmaDefragmentationContext,
        pass_info: &mut ffi::VmaDefragmentationPassMoveInfo,
    ) -> bool {
        let result = ffi::vmaEndDefragmentationPass(self.internal, context, pass_info);
        return result == vk::Result::INCOMPLETE;
    }

    /// Binds buffer to allocation.
    ///
    /// Binds specified buffer to region of memory represented by specified allocation.
    /// Gets `ash::vk::DeviceMemory` handle and offset from the allocation.
    ///
    /// If you want to create a buffer, allocate memory for it and bind them together separately,
    /// you should use this function for binding instead of `ash::vk::Device::bind_buffer_memory`,
    /// because it ensures proper synchronization so that when a `ash::vk::DeviceMemory` object is
    /// used by multiple allocations, calls to `ash::vk::Device::bind_buffer_memory()` or
    /// `ash::vk::Device::map_memory()` won't happen from multiple threads simultaneously
    /// (which is illegal in Vulkan).
    ///
    /// It is recommended to use function `Allocator::create_buffer` instead of this one.
    pub unsafe fn bind_buffer_memory(
        &self,
        buffer: ash::vk::Buffer,
        allocation: Allocation,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaBindBufferMemory(self.internal, allocation, buffer))
    }

    /// Binds image to allocation.
    ///
    /// Binds specified image to region of memory represented by specified allocation.
    /// Gets `ash::vk::DeviceMemory` handle and offset from the allocation.
    ///
    /// If you want to create a image, allocate memory for it and bind them together separately,
    /// you should use this function for binding instead of `ash::vk::Device::bind_image_memory`,
    /// because it ensures proper synchronization so that when a `ash::vk::DeviceMemory` object is
    /// used by multiple allocations, calls to `ash::vk::Device::bind_image_memory()` or
    /// `ash::vk::Device::map_memory()` won't happen from multiple threads simultaneously
    /// (which is illegal in Vulkan).
    ///
    /// It is recommended to use function `Allocator::create_image` instead of this one.
    pub unsafe fn bind_image_memory(
        &self,
        image: ash::vk::Image,
        allocation: Allocation,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaBindImageMemory(self.internal, allocation, image))
    }

    /// This function automatically creates a buffer, allocates appropriate memory
    /// for it, and binds the buffer with the memory.
    ///
    /// If the function succeeded, you must destroy both buffer and allocation when you
    /// no longer need them using either convenience function `Allocator::destroy_buffer` or
    /// separately, using `ash::Device::destroy_buffer` and `Allocator::free_memory`.
    ///
    /// If `AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION` flag was used,
    /// VK_KHR_dedicated_allocation extension is used internally to query driver whether
    /// it requires or prefers the new buffer to have dedicated allocation. If yes,
    /// and if dedicated allocation is possible (AllocationCreateInfo::pool is null
    /// and `AllocationCreateFlags::NEVER_ALLOCATE` is not used), it creates dedicated
    /// allocation for this buffer, just like when using `AllocationCreateFlags::DEDICATED_MEMORY`.
    pub unsafe fn create_buffer(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        allocation_create_info: &AllocationCreateInfo,
    ) -> VkResult<(ash::vk::Buffer, Allocation, AllocationInfo)> {
        let mut buffer = vk::Buffer::null();
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaCreateBuffer(
            self.internal,
            &*buffer_info,
            &allocation_create_info.inner,
            &mut buffer,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((buffer, allocation, allocation_info))
    }

    /// Destroys Vulkan buffer and frees allocated memory.
    ///
    /// This is just a convenience function equivalent to:
    ///
    /// ```ignore
    /// ash::vk::Device::destroy_buffer(buffer, None);
    /// Allocator::free_memory(allocator, allocation);
    /// ```
    ///
    /// It it safe to pass null as `buffer` and/or `allocation`.
    pub unsafe fn destroy_buffer(&self, buffer: ash::vk::Buffer, allocation: Allocation) {
        ffi::vmaDestroyBuffer(self.internal, buffer, allocation);
    }

    /// This function automatically creates an image, allocates appropriate memory
    /// for it, and binds the image with the memory.
    ///
    /// If the function succeeded, you must destroy both image and allocation when you
    /// no longer need them using either convenience function `Allocator::destroy_image` or
    /// separately, using `ash::Device::destroy_image` and `Allocator::free_memory`.
    ///
    /// If `AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION` flag was used,
    /// `VK_KHR_dedicated_allocation extension` is used internally to query driver whether
    /// it requires or prefers the new image to have dedicated allocation. If yes,
    /// and if dedicated allocation is possible (AllocationCreateInfo::pool is null
    /// and `AllocationCreateFlags::NEVER_ALLOCATE` is not used), it creates dedicated
    /// allocation for this image, just like when using `AllocationCreateFlags::DEDICATED_MEMORY`.
    ///
    /// If `VK_ERROR_VALIDAITON_FAILED_EXT` is returned, VMA may have encountered a problem
    /// that is not caught by the validation layers. One example is if you try to create a 0x0
    /// image, a panic will occur and `VK_ERROR_VALIDAITON_FAILED_EXT` is thrown.
    pub unsafe fn create_image(
        &self,
        image_info: &ash::vk::ImageCreateInfo,
        allocation_create_info: &AllocationCreateInfo,
    ) -> VkResult<(ash::vk::Image, Allocation, AllocationInfo)> {
        let mut image = vk::Image::null();
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaCreateImage(
            self.internal,
            &*image_info,
            &allocation_create_info.inner,
            &mut image,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((image, allocation, allocation_info))
    }

    /// Destroys Vulkan image and frees allocated memory.
    ///
    /// This is just a convenience function equivalent to:
    ///
    /// ```ignore
    /// ash::vk::Device::destroy_image(image, None);
    /// Allocator::free_memory(allocator, allocation);
    /// ```
    ///
    /// It it safe to pass null as `image` and/or `allocation`.
    pub unsafe fn destroy_image(&self, image: ash::vk::Image, allocation: Allocation) {
        ffi::vmaDestroyImage(self.internal, image, allocation);
    }

    /// Destroys the internal allocator instance. After this has been called,
    /// no other functions may be called. Useful for ensuring a specific destruction
    /// order (for example, if an Allocator is a member of something that owns the Vulkan
    /// instance and destroys it in its own Drop).
    pub unsafe fn destroy(&mut self) {
        if !self.internal.is_null() {
            ffi::vmaDestroyAllocator(self.internal);
            self.internal = std::ptr::null_mut();
        }
    }
}

/// Custom `Drop` implementation to clean up internal allocation instance
impl Drop for Allocator {
    fn drop(&mut self) {
        unsafe {
            self.destroy();
        }
    }
}
