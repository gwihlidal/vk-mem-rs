//! Easy to use, high performance memory manager for Vulkan.

mod definitions;
mod defragmentation;
mod ffi;
mod pool;
pub use definitions::*;
pub use defragmentation::*;
pub use pool::*;

use ash::prelude::VkResult;
use ash::vk;
use std::mem;
use std::ops::Deref;

/// Main allocator object
pub struct Allocator {
    /// Pointer to internal VmaAllocator instance
    internal: ffi::VmaAllocator,
}

// Allocator is internally thread safe unless AllocatorCreateFlags::EXTERNALLY_SYNCHRONIZED is used (then you need to add synchronization!)
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

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
#[derive(Debug)]
pub struct Allocation(ffi::VmaAllocation);
unsafe impl Send for Allocation {}
unsafe impl Sync for Allocation {}

impl Allocator {
    /// Constructor a new `Allocator` using the provided options.
    pub fn new<'a, I, D>(mut create_info: AllocatorCreateInfo<'a, I, D>) -> VkResult<Self>
    where
        I: Deref<Target = ash::Instance>,
        D: Deref<Target = ash::Device>,
    {
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
        #[cfg(feature = "loaded")]
        {
            create_info.inner.pVulkanFunctions = &routed_functions;
        }
        unsafe {
            let mut internal: ffi::VmaAllocator = mem::zeroed();
            ffi::vmaCreateAllocator(&create_info.inner as *const _, &mut internal).result()?;

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
    pub unsafe fn get_memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        let mut properties: *const vk::PhysicalDeviceMemoryProperties = std::ptr::null();
        ffi::vmaGetMemoryProperties(self.internal, &mut properties);

        &*properties
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
    pub fn calculate_statistics(&self) -> VkResult<ffi::VmaTotalStatistics> {
        unsafe {
            let mut vma_stats: ffi::VmaTotalStatistics = mem::zeroed();
            ffi::vmaCalculateStatistics(self.internal, &mut vma_stats);
            Ok(vma_stats)
        }
    }

    /// Retrieves information about current memory usage and budget for all memory heaps.
    ///
    /// This function is called "get" not "calculate" because it is very fast, suitable to be called
    /// every frame or every allocation. For more detailed statistics use vmaCalculateStatistics().
    ///
    /// Note that when using allocator from multiple threads, returned information may immediately
    /// become outdated.
    pub fn get_heap_budgets(&self) -> VkResult<Vec<ffi::VmaBudget>> {
        unsafe {
            let len = self.get_memory_properties().memory_heap_count as usize;
            let mut vma_budgets: Vec<ffi::VmaBudget> = Vec::with_capacity(len);
            ffi::vmaGetHeapBudgets(self.internal, vma_budgets.as_mut_ptr());
            vma_budgets.set_len(len);
            Ok(vma_budgets)
        }
    }

    /// Frees memory previously allocated using `Allocator::allocate_memory`,
    /// `Allocator::allocate_memory_for_buffer`, or `Allocator::allocate_memory_for_image`.
    pub unsafe fn free_memory(&self, allocation: Allocation) {
        ffi::vmaFreeMemory(self.internal, allocation.0);
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
    pub unsafe fn get_allocation_info(&self, allocation: &Allocation) -> VkResult<AllocationInfo> {
        let mut allocation_info: ffi::VmaAllocationInfo = mem::zeroed();
        ffi::vmaGetAllocationInfo(self.internal, allocation.0, &mut allocation_info);
        Ok(allocation_info.into())
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
        allocation: &mut Allocation,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        ffi::vmaSetAllocationUserData(self.internal, allocation.0, user_data);
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
    pub unsafe fn map_memory(&self, allocation: &mut Allocation) -> VkResult<*mut u8> {
        let mut mapped_data: *mut ::std::os::raw::c_void = ::std::ptr::null_mut();
        ffi::vmaMapMemory(self.internal, allocation.0, &mut mapped_data).result()?;

        Ok(mapped_data as *mut u8)
    }

    /// Unmaps memory represented by given allocation, mapped previously using `Allocator::map_memory`.
    pub unsafe fn unmap_memory(&self, allocation: &mut Allocation) {
        ffi::vmaUnmapMemory(self.internal, allocation.0);
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
    pub fn flush_allocation(
        &self,
        allocation: &Allocation,
        offset: usize,
        size: usize,
    ) -> VkResult<()> {
        unsafe {
            ffi::vmaFlushAllocation(
                self.internal,
                allocation.0,
                offset as vk::DeviceSize,
                size as vk::DeviceSize,
            )
            .result()
        }
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
    pub fn invalidate_allocation(
        &self,
        allocation: &Allocation,
        offset: usize,
        size: usize,
    ) -> VkResult<()> {
        unsafe {
            ffi::vmaInvalidateAllocation(
                self.internal,
                allocation.0,
                offset as vk::DeviceSize,
                size as vk::DeviceSize,
            )
            .result()
        }
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
        ffi::vmaCheckCorruption(self.internal, memory_types.as_raw()).result()
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
        allocation: &Allocation,
        buffer: ash::vk::Buffer,
    ) -> VkResult<()> {
        ffi::vmaBindBufferMemory(self.internal, allocation.0, buffer).result()
    }

    /// Binds buffer to allocation with additional parameters.
    ///
    /// * `allocation`
    /// * `allocation_local_offset` - Additional offset to be added while binding, relative to the beginning of the `allocation`. Normally it should be 0.
    /// * `buffer`
    /// * `next` - A chain of structures to be attached to `VkBindImageMemoryInfoKHR` structure used internally. Normally it should be null.
    ///
    /// This function is similar to vmaBindImageMemory(), but it provides additional parameters.
    ///
    /// If `pNext` is not null, #VmaAllocator object must have been created with #VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT flag
    /// or with VmaAllocatorCreateInfo::vulkanApiVersion `>= VK_API_VERSION_1_1`. Otherwise the call fails.
    pub unsafe fn bind_buffer_memory2(
        &self,
        allocation: &Allocation,
        allocation_local_offset: vk::DeviceSize,
        buffer: ash::vk::Buffer,
        next: *const ::std::os::raw::c_void,
    ) -> VkResult<()> {
        ffi::vmaBindBufferMemory2(
            self.internal,
            allocation.0,
            allocation_local_offset,
            buffer,
            next,
        )
        .result()
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
        allocation: &Allocation,
        image: ash::vk::Image,
    ) -> VkResult<()> {
        ffi::vmaBindImageMemory(self.internal, allocation.0, image).result()
    }

    /// Binds image to allocation with additional parameters.
    ///
    /// * `allocation`
    /// * `allocation_local_offset` - Additional offset to be added while binding, relative to the beginning of the `allocation`. Normally it should be 0.
    /// * `image`
    /// * `next` - A chain of structures to be attached to `VkBindImageMemoryInfoKHR` structure used internally. Normally it should be null.
    ///
    /// This function is similar to vmaBindImageMemory(), but it provides additional parameters.
    ///
    /// If `pNext` is not null, #VmaAllocator object must have been created with #VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT flag
    /// or with VmaAllocatorCreateInfo::vulkanApiVersion `>= VK_API_VERSION_1_1`. Otherwise the call fails.
    pub unsafe fn bind_image_memory2(
        &self,
        allocation: &Allocation,
        allocation_local_offset: vk::DeviceSize,
        image: ash::vk::Image,
        next: *const ::std::os::raw::c_void,
    ) -> VkResult<()> {
        ffi::vmaBindImageMemory2(
            self.internal,
            allocation.0,
            allocation_local_offset,
            image,
            next,
        )
        .result()
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
        ffi::vmaDestroyBuffer(self.internal, buffer, allocation.0);
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
        ffi::vmaDestroyImage(self.internal, image, allocation.0);
    }
    /// Flushes memory of given set of allocations."]
    ///
    /// Calls `vkFlushMappedMemoryRanges()` for memory associated with given ranges of given allocations."]
    /// For more information, see documentation of vmaFlushAllocation()."]
    ///
    /// * `allocations`
    /// * `offsets` - If not None, it must be a slice of offsets of regions to flush, relative to the beginning of respective allocations. None means all ofsets are zero.
    /// * `sizes` - If not None, it must be a slice of sizes of regions to flush in respective allocations. None means `VK_WHOLE_SIZE` for all allocations.
    pub unsafe fn flush_allocations<'a>(
        &self,
        allocations: impl IntoIterator<Item = &'a Allocation>,
        offsets: Option<&[vk::DeviceSize]>,
        sizes: Option<&[vk::DeviceSize]>,
    ) -> VkResult<()> {
        let allocations: Vec<ffi::VmaAllocation> = allocations.into_iter().map(|a| a.0).collect();
        ffi::vmaFlushAllocations(
            self.internal,
            allocations.len() as u32,
            allocations.as_ptr() as *mut _,
            offsets.map_or(std::ptr::null(), |offsets| offsets.as_ptr()),
            sizes.map_or(std::ptr::null(), |sizes| sizes.as_ptr()),
        )
        .result()
    }

    /// Invalidates memory of given set of allocations."]
    ///
    /// Calls `vkInvalidateMappedMemoryRanges()` for memory associated with given ranges of given allocations."]
    /// For more information, see documentation of vmaInvalidateAllocation()."]
    ///
    /// * `allocations`
    /// * `offsets` - If not None, it must be a slice of offsets of regions to flush, relative to the beginning of respective allocations. None means all ofsets are zero.
    /// * `sizes` - If not None, it must be a slice of sizes of regions to flush in respective allocations. None means `VK_WHOLE_SIZE` for all allocations.
    pub unsafe fn invalidate_allocations<'a>(
        &self,
        allocations: impl IntoIterator<Item = &'a Allocation>,
        offsets: Option<&[vk::DeviceSize]>,
        sizes: Option<&[vk::DeviceSize]>,
    ) -> VkResult<()> {
        let allocations: Vec<ffi::VmaAllocation> = allocations.into_iter().map(|a| a.0).collect();
        ffi::vmaInvalidateAllocations(
            self.internal,
            allocations.len() as u32,
            allocations.as_ptr() as *mut _,
            offsets.map_or(std::ptr::null(), |offsets| offsets.as_ptr()),
            sizes.map_or(std::ptr::null(), |sizes| sizes.as_ptr()),
        )
        .result()
    }
}

/// Custom `Drop` implementation to clean up internal allocation instance
impl Drop for Allocator {
    fn drop(&mut self) {
        unsafe {
            ffi::vmaDestroyAllocator(self.internal);
            self.internal = std::ptr::null_mut();
        }
    }
}
