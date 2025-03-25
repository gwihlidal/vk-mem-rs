//! Easy to use, high performance memory manager for Vulkan.

mod definitions;
mod defragmentation;
mod error;
mod ffi;
mod pool;
mod virtual_block;

pub use definitions::*;
pub use defragmentation::*;
pub use pool::*;
pub use virtual_block::*;

use crate::error::{VmaError, VmaResult};
use crate::ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT;
use crate::ffi::{VmaAllocatorCreateFlagBits, VmaAllocatorCreateFlags};
use ash::vk;
use std::mem;
use std::ops::BitAnd;
use std::panic::Location;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub(crate) type AllocationId = u64;

/// Main allocator object
#[derive(Clone)]
pub struct Allocator {
    /// Pointer to internal VmaAllocator instance
    internal: ffi::VmaAllocator,

    /// Vulkan API version
    pub(crate) vulkan_api_version: u32,

    /// Creation Flags
    pub(crate) internal_create_flags: VmaAllocatorCreateFlags,

    /// List of all mapped allocators, for unmap verification and leak detection
    pub(crate) mapped_allocators: Arc<Mutex<Vec<AllocationId>>>,

    /// Counter for generating unique allocation IDs
    pub(crate) next_allocation_id: Arc<AtomicU64>,
}

// Allocator is internally thread safe unless AllocatorCreateFlags::EXTERNALLY_SYNCHRONIZED is used (then you need to add synchronization!)
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

/// Represents single memory allocation.
///
/// It may be either dedicated block of `vk::DeviceMemory` or a specific region of a
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
#[derive(Clone, Copy)]
pub struct Allocation {
    /// Pointer to internal VmaAllocation instance
    pub(crate) internal: ffi::VmaAllocation,

    /// Unique identifier for each Allocation inside an Allocator
    pub(crate) id: AllocationId,

    /// Create info that this Allocation was created with
    pub(crate) info: AllocationCreateInfo,
}

unsafe impl Send for Allocation {}
unsafe impl Sync for Allocation {}

impl BitAnd<VmaAllocatorCreateFlagBits> for VmaAllocatorCreateFlags {
    type Output = u32;

    fn bitand(self, rhs: VmaAllocatorCreateFlagBits) -> Self::Output {
        (self as u32) & (rhs as u32)
    }
}

impl Allocator {
    /// Construct a new `Allocator` using the provided options.
    ///
    /// # Safety
    /// [`AllocatorCreateInfo::instance`], [`AllocatorCreateInfo::device`] and
    /// [`AllocatorCreateInfo::physical_device`] must be valid throughout the lifetime of the allocator.
    pub fn new(create_info: AllocatorCreateInfo) -> VmaResult<Self> {
        unsafe extern "system" fn get_instance_proc_addr_stub(
            _instance: vk::Instance,
            _p_name: *const std::os::raw::c_char,
        ) -> vk::PFN_vkVoidFunction {
            panic!("VMA_DYNAMIC_VULKAN_FUNCTIONS is unsupported")
        }

        unsafe extern "system" fn get_get_device_proc_stub(
            _device: vk::Device,
            _p_name: *const std::os::raw::c_char,
        ) -> vk::PFN_vkVoidFunction {
            panic!("VMA_DYNAMIC_VULKAN_FUNCTIONS is unsupported")
        }

        let mut raw_create_info = ffi::VmaAllocatorCreateInfo {
            flags: create_info.flags.bits(),
            physicalDevice: create_info.physical_device,
            device: create_info.device.handle(),
            preferredLargeHeapBlockSize: create_info.preferred_large_heap_block_size,
            pAllocationCallbacks: create_info
                .allocation_callbacks
                .map(|a| unsafe { mem::transmute(a) })
                .unwrap_or(std::ptr::null()),
            pDeviceMemoryCallbacks: create_info
                .device_memory_callbacks
                .map(|a| a as *const _)
                .unwrap_or(std::ptr::null()),
            pHeapSizeLimit: if create_info.heap_size_limits.is_empty() {
                std::ptr::null()
            } else {
                create_info.heap_size_limits.as_ptr()
            },
            instance: create_info.instance.handle(),
            vulkanApiVersion: create_info.vulkan_api_version,
            pVulkanFunctions: std::ptr::null(),
            pTypeExternalMemoryHandleTypes: if create_info
                .type_external_memory_handle_types
                .is_empty()
            {
                std::ptr::null()
            } else {
                create_info.type_external_memory_handle_types.as_ptr()
            },
        };

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
            raw_create_info.pVulkanFunctions = &routed_functions;
        }
        unsafe {
            let mut internal: ffi::VmaAllocator = mem::zeroed();
            ffi::vmaCreateAllocator(&raw_create_info, &mut internal).result()?;

            // SAFETY:
            // Make sure the allocator is initialized before returning it
            if internal.is_null() {
                return Err(VmaError::NotInitialized());
            }

            Ok(Allocator {
                internal,
                vulkan_api_version: create_info.vulkan_api_version,
                internal_create_flags: create_info.flags.bits(),
                mapped_allocators: Arc::new(Mutex::new(Vec::new())),
                next_allocation_id: Arc::new(AtomicU64::new(1)),
            })
        }
    }

    /// Get the next unique allocation ID
    fn generate_allocation_id(&self) -> AllocationId {
        self.next_allocation_id.fetch_add(1, Ordering::SeqCst)
    }

    /// The allocator fetches `vk::PhysicalDeviceProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub unsafe fn get_physical_device_properties(&self) -> VmaResult<vk::PhysicalDeviceProperties> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        let mut properties = vk::PhysicalDeviceProperties::default();
        ffi::vmaGetPhysicalDeviceProperties(
            self.internal,
            &mut properties as *mut _ as *mut *const _,
        );

        Ok(properties)
    }

    /// The allocator fetches `vk::PhysicalDeviceMemoryProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub fn get_memory_properties(&self) -> VmaResult<&vk::PhysicalDeviceMemoryProperties> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        unsafe {
            let mut properties: *const vk::PhysicalDeviceMemoryProperties = std::ptr::null();
            ffi::vmaGetMemoryProperties(self.internal, &mut properties);

            Ok(&*properties)
        }
    }

    /// Sets index of the current frame.
    ///
    /// This function must be used if you make allocations with `AllocationCreateFlags::CAN_BECOME_LOST` and
    /// `AllocationCreateFlags::CAN_MAKE_OTHER_LOST` flags to inform the allocator when a new frame begins.
    /// Allocations queried using `Allocator::get_allocation_info` cannot become lost
    /// in the current frame.
    pub fn set_current_frame_index(&self, frame_index: u32) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        unsafe {
            ffi::vmaSetCurrentFrameIndex(self.internal, frame_index);

            Ok(())
        }
    }

    /// Retrieves statistics from current state of the `Allocator`.
    pub fn calculate_statistics(&self) -> VmaResult<ffi::VmaTotalStatistics> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

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
    pub fn get_heap_budgets(&self) -> VmaResult<Vec<ffi::VmaBudget>> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        unsafe {
            let len = self.get_memory_properties()?.memory_heap_count as usize;
            let mut vma_budgets: Vec<ffi::VmaBudget> = Vec::with_capacity(len);
            ffi::vmaGetHeapBudgets(self.internal, vma_budgets.as_mut_ptr());
            vma_budgets.set_len(len);
            Ok(vma_budgets)
        }
    }

    /// Frees memory previously allocated using `Allocator::allocate_memory`,
    /// `Allocator::allocate_memory_for_buffer`, or `Allocator::allocate_memory_for_image`.
    pub fn free_memory(&self, allocation: &mut Allocation) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to free an invalid Allocation",
            ));
        }

        unsafe {
            ffi::vmaFreeMemory(self.internal, allocation.internal);
        }

        Ok(())
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
    pub fn free_memory_pages(&self, allocations: &mut [Allocation]) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        unsafe {
            ffi::vmaFreeMemoryPages(
                self.internal,
                allocations.len(),
                allocations.as_ptr() as *mut _,
            );
        }

        Ok(())
    }

    /// Returns current information about specified allocation and atomically marks it as used in current frame.
    ///
    /// Current parameters of given allocation are returned in the result object, available through accessors.
    ///
    /// This function also atomically "touches" allocation - marks it as used in current frame,
    /// just like `Allocator::touch_allocation`.
    ///
    /// If the allocation is in lost state, `allocation.get_device_memory` returns `vk::DeviceMemory::null()`.
    ///
    /// Although this function uses atomics and doesn't lock any mutex, so it should be quite efficient,
    /// you can avoid calling it too often.
    ///
    /// If you just want to check if allocation is not lost, `Allocator::touch_allocation` will work faster.
    pub fn get_allocation_info(&self, allocation: &Allocation) -> VmaResult<AllocationInfo> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to get info for an invalid Allocation",
            ));
        }

        unsafe {
            let mut allocation_info: ffi::VmaAllocationInfo = mem::zeroed();
            ffi::vmaGetAllocationInfo(self.internal, allocation.internal, &mut allocation_info);
            Ok(allocation_info.into())
        }
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
    pub fn set_allocation_user_data(
        &self,
        allocation: &mut Allocation,
        user_data: *mut std::os::raw::c_void,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to set user data for an invalid Allocation",
            ));
        }

        unsafe {
            ffi::vmaSetAllocationUserData(self.internal, allocation.internal, user_data);
        }

        Ok(())
    }

    /// Maps memory represented by given allocation and returns pointer to it.
    ///
    /// Maps memory represented by given allocation to make it accessible to CPU code.
    /// When succeeded, result is a pointer to first byte of this memory.
    ///
    /// If the allocation is part of bigger `vk::DeviceMemory` block, the pointer is
    /// correctly offset to the beginning of region assigned to this particular
    /// allocation.
    ///
    /// Mapping is internally reference-counted and synchronized, so despite raw Vulkan
    /// function `vk::Device::MapMemory` cannot be used to map same block of
    /// `vk::DeviceMemory` multiple times simultaneously, it is safe to call this
    /// function on allocations assigned to the same memory block. Actual Vulkan memory
    /// will be mapped on first mapping and unmapped on last unmapping.
    ///
    /// If the function succeeded, you must call `Allocator::unmap_memory` to unmap the
    /// allocation when mapping is no longer needed or before freeing the allocation, at
    /// the latest.
    ///
    /// It is also safe to call this function multiple times on the same allocation. You
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
    /// `vk::MemoryPropertyFlags::HOST_VISIBLE`.
    #[track_caller]
    pub fn map_memory(&self, allocation: &mut Allocation) -> VmaResult<*mut u8> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to map an invalid Allocation",
            ));
        }

        if allocation.info.required_flags & vk::MemoryPropertyFlags::HOST_VISIBLE
            != vk::MemoryPropertyFlags::HOST_VISIBLE
        {
            return Err(VmaError::MappingNotHostVisible(
                "Attempted to map an allocation that is not HOST_VISIBLE", Location::caller(),
            ));
        }

        let mut mapped_allocators = self.mapped_allocators.lock().unwrap();

        unsafe {
            let mut mapped_data: *mut std::os::raw::c_void = std::ptr::null_mut();
            ffi::vmaMapMemory(self.internal, allocation.internal, &mut mapped_data).result()?;

            // Sanity check
            if mapped_data.is_null() {
                return Err(VmaError::UnableToMapMemory(
                    "Failed to map memory, returned null pointer", Location::caller(),
                ));
            }

            // Mark allocation as mapped
            mapped_allocators.push(allocation.id);

            Ok(mapped_data as *mut u8)
        }
    }

    /// Unmaps memory represented by given allocation, mapped previously using `Allocator::map_memory`.
    #[track_caller]
    pub fn unmap_memory(&self, allocation: &mut Allocation) -> VmaResult<()> {
        //
        // Sanity check:
        // - Make sure the allocation is actually mapped at least once excluding the 0th mapping
        // - as map_memory requires.
        //
        let mapped_allocators = self.mapped_allocators.lock().unwrap();
        let found = mapped_allocators.iter().find(|&x| *x == allocation.id);

        if found.is_none() {
            return Err(VmaError::PreviouslyUnmapped(
                "Attempted to unmap an allocation that was not previously mapped at {}", Location::caller()));
        }

        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to unmap an invalid Allocation",
            ));
        }

        unsafe {
            ffi::vmaUnmapMemory(self.internal, allocation.internal);
        }

        // Remove only the first occurrence of this allocation id
        let mut mapped = self.mapped_allocators.lock().unwrap();
        if let Some(index) = mapped.iter().position(|x| *x == allocation.id) {
            mapped.remove(index);
        }

        Ok(())
    }

    /// Flushes memory of given allocation.
    ///
    /// Calls `vk::Device::FlushMappedMemoryRanges` for memory associated with given range of given allocation.
    ///
    /// - `offset` must be relative to the beginning of allocation.
    /// - `size` can be `vk::WHOLE_SIZE`. It means all memory from `offset` the end of given allocation.
    /// - `offset` and `size` don't have to be aligned; hey are internally rounded down/up to multiple of `nonCoherentAtomSize`.
    /// - If `size` is 0, this call is ignored.
    /// - If memory type that the `allocation` belongs to is not `vk::MemoryPropertyFlags::HOST_VISIBLE` or it is `vk::MemoryPropertyFlags::HOST_COHERENT`, this call is ignored.
    pub fn flush_allocation(
        &self,
        allocation: &Allocation,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to flush an invalid Allocation",
            ));
        }

        unsafe {
            ffi::vmaFlushAllocation(self.internal, allocation.internal, offset, size)
                .result()
                .map_err(VmaError::from)
        }
    }

    /// Invalidates memory of given allocation.
    ///
    /// Calls `vk::Device::invalidate_mapped_memory_ranges` for memory associated with given range of given allocation.
    ///
    /// - `offset` must be relative to the beginning of allocation.
    /// - `size` can be `vk::WHOLE_SIZE`. It means all memory from `offset` the end of given allocation.
    /// - `offset` and `size` don't have to be aligned. They are internally rounded down/up to multiple of `nonCoherentAtomSize`.
    /// - If `size` is 0, this call is ignored.
    /// - If memory type that the `allocation` belongs to is not `vk::MemoryPropertyFlags::HOST_VISIBLE` or it is `vk::MemoryPropertyFlags::HOST_COHERENT`, this call is ignored.
    pub fn invalidate_allocation(
        &self,
        allocation: &Allocation,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to invalidate an invalid Allocation",
            ));
        }

        unsafe {
            ffi::vmaInvalidateAllocation(self.internal, allocation.internal, offset, size)
                .result()
                .map_err(VmaError::from)
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
    /// - `vk::Result::ERROR_FEATURE_NOT_PRESENT` - corruption detection is not enabled for any of specified memory types.
    /// - `vk::Result::ERROR_VALIDATION_FAILED_EXT` - corruption detection has been performed and found memory corruptions around one of the allocations.
    ///   `VMA_ASSERT` is also fired in that case.
    /// - Other value: Error returned by Vulkan, e.g. memory mapping failure.
    pub fn check_corruption(&self, memory_types: vk::MemoryPropertyFlags) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        unsafe {
            ffi::vmaCheckCorruption(self.internal, memory_types.as_raw())
                .result()
                .map_err(VmaError::from)
        }
    }

    /// Binds buffer to allocation.
    ///
    /// Binds specified buffer to region of memory represented by specified allocation.
    /// Gets `vk::DeviceMemory` handle and offset from the allocation.
    ///
    /// If you want to create a buffer, allocate memory for it and bind them together separately,
    /// you should use this function for binding instead of `vk::Device::bind_buffer_memory`,
    /// because it ensures proper synchronization so that when a `vk::DeviceMemory` object is
    /// used by multiple allocations, calls to `vk::Device::bind_buffer_memory()` or
    /// `vk::Device::map_memory()` won't happen from multiple threads simultaneously
    /// (which is illegal in Vulkan).
    ///
    /// It is recommended to use function `Allocator::create_buffer` instead of this one.
    pub fn bind_buffer_memory(&self, allocation: &Allocation, buffer: vk::Buffer) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to bind buffer to an invalid Allocation",
            ));
        }

        if buffer == vk::Buffer::null() {
            return Err(VmaError::InvalidBuffer());
        }

        unsafe {
            ffi::vmaBindBufferMemory(self.internal, allocation.internal, buffer)
                .result()
                .map_err(VmaError::from)
        }
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
    /// or with VmaAllocatorCreateInfo::vulkanApiVersion `>= VK_API_VERSION_1_1`. Otherwise, the call fails.
    pub unsafe fn bind_buffer_memory2(
        &self,
        allocation: &Allocation,
        allocation_local_offset: vk::DeviceSize,
        buffer: vk::Buffer,
        next: *const std::os::raw::c_void,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to bind buffer to an invalid Allocation",
            ));
        }

        if self.vulkan_api_version < vk::API_VERSION_1_1
            && (self.internal_create_flags & VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT) == 0
        {
            return Err(VmaError::UnsupportedVulkanFeature(
                "VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT or Vulkan 1.1 is required",
            ));
        }

        if buffer == vk::Buffer::null() {
            return Err(VmaError::InvalidBuffer());
        }

        ffi::vmaBindBufferMemory2(
            self.internal,
            allocation.internal,
            allocation_local_offset,
            buffer,
            next,
        )
        .result()
        .map_err(VmaError::from)
    }

    /// Binds image to allocation.
    ///
    /// Binds specified image to region of memory represented by specified allocation.
    /// Gets `vk::DeviceMemory` handle and offset from the allocation.
    ///
    /// If you want to create an image, allocate memory for it and bind them together separately,
    /// you should use this function for binding instead of `vk::Device::bind_image_memory`,
    /// because it ensures proper synchronization so that when a `vk::DeviceMemory` object is
    /// used by multiple allocations, calls to `vk::Device::bind_image_memory()` or
    /// `vk::Device::map_memory()` won't happen from multiple threads simultaneously
    /// (which is illegal in Vulkan).
    ///
    /// It is recommended to use function `Allocator::create_image` instead of this one.
    pub unsafe fn bind_image_memory(
        &self,
        allocation: &Allocation,
        image: vk::Image,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to bind image to an invalid Allocation",
            ));
        }

        ffi::vmaBindImageMemory(self.internal, allocation.internal, image)
            .result()
            .map_err(VmaError::from)
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
    /// or with VmaAllocatorCreateInfo::vulkanApiVersion `>= VK_API_VERSION_1_1`. Otherwise, the call fails.
    pub unsafe fn bind_image_memory2(
        &self,
        allocation: &Allocation,
        allocation_local_offset: vk::DeviceSize,
        image: vk::Image,
        next: *const std::os::raw::c_void,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        if allocation.internal.is_null() {
            return Err(VmaError::InvalidParameter(
                "Attempted to bind image to an invalid Allocation",
            ));
        }

        if self.vulkan_api_version < vk::API_VERSION_1_1
            && (self.internal_create_flags & VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT) == 0
        {
            return Err(VmaError::UnsupportedVulkanFeature(
                "VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT or Vulkan 1.1 is required",
            ));
        }

        if image == vk::Image::null() {
            return Err(VmaError::InvalidImage());
        }

        ffi::vmaBindImageMemory2(
            self.internal,
            allocation.internal,
            allocation_local_offset,
            image,
            next,
        )
        .result()
        .map_err(VmaError::from)
    }

    /// Destroys Vulkan buffer and frees allocated memory.
    ///
    /// This is just a convenience function equivalent to:
    ///
    /// ```ignore
    /// vk::Device::destroy_buffer(buffer, None);
    /// Allocator::free_memory(allocator, allocation);
    /// ```
    ///
    /// It is safe to pass null as `buffer` and/or `allocation`.
    pub unsafe fn destroy_buffer(
        &self,
        buffer: vk::Buffer,
        allocation: &mut Allocation,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        ffi::vmaDestroyBuffer(self.internal, buffer, allocation.internal);

        Ok(())
    }

    /// Destroys Vulkan image and frees allocated memory.
    ///
    /// This is just a convenience function equivalent to:
    ///
    /// ```ignore
    /// vk::Device::destroy_image(image, None);
    /// Allocator::free_memory(allocator, allocation);
    /// ```
    ///
    /// It is safe to pass null as `image` and/or `allocation`.
    pub unsafe fn destroy_image(
        &self,
        image: vk::Image,
        allocation: &mut Allocation,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        ffi::vmaDestroyImage(self.internal, image, allocation.internal);

        Ok(())
    }
    /// Flushes memory of given set of allocations.
    ///
    /// Calls `vkFlushMappedMemoryRanges()` for memory associated with given ranges of given allocations.
    /// For more information, see documentation of vmaFlushAllocation().
    ///
    /// * `allocations`
    /// * `offsets` - If not None, it must be a slice of offsets of regions to flush, relative to the beginning of respective allocations. None means all offsets are zero.
    /// * `sizes` - If not None, it must be a slice of sizes of regions to flush in respective allocations. None means `VK_WHOLE_SIZE` for all allocations.
    pub unsafe fn flush_allocations<'a>(
        &self,
        allocations: impl IntoIterator<Item = &'a Allocation>,
        offsets: Option<&[vk::DeviceSize]>,
        sizes: Option<&[vk::DeviceSize]>,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        let allocations: Vec<ffi::VmaAllocation> =
            allocations.into_iter().map(|a| a.internal).collect();
        ffi::vmaFlushAllocations(
            self.internal,
            allocations.len() as u32,
            allocations.as_ptr() as *mut _,
            offsets.map_or(std::ptr::null(), |offsets| offsets.as_ptr()),
            sizes.map_or(std::ptr::null(), |sizes| sizes.as_ptr()),
        )
        .result()
        .map_err(VmaError::from)
    }

    /// Invalidates memory of given set of allocations.
    ///
    /// Calls `vkInvalidateMappedMemoryRanges()` for memory associated with given ranges of given allocations.
    /// For more information, see documentation of vmaInvalidateAllocation().
    ///
    /// * `allocations`
    /// * `offsets` - If not None, it must be a slice of offsets of regions to flush, relative to the beginning of respective allocations. None means all offsets are zero.
    /// * `sizes` - If not None, it must be a slice of sizes of regions to flush in respective allocations. None means `VK_WHOLE_SIZE` for all allocations.
    pub unsafe fn invalidate_allocations<'a>(
        &self,
        allocations: impl IntoIterator<Item = &'a Allocation>,
        offsets: Option<&[vk::DeviceSize]>,
        sizes: Option<&[vk::DeviceSize]>,
    ) -> VmaResult<()> {
        if self.internal.is_null() {
            return Err(VmaError::NotInitialized());
        }

        let allocations: Vec<ffi::VmaAllocation> =
            allocations.into_iter().map(|a| a.internal).collect();
        ffi::vmaInvalidateAllocations(
            self.internal,
            allocations.len() as u32,
            allocations.as_ptr() as *mut _,
            offsets.map_or(std::ptr::null(), |offsets| offsets.as_ptr()),
            sizes.map_or(std::ptr::null(), |sizes| sizes.as_ptr()),
        )
        .result()
        .map_err(VmaError::from)
    }

    pub fn destroy(&mut self) {
        if self.internal.is_null() {
            return;
        }

        unsafe {
            ffi::vmaDestroyAllocator(self.internal);
            self.internal = std::ptr::null_mut();
        }
    }
}

/// Custom `Drop` implementation to clean up internal allocation instance
impl Drop for Allocator {
    fn drop(&mut self) {
        self.destroy();
    }
}

impl Allocation {
    /// Creates a new Allocation with internal handle
    pub(crate) fn new(
        internal: ffi::VmaAllocation,
        id: AllocationId,
        info: AllocationCreateInfo,
    ) -> Self {
        Self { internal, id, info }
    }
}
