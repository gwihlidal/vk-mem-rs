use crate::ffi::{self};
use ash::vk;
use ash::vk::PhysicalDevice;
use ash::{Device, Instance};
use bitflags::bitflags;
use std::marker::PhantomData;

/// Intended usage of memory.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum MemoryUsage {
    /// No intended memory usage specified.
    /// Use other members of `AllocationCreateInfo` to specify your requirements.
    Unknown,

    /// Memory will be used on device only, so fast access from the device is preferred.
    /// It usually means device-local GPU (video) memory.
    /// No need to be mappable on host.
    /// It is roughly equivalent of `D3D12_HEAP_TYPE_DEFAULT`.
    ///
    /// Usage:
    ///
    /// - Resources written and read by device, e.g. images used as attachments.
    /// - Resources transferred from host once (immutable) or infrequently and read by
    ///   device multiple times, e.g. textures to be sampled, vertex buffers, uniform
    ///   (constant) buffers, and majority of other types of resources used on GPU.
    ///
    /// Allocation may still end up in `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` memory on some implementations.
    /// In such case, you are free to map it.
    /// You can use `AllocationCreateFlags::MAPPED` with this usage type.
    #[deprecated(since = "0.3")]
    GpuOnly,

    /// Memory will be mappable on host.
    /// It usually means CPU (system) memory.
    /// Guarantees to be `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` and `ash::vk::MemoryPropertyFlags::HOST_COHERENT`.
    /// CPU access is typically uncached. Writes may be write-combined.
    /// Resources created in this pool may still be accessible to the device, but access to them can be slow.
    /// It is roughly equivalent of `D3D12_HEAP_TYPE_UPLOAD`.
    ///
    /// Usage: Staging copy of resources used as transfer source.
    #[deprecated(since = "0.3")]
    CpuOnly,

    /// Memory that is both mappable on host (guarantees to be `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`) and preferably fast to access by GPU.
    /// CPU access is typically uncached. Writes may be write-combined.
    ///
    /// Usage: Resources written frequently by host (dynamic), read by device. E.g. textures, vertex buffers,
    /// uniform buffers updated every frame or every draw call.
    #[deprecated(since = "0.3")]
    CpuToGpu,

    /// Memory mappable on host (guarantees to be `ash::vk::MemoryPropertFlags::HOST_VISIBLE`) and cached.
    /// It is roughly equivalent of `D3D12_HEAP_TYPE_READBACK`.
    ///
    /// Usage:
    ///
    /// - Resources written by device, read by host - results of some computations, e.g. screen capture, average scene luminance for HDR tone mapping.
    /// - Any resources read or accessed randomly on host, e.g. CPU-side copy of vertex buffer used as source of transfer, but also used for collision detection.
    #[deprecated(since = "0.3")]
    GpuToCpu,

    /// Prefers not `VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT`.
    #[deprecated(since = "0.3")]
    CpuCopy,

    /// Lazily allocated GPU memory having (guarantees to be `ash::vk::MemoryPropertFlags::LAZILY_ALLOCATED`).
    /// Exists mostly on mobile platforms. Using it on desktop PC or other GPUs with no such memory type present will fail the allocation.
    ///
    /// Usage:
    ///
    /// -  Memory for transient attachment images (color attachments, depth attachments etc.), created with `VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT`.
    /// Allocations with this usage are always created as dedicated - it implies #VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT.
    GpuLazy,

    /// Selects best memory type automatically.
    /// This flag is recommended for most common use cases.
    ///
    /// When using this flag, if you want to map the allocation (using vmaMapMemory() or #VMA_ALLOCATION_CREATE_MAPPED_BIT),
    /// you must pass one of the flags: #VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT or #VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT
    /// in VmaAllocationCreateInfo::flags.
    ///
    /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
    /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
    /// and not with generic memory allocation functions.
    Auto,

    /// Selects best memory type automatically with preference for GPU (device) memory.
    ///
    /// When using this flag, if you want to map the allocation (using vmaMapMemory() or #VMA_ALLOCATION_CREATE_MAPPED_BIT),
    /// you must pass one of the flags: #VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT or #VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT
    /// in VmaAllocationCreateInfo::flags.
    ///
    /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
    /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
    /// and not with generic memory allocation functions.
    AutoPreferDevice,

    /// Selects best memory type automatically with preference for CPU (host) memory.
    ///
    /// When using this flag, if you want to map the allocation (using vmaMapMemory() or #VMA_ALLOCATION_CREATE_MAPPED_BIT),
    /// you must pass one of the flags: #VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT or #VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT
    /// in VmaAllocationCreateInfo::flags.
    ///
    /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
    /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
    /// and not with generic memory allocation functions.
    AutoPreferHost,
}

bitflags! {
    /// Flags for configuring `Allocator` construction.
    #[derive(Clone)]pub struct AllocatorCreateFlags: u32 {
        /// No allocator configuration other than defaults.
        const NONE = 0;

        /// Allocator and all objects created from it will not be synchronized internally,
        /// so you must guarantee they are used from only one thread at a time or synchronized
        /// externally by you. Using this flag may increase performance because internal
        /// mutexes are not used.
        const EXTERNALLY_SYNCHRONIZED = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_EXTERNALLY_SYNCHRONIZED_BIT as u32;

        /// Enables usage of `VK_KHR_dedicated_allocation` extension.
        ///
        /// Using this extenion will automatically allocate dedicated blocks of memory for
        /// some buffers and images instead of suballocating place for them out of bigger
        /// memory blocks (as if you explicitly used `AllocationCreateFlags::DEDICATED_MEMORY` flag) when it is
        /// recommended by the driver. It may improve performance on some GPUs.
        ///
        /// You may set this flag only if you found out that following device extensions are
        /// supported, you enabled them while creating Vulkan device passed as
        /// `AllocatorCreateInfo::device`, and you want them to be used internally by this
        /// library:
        ///
        /// - VK_KHR_get_memory_requirements2
        /// - VK_KHR_dedicated_allocation
        ///
        /// When this flag is set, you can experience following warnings reported by Vulkan
        /// validation layer. You can ignore them.
        /// `> vkBindBufferMemory(): Binding memory to buffer 0x2d but vkGetBufferMemoryRequirements() has not been called on that buffer.`
        const KHR_DEDICATED_ALLOCATION = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_KHR_DEDICATED_ALLOCATION_BIT as u32;

        /// Enables usage of VK_KHR_bind_memory2 extension.
        ///
        /// The flag works only if VmaAllocatorCreateInfo::vulkanApiVersion `== VK_API_VERSION_1_0`.
        /// When it is `VK_API_VERSION_1_1`, the flag is ignored because the extension has been promoted to Vulkan 1.1.
        ///
        /// You may set this flag only if you found out that this device extension is supported,
        /// you enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// and you want it to be used internally by this library.
        ///
        /// The extension provides functions `vkBindBufferMemory2KHR` and `vkBindImageMemory2KHR`,
        /// which allow to pass a chain of `pNext` structures while binding.
        /// This flag is required if you use `pNext` parameter in vmaBindBufferMemory2() or vmaBindImageMemory2().
        const KHR_BIND_MEMORY2 = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT as u32;

        /// Enables usage of VK_EXT_memory_budget extension.
        ///
        /// You may set this flag only if you found out that this device extension is supported,
        /// you enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// and you want it to be used internally by this library, along with another instance extension
        /// VK_KHR_get_physical_device_properties2, which is required by it (or Vulkan 1.1, where this extension is promoted).
        ///
        /// The extension provides query for current memory usage and budget, which will probably
        /// be more accurate than an estimation used by the library otherwise.
        const EXT_MEMORY_BUDGET = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_EXT_MEMORY_BUDGET_BIT as u32;

        /// Enables usage of VK_AMD_device_coherent_memory extension.
        ///
        /// You may set this flag only if you:
        ///
        /// - found out that this device extension is supported and enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// - checked that `VkPhysicalDeviceCoherentMemoryFeaturesAMD::deviceCoherentMemory` is true and set it while creating the Vulkan device,
        /// - want it to be used internally by this library.
        ///
        /// The extension and accompanying device feature provide access to memory types with
        /// `VK_MEMORY_PROPERTY_DEVICE_COHERENT_BIT_AMD` and `VK_MEMORY_PROPERTY_DEVICE_UNCACHED_BIT_AMD` flags.
        /// They are useful mostly for writing breadcrumb markers - a common method for debugging GPU crash/hang/TDR.
        ///
        /// When the extension is not enabled, such memory types are still enumerated, but their usage is illegal.
        /// To protect from this error, if you don't create the allocator with this flag, it will refuse to allocate any memory or create a custom pool in such memory type,
        /// returning `VK_ERROR_FEATURE_NOT_PRESENT`.
        const AMD_DEVICE_COHERENT_MEMORY = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_AMD_DEVICE_COHERENT_MEMORY_BIT as u32;

        /// You may set this flag only if you:
        ///
        /// 1. (For Vulkan version < 1.2) Found as available and enabled device extension
        /// VK_KHR_buffer_device_address.
        /// This extension is promoted to core Vulkan 1.2.
        /// 2. Found as available and enabled device feature `VkPhysicalDeviceBufferDeviceAddressFeatures::bufferDeviceAddress`.
        ///
        /// When this flag is set, you can create buffers with `VK_BUFFER_USAGE_SHADER_DEVICE_ADDRESS_BIT` using VMA.
        /// The library automatically adds `VK_MEMORY_ALLOCATE_DEVICE_ADDRESS_BIT` to
        /// allocated memory blocks wherever it might be needed.
        ///
        /// For more information, see documentation chapter \ref enabling_buffer_device_address.
        const BUFFER_DEVICE_ADDRESS = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_BUFFER_DEVICE_ADDRESS_BIT as u32;

        /// Enables usage of VK_EXT_memory_priority extension in the library.
        ///
        /// You may set this flag only if you found available and enabled this device extension,
        /// along with `VkPhysicalDeviceMemoryPriorityFeaturesEXT::memoryPriority == VK_TRUE`,
        /// while creating Vulkan device passed as VmaAllocatorCreateInfo::device.
        ///
        /// When this flag is used, VmaAllocationCreateInfo::priority and VmaPoolCreateInfo::priority
        /// are used to set priorities of allocated Vulkan memory. Without it, these variables are ignored.
        ///
        /// A priority must be a floating-point value between 0 and 1, indicating the priority of the allocation relative to other memory allocations.
        /// Larger values are higher priority. The granularity of the priorities is implementation-dependent.
        /// It is automatically passed to every call to `vkAllocateMemory` done by the library using structure `VkMemoryPriorityAllocateInfoEXT`.
        /// The value to be used for default priority is 0.5.
        /// For more details, see the documentation of the VK_EXT_memory_priority extension.
        const EXT_MEMORY_PRIORITY = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT as u32;

        /// Enables usage of VK_KHR_maintenance4 extension in the library.
        ///
        /// You may set this flag only if you found available and enabled this device extension,
        /// while creating Vulkan device passed as VmaAllocatorCreateInfo::device.
        const KHR_MAINTENANCE4 = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_KHR_MAINTENANCE4_BIT as u32;
        /// Enables usage of VK_KHR_maintenance5 extension in the library.
        ///
        /// You may set this flag only if you found available and enabled this device extension,
        /// while creating Vulkan device passed as VmaAllocatorCreateInfo::device.
        const KHR_MAINTENANCE5 = ffi::VmaAllocatorCreateFlagBits::VMA_ALLOCATOR_CREATE_KHR_MAINTENANCE5_BIT as u32;
    }
}

bitflags! {
    /// Flags for configuring `Allocation` construction.
    #[derive(Debug, Clone, Copy)]
    pub struct AllocationCreateFlags: u32 {
        /// Set this flag if the allocation should have its own memory block.
        ///
        /// Use it for special, big resources, like fullscreen images used as attachments.
        const DEDICATED_MEMORY = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT as u32;

        /// Set this flag to only try to allocate from existing `ash::vk::DeviceMemory` blocks and never create new such block.
        ///
        /// If new allocation cannot be placed in any of the existing blocks, allocation
        /// fails with `ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY` error.
        ///
        /// You should not use `AllocationCreateFlags::DEDICATED_MEMORY` and `AllocationCreateFlags::NEVER_ALLOCATE` at the same time. It makes no sense.
        const NEVER_ALLOCATE = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_NEVER_ALLOCATE_BIT as u32;

        /// Set this flag to use a memory that will be persistently mapped and retrieve pointer to it.
        ///
        /// Pointer to mapped memory will be returned through `Allocation::get_mapped_data()`.
        ///
        /// Is it valid to use this flag for allocation made from memory type that is not
        /// `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`. This flag is then ignored and memory is not mapped. This is
        /// useful if you need an allocation that is efficient to use on GPU
        /// (`ash::vk::MemoryPropertyFlags::DEVICE_LOCAL`) and still want to map it directly if possible on platforms that
        /// support it (e.g. Intel GPU).
        ///
        /// You should not use this flag together with `AllocationCreateFlags::CAN_BECOME_LOST`.
        const MAPPED = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_MAPPED_BIT as u32;

        /// Set this flag to treat `AllocationCreateInfo::user_data` as pointer to a
        /// null-terminated string. Instead of copying pointer value, a local copy of the
        /// string is made and stored in allocation's user data. The string is automatically
        /// freed together with the allocation. It is also used in `Allocator::build_stats_string`.
        #[deprecated(since = "0.3", note = "Consider using vmaSetAllocationName() instead.")]
        const USER_DATA_COPY_STRING = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_USER_DATA_COPY_STRING_BIT as u32;

        /// Allocation will be created from upper stack in a double stack pool.
        ///
        /// This flag is only allowed for custom pools created with `AllocatorPoolCreateFlags::LINEAR_ALGORITHM` flag.
        const UPPER_ADDRESS = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_UPPER_ADDRESS_BIT as u32;

        /// Create both buffer/image and allocation, but don't bind them together.
        /// It is useful when you want to bind yourself to do some more advanced binding, e.g. using some extensions.
        /// The flag is meaningful only with functions that bind by default, such as `Allocator::create_buffer`
        /// or `Allocator::create_image`. Otherwise it is ignored.
        ///
        /// If you want to make sure the new buffer/image is not tied to the new memory allocation
        /// through `VkMemoryDedicatedAllocateInfoKHR` structure in case the allocation ends up in its own memory block,
        /// use also flag #VMA_ALLOCATION_CREATE_CAN_ALIAS_BIT.
        const DONT_BIND = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_DONT_BIND_BIT as u32;

        /// Create allocation only if additional device memory required for it, if any, won't exceed
        /// memory budget. Otherwise return `VK_ERROR_OUT_OF_DEVICE_MEMORY`.
        const WITHIN_BUDGET = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_WITHIN_BUDGET_BIT as u32;

        /// Set this flag if the allocated memory will have aliasing resources.
        ///
        /// Usage of this flag prevents supplying `VkMemoryDedicatedAllocateInfoKHR` when #VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT is specified.
        /// Otherwise created dedicated memory will not be suitable for aliasing resources, resulting in Vulkan Validation Layer errors.
        const CAN_ALIAS = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_CAN_ALIAS_BIT as u32;

        /// Requests possibility to map the allocation (using vmaMapMemory() or #VMA_ALLOCATION_CREATE_MAPPED_BIT).
        ///
        /// - If you use #VMA_MEMORY_USAGE_AUTO or other `VMA_MEMORY_USAGE_AUTO*` value,
        /// you must use this flag to be able to map the allocation. Otherwise, mapping is incorrect.
        /// - If you use other value of #VmaMemoryUsage, this flag is ignored and mapping is always possible in memory types that are `HOST_VISIBLE`.
        /// This includes allocations created in custom_memory_pools.
        ///
        /// Declares that mapped memory will only be written sequentially, e.g. using `memcpy()` or a loop writing number-by-number,
        /// never read or accessed randomly, so a memory type can be selected that is uncached and write-combined.
        ///
        /// Violating this declaration may work correctly, but will likely be very slow.
        /// Watch out for implicit reads introduced by doing e.g. `pMappedData[i] += x;`
        /// Better prepare your data in a local variable and `memcpy()` it to the mapped pointer all at once.
        const HOST_ACCESS_SEQUENTIAL_WRITE = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT as u32;

        /// Requests possibility to map the allocation (using vmaMapMemory() or #VMA_ALLOCATION_CREATE_MAPPED_BIT).
        ///
        /// - If you use #VMA_MEMORY_USAGE_AUTO or other `VMA_MEMORY_USAGE_AUTO*` value,
        /// you must use this flag to be able to map the allocation. Otherwise, mapping is incorrect.
        /// - If you use other value of #VmaMemoryUsage, this flag is ignored and mapping is always possible in memory types that are `HOST_VISIBLE`.
        /// This includes allocations created in custom_memory_pools.
        ///
        /// Declares that mapped memory can be read, written, and accessed in random order,
        /// so a `HOST_CACHED` memory type is required.
        const HOST_ACCESS_RANDOM = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT as u32;

        /// Together with #VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT or #VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT,
        /// it says that despite request for host access, a not-`HOST_VISIBLE` memory type can be selected
        /// if it may improve performance.
        ///
        /// By using this flag, you declare that you will check if the allocation ended up in a `HOST_VISIBLE` memory type
        /// (e.g. using vmaGetAllocationMemoryProperties()) and if not, you will create some "staging" buffer and
        /// issue an explicit transfer to write/read your data.
        /// To prepare for this possibility, don't forget to add appropriate flags like
        /// `VK_BUFFER_USAGE_TRANSFER_DST_BIT`, `VK_BUFFER_USAGE_TRANSFER_SRC_BIT` to the parameters of created buffer or image.
        const HOST_ACCESS_ALLOW_TRANSFER_INSTEAD = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_HOST_ACCESS_ALLOW_TRANSFER_INSTEAD_BIT as u32;

        /// Allocation strategy that chooses smallest possible free range for the allocation
        /// to minimize memory usage and fragmentation, possibly at the expense of allocation time.
        const STRATEGY_MIN_MEMORY = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_STRATEGY_MIN_MEMORY_BIT as u32;

        /// Alias to `STRATEGY_MIN_MEMORY`.
        const STRATEGY_BEST_FIT = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_STRATEGY_MIN_MEMORY_BIT as u32;

        /// Allocation strategy that chooses first suitable free range for the allocation -
        /// not necessarily in terms of the smallest offset but the one that is easiest and fastest to find
        /// to minimize allocation time, possibly at the expense of allocation quality.
        const STRATEGY_MIN_TIME = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_STRATEGY_MIN_TIME_BIT as u32;

        /// Alias to `STRATEGY_MIN_TIME`.
        const STRATEGY_FIRST_FIT = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_STRATEGY_MIN_TIME_BIT as u32;

        /// Allocation strategy that chooses always the lowest offset in available space.
        /// This is not the most efficient strategy but achieves highly packed data.
        /// Used internally by defragmentation, not recomended in typical usage.
        const STRATEGY_MIN_OFFSET = ffi::VmaAllocationCreateFlagBits::VMA_ALLOCATION_CREATE_STRATEGY_MIN_OFFSET_BIT as u32;
    }
}

bitflags! {
    /// Flags for configuring `AllocatorPool` construction.
    #[derive(Default, Clone, Copy)]
    pub struct AllocatorPoolCreateFlags: u32 {
        /// Use this flag if you always allocate only buffers and linear images or only optimal images
        /// out of this pool and so buffer-image granularity can be ignored.
        ///
        /// This is an optional optimization flag.
        ///
        /// If you always allocate using `Allocator::create_buffer`, `Allocator::create_image`,
        /// `Allocator::allocate_memory_for_buffer`, then you don't need to use it because allocator
        /// knows exact type of your allocations so it can handle buffer-image granularity
        /// in the optimal way.
        ///
        /// If you also allocate using `Allocator::allocate_memory_for_image` or `Allocator::allocate_memory`,
        /// exact type of such allocations is not known, so allocator must be conservative
        /// in handling buffer-image granularity, which can lead to suboptimal allocation
        /// (wasted memory). In that case, if you can make sure you always allocate only
        /// buffers and linear images or only optimal images out of this pool, use this flag
        /// to make allocator disregard buffer-image granularity and so make allocations
        /// faster and more optimal.
        const IGNORE_BUFFER_IMAGE_GRANULARITY = ffi::VmaPoolCreateFlagBits::VMA_POOL_CREATE_IGNORE_BUFFER_IMAGE_GRANULARITY_BIT as u32;

        /// Enables alternative, linear allocation algorithm in this pool.
        ///
        /// Specify this flag to enable linear allocation algorithm, which always creates
        /// new allocations after last one and doesn't reuse space from allocations freed in
        /// between. It trades memory consumption for simplified algorithm and data
        /// structure, which has better performance and uses less memory for metadata.
        ///
        /// By using this flag, you can achieve behavior of free-at-once, stack,
        /// ring buffer, and double stack.
        ///
        /// When using this flag, you must specify PoolCreateInfo::max_block_count == 1 (or 0 for default).
        const LINEAR_ALGORITHM = ffi::VmaPoolCreateFlagBits::VMA_POOL_CREATE_LINEAR_ALGORITHM_BIT as u32;

        /// Bit mask to extract only `*_ALGORITHM` bits from entire set of flags.
        const ALGORITHM_MASK = ffi::VmaPoolCreateFlagBits::VMA_POOL_CREATE_ALGORITHM_MASK as u32;
    }
}

#[derive(Clone)]
pub struct AllocatorCreateInfo<'a> {
    /// Vulkan physical device. It must be valid throughout whole lifetime of created allocator.
    pub(crate) physical_device: PhysicalDevice,

    /// Vulkan device.
    /// It must be valid throughout the whole lifetime of created allocator.
    pub(crate) device: &'a Device,

    /// Handle to Vulkan instance object.
    /// Must be valid throughout the whole lifetime of created allocator.
    pub(crate) instance: &'a Instance,

    /// Flags for created allocator.
    pub flags: AllocatorCreateFlags,

    /// Preferred size of a single [`vk::DeviceMemory`] block to be allocated from large heaps > 1 GiB. Optional.
    /// Set to 0 to use default, which is currently 256 MiB.
    pub preferred_large_heap_block_size: vk::DeviceSize,

    /// Custom CPU memory allocation callbacks. Optional.
    /// When specified, will also be used for all CPU-side memory allocations.
    pub allocation_callbacks: Option<&'a vk::AllocationCallbacks<'a>>,

    /// Informative callbacks for [`vk::AllocateMemory`], [`vk::FreeMemory`]. Optional.
    pub device_memory_callbacks: Option<&'a ffi::VmaDeviceMemoryCallbacks>,

    /// An empty array, or an array of limits on maximum number of bytes that can be allocated out of particular Vulkan memory heap.
    /// When it is not empty, it must be an array of [`vk::PhysicalDeviceMemoryProperties::memoryHeapCount`] elements, defining limit on
    /// maximum number of bytes that can be allocated out of particular Vulkan memory
    /// heap.
    ///
    /// Any of the elements may be equal to `VK_WHOLE_SIZE`, which means no limit on that
    /// heap. This is also the default in case of `pHeapSizeLimit` = NULL.
    ///
    /// If there is a limit defined for a heap:
    /// - If user tries to allocate more memory from that heap using this allocator,
    ///   the allocation fails with `VK_ERROR_OUT_OF_DEVICE_MEMORY`.
    /// - If the limit is smaller than heap size reported in `VkMemoryHeap::size`, the
    ///   value of this limit will be reported instead when using vmaGetMemoryProperties().
    ///
    /// Warning! Using this feature may not be equivalent to installing a GPU with
    /// smaller amount of memory, because graphics driver doesn't necessary fail new
    /// allocations with [`VK_ERROR_OUT_OF_DEVICE_MEMORY`] result when memory capacity is
    /// exceeded. It may return success and just silently migrate some device memory
    /// blocks to system RAM. This driver behavior can also be controlled using
    /// VK_AMD_memory_overallocation_behavior extension.
    pub heap_size_limits: &'a [ash::vk::DeviceSize],
    /// Optional. Vulkan version that the application uses.
    /// It must be a value in the format as created by macro `VK_MAKE_VERSION` or a constant like:
    /// `VK_API_VERSION_1_1`, `VK_API_VERSION_1_0`.
    /// The patch version number specified is ignored. Only the major and minor versions are considered.
    /// Only versions 1.0, 1.1, 1.2, 1.3 are supported by the current implementation.
    /// Leaving it initialized to zero is equivalent to `VK_API_VERSION_1_0`.
    /// It must match the Vulkan version used by the application and supported on the selected physical device,
    /// so it must be no higher than `VkApplicationInfo::apiVersion` passed to `vkCreateInstance`
    /// and no higher than `VkPhysicalDeviceProperties::apiVersion` found on the physical device used.
    pub vulkan_api_version: u32,
    /// Either an empty array or an array of external memory handle types for each Vulkan memory type.
    /// If not empty, it must be a pointer to an array of `VkPhysicalDeviceMemoryProperties::memoryTypeCount`
    /// elements, defining external memory handle types of particular Vulkan memory type,
    /// to be passed using `VkExportMemoryAllocateInfoKHR`.
    ///
    /// Any of the elements may be equal to 0, which means not to use `VkExportMemoryAllocateInfoKHR` on this memory type.
    /// This is also the default in case of `pTypeExternalMemoryHandleTypes` = NULL.
    pub type_external_memory_handle_types: &'a [vk::ExternalMemoryHandleTypeFlagsKHR],
}

impl<'a> AllocatorCreateInfo<'a> {
    pub fn new(
        instance: &'a ash::Instance,
        device: &'a ash::Device,
        physical_device: ash::vk::PhysicalDevice,
    ) -> AllocatorCreateInfo<'a> {
        AllocatorCreateInfo {
            physical_device,
            device,
            instance,
            flags: AllocatorCreateFlags::empty(),
            preferred_large_heap_block_size: 0,
            allocation_callbacks: None,
            device_memory_callbacks: None,
            heap_size_limits: &[],
            vulkan_api_version: 0,
            type_external_memory_handle_types: &[],
        }
    }
}

#[derive(Clone)]
pub struct PoolCreateInfo<'a> {
    ///  Vulkan memory type index to allocate this pool from.
    pub memory_type_index: u32,
    pub flags: AllocatorPoolCreateFlags,
    /// Size of a single [`vk::DeviceMemory`] block to be allocated as part of this pool, in bytes. Optional.
    /// Specify nonzero to set explicit, constant size of memory blocks used by this pool.
    /// Leave 0 to use default and let the library manage block sizes automatically.
    /// Sizes of particular blocks may vary.
    /// In this case, the pool will also support dedicated allocations.
    pub block_size: vk::DeviceSize,
    /// Minimum number of blocks to be always allocated in this pool, even if they stay empty.
    /// Set to 0 to have no preallocated blocks and allow the pool be completely empty.
    pub min_block_count: usize,
    /// Maximum number of blocks that can be allocated in this pool. Optional.
    /// Set to 0 to use default, which is [`usize::MAX`], which means no limit.
    /// Set to same value as VmaPoolCreateInfo::minBlockCount to have fixed amount of memory allocated
    /// throughout whole lifetime of this pool.
    pub max_block_count: usize,
    /// A floating-point value between 0 and 1, indicating the priority of the allocations in this pool relative to other memory allocations.
    /// It is used only when #VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT flag was used during creation of the #VmaAllocator object.
    /// Otherwise, this variable is ignored.
    pub priority: f32,
    /// Additional minimum alignment to be used for all allocations created from this pool. Can be 0.
    /// Leave 0 (default) not to impose any additional alignment. If not 0, it must be a power of two.
    /// It can be useful in cases where alignment returned by Vulkan by functions like `vkGetBufferMemoryRequirements` is not enough,
    /// e.g. when doing interop with OpenGL.
    pub min_allocation_alignment: vk::DeviceSize,
    /// Additional `pNext` chain to be attached to `VkMemoryAllocateInfo` used for every allocation made by this pool. Optional.
    /// If not null, it must point to a `pNext` chain of structures that can be attached to `VkMemoryAllocateInfo`.
    /// It can be useful for special needs such as adding `VkExportMemoryAllocateInfoKHR`.
    /// Structures pointed by this member must remain alive and unchanged for the whole lifetime of the custom pool.
    /// Please note that some structures, e.g. `VkMemoryPriorityAllocateInfoEXT`, `VkMemoryDedicatedAllocateInfoKHR`,
    /// can be attached automatically by this library when using other, more convenient of its features.
    pub memory_allocate_next: *const std::ffi::c_void,
    pub _marker: PhantomData<&'a mut ()>,
}
impl<'a> PoolCreateInfo<'a> {
    pub fn push_next<T: vk::ExtendsMemoryAllocateInfo>(&mut self, next: &'a mut T) {
        let info = vk::MemoryAllocateInfo {
            p_next: self.memory_allocate_next,
            ..Default::default()
        };
        let info = info.push_next(next);
        self.memory_allocate_next = info.p_next;
    }
}
impl Default for PoolCreateInfo<'_> {
    fn default() -> Self {
        Self {
            memory_type_index: 0,
            flags: AllocatorPoolCreateFlags::empty(),
            block_size: 0,
            min_block_count: 0,
            max_block_count: 0,
            priority: 0.0,
            min_allocation_alignment: 0,
            memory_allocate_next: std::ptr::null_mut(),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub struct AllocationCreateInfo {
    pub flags: AllocationCreateFlags,
    /// Intended usage of memory.
    ///
    /// You can leave `MemoryUsage::Unknown` if you specify memory requirements in other way.
    ///
    /// If `pool` is not null, this member is ignored.
    pub usage: MemoryUsage,
    /// Flags that must be set in a Memory Type chosen for an allocation.
    ///
    /// Leave 0 if you specify memory requirements in other way.
    ///
    /// If `pool` is not null, this member is ignored.
    pub required_flags: vk::MemoryPropertyFlags,
    /// Flags that preferably should be set in a memory type chosen for an allocation."]
    ///
    /// Set to 0 if no additional flags are preferred.
    /// If `pool` is not null, this member is ignored.
    pub preferred_flags: vk::MemoryPropertyFlags,
    /// Bitmask containing one bit set for every memory type acceptable for this allocation.
    ///
    /// Value 0 is equivalent to `UINT32_MAX` - it means any memory type is accepted if
    /// it meets other requirements specified by this structure, with no further
    /// restrictions on memory type index.
    ///
    /// If `pool` is not null, this member is ignored.
    pub memory_type_bits: u32,
    /// Custom general-purpose pointer that will be stored in `Allocation`,
    /// can be read as VmaAllocationInfo::pUserData and changed using vmaSetAllocationUserData().
    ///
    /// If #VMA_ALLOCATION_CREATE_USER_DATA_COPY_STRING_BIT is used, it must be either
    /// null or pointer to a null-terminated string. The string will be then copied to
    /// internal buffer, so it doesn't need to be valid after allocation call.
    pub user_data: usize,
    /// A floating-point value between 0 and 1, indicating the priority of the allocation relative to other memory allocations.
    ///
    /// It is used only when #VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT flag was used during creation of the #VmaAllocator object
    /// and this allocation ends up as dedicated or is explicitly forced as dedicated using #VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT.
    /// Otherwise, it has the priority of a memory block where it is placed and this variable is ignored.
    pub priority: f32,
}

impl Default for AllocationCreateInfo {
    fn default() -> Self {
        Self {
            flags: AllocationCreateFlags::empty(),
            usage: MemoryUsage::Unknown,
            required_flags: vk::MemoryPropertyFlags::empty(),
            preferred_flags: vk::MemoryPropertyFlags::empty(),
            memory_type_bits: 0,
            user_data: 0,
            priority: 0.0,
        }
    }
}

impl From<&AllocationCreateInfo> for ffi::VmaAllocationCreateInfo {
    fn from(info: &AllocationCreateInfo) -> Self {
        let usage = match info.usage {
            MemoryUsage::Unknown => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_UNKNOWN,
            #[allow(deprecated)]
            MemoryUsage::GpuOnly => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_GPU_ONLY,
            #[allow(deprecated)]
            MemoryUsage::CpuOnly => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_CPU_ONLY,
            #[allow(deprecated)]
            MemoryUsage::CpuToGpu => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_CPU_TO_GPU,
            #[allow(deprecated)]
            MemoryUsage::GpuToCpu => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_GPU_TO_CPU,
            #[allow(deprecated)]
            MemoryUsage::CpuCopy => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_CPU_COPY,
            MemoryUsage::GpuLazy => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_GPU_LAZILY_ALLOCATED,
            MemoryUsage::Auto => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_AUTO,
            MemoryUsage::AutoPreferDevice => {
                ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_AUTO_PREFER_DEVICE
            }
            MemoryUsage::AutoPreferHost => ffi::VmaMemoryUsage::VMA_MEMORY_USAGE_AUTO_PREFER_HOST,
        };
        ffi::VmaAllocationCreateInfo {
            flags: info.flags.bits(),
            usage,
            requiredFlags: info.required_flags,
            preferredFlags: info.preferred_flags,
            memoryTypeBits: info.memory_type_bits,
            pool: std::ptr::null_mut(),
            pUserData: info.user_data as _,
            priority: info.priority,
        }
    }
}

/// Parameters of `Allocation` objects, that can be retrieved using `Allocator::get_allocation_info`.
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Memory type index that this allocation was allocated from. It never changes.
    pub memory_type: u32,
    /// Handle to Vulkan memory object.
    ///
    /// Same memory object can be shared by multiple allocations.
    ///
    /// It can change after the allocation is moved during \\ref defragmentation.
    pub device_memory: vk::DeviceMemory,
    /// Offset in `VkDeviceMemory` object to the beginning of this allocation, in bytes. `(deviceMemory, offset)` pair is unique to this allocation.
    ///
    /// You usually don't need to use this offset. If you create a buffer or an image together with the allocation using e.g. function
    /// vmaCreateBuffer(), vmaCreateImage(), functions that operate on these resources refer to the beginning of the buffer or image,
    /// not entire device memory block. Functions like vmaMapMemory(), vmaBindBufferMemory() also refer to the beginning of the allocation
    /// and apply this offset automatically.
    ///
    /// It can change after the allocation is moved during \\ref defragmentation.
    pub offset: vk::DeviceSize,
    /// Size of this allocation, in bytes. It never changes.
    ///
    /// Allocation size returned in this variable may be greater than the size
    /// requested for the resource e.g. as `VkBufferCreateInfo::size`. Whole size of the
    /// allocation is accessible for operations on memory e.g. using a pointer after
    /// mapping with vmaMapMemory(), but operations on the resource e.g. using
    /// `vkCmdCopyBuffer` must be limited to the size of the resource.
    pub size: vk::DeviceSize,
    /// Pointer to the beginning of this allocation as mapped data.
    ///
    /// If the allocation hasn't been mapped using vmaMapMemory() and hasn't been
    /// created with #VMA_ALLOCATION_CREATE_MAPPED_BIT flag, this value is null.
    ///
    /// It can change after call to vmaMapMemory(), vmaUnmapMemory().
    /// It can also change after the allocation is moved during defragmentation.
    pub mapped_data: *mut ::std::os::raw::c_void,
    /// Custom general-purpose pointer that was passed as VmaAllocationCreateInfo::pUserData or set using vmaSetAllocationUserData().
    ///
    /// It can change after call to vmaSetAllocationUserData() for this allocation.
    pub user_data: usize,
}

impl From<&ffi::VmaAllocationInfo> for AllocationInfo {
    fn from(info: &ffi::VmaAllocationInfo) -> Self {
        Self {
            memory_type: info.memoryType,
            device_memory: info.deviceMemory,
            offset: info.offset,
            size: info.size,
            mapped_data: info.pMappedData,
            user_data: info.pUserData as _,
        }
    }
}
impl From<ffi::VmaAllocationInfo> for AllocationInfo {
    fn from(info: ffi::VmaAllocationInfo) -> Self {
        (&info).into()
    }
}

bitflags! {
    /// Flags for configuring `VirtualBlock` construction
    #[derive(Default)]
    pub struct VirtualBlockCreateFlags: u32 {
        /// Enables alternative, linear allocation algorithm in this pool.
        ///
        /// Specify this flag to enable linear allocation algorithm, which always creates
        /// new allocations after last one and doesn't reuse space from allocations freed in
        /// between. It trades memory consumption for simplified algorithm and data
        /// structure, which has better performance and uses less memory for metadata.
        ///
        /// By using this flag, you can achieve behavior of free-at-once, stack,
        /// ring buffer, and double stack.
        const VMA_VIRTUAL_BLOCK_CREATE_LINEAR_ALGORITHM_BIT = ffi::VmaVirtualBlockCreateFlagBits::VMA_VIRTUAL_BLOCK_CREATE_LINEAR_ALGORITHM_BIT as u32;
    }
}

bitflags! {
    /// Flags for configuring `VirtualBlock` construction
    #[derive(Clone, Copy, Debug)]
    pub struct VirtualAllocationCreateFlags: u32 {
        /// Allocation will be created from upper stack in a double stack pool.
        ///
        /// This flag is only allowed for virtual blocks created with #VMA_VIRTUAL_BLOCK_CREATE_LINEAR_ALGORITHM_BIT flag.
        const VMA_VIRTUAL_ALLOCATION_CREATE_UPPER_ADDRESS_BIT = ffi::VmaVirtualAllocationCreateFlagBits::VMA_VIRTUAL_ALLOCATION_CREATE_UPPER_ADDRESS_BIT as u32;
        /// Allocation strategy that tries to minimize memory usage.
        const VMA_VIRTUAL_ALLOCATION_CREATE_STRATEGY_MIN_MEMORY_BIT = ffi::VmaVirtualAllocationCreateFlagBits::VMA_VIRTUAL_ALLOCATION_CREATE_STRATEGY_MIN_MEMORY_BIT as u32;
        /// Allocation strategy that tries to minimize allocation time.
        const VMA_VIRTUAL_ALLOCATION_CREATE_STRATEGY_MIN_TIME_BIT = ffi::VmaVirtualAllocationCreateFlagBits::VMA_VIRTUAL_ALLOCATION_CREATE_STRATEGY_MIN_TIME_BIT as u32;
        /// Allocation strategy that chooses always the lowest offset in available space.
        /// This is not the most efficient strategy but achieves highly packed data.
        const VMA_VIRTUAL_ALLOCATION_CREATE_STRATEGY_MIN_OFFSET_BIT = ffi::VmaVirtualAllocationCreateFlagBits::VMA_VIRTUAL_ALLOCATION_CREATE_STRATEGY_MIN_OFFSET_BIT as u32;
        /// A bit mask to extract only `STRATEGY` bits from entire set of flags.
        ///
        /// These strategy flags are binary compatible with equivalent flags in #VmaAllocationCreateFlagBits.
        const VMA_VIRTUAL_ALLOCATION_CREATE_STRATEGY_MASK = ffi::VmaVirtualAllocationCreateFlagBits::VMA_VIRTUAL_ALLOCATION_CREATE_STRATEGY_MASK as u32;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VirtualAllocationCreateInfo {
    /// Size of the allocation.
    ///
    /// Cannot be zero.
    pub size: u64,
    /// Required alignment of the allocation. Optional.
    ///
    /// Must be power of two. Special value 0 has the same meaning as 1 - means no special alignment is required, so allocation can start at any offset.
    pub alignment: u64,
    /// Custom pointer to be associated with the allocation. Optional.
    ///
    /// It can be any value and can be used for user-defined purposes. It can be fetched or changed later.
    pub user_data: usize,
    /// Flags to configure allocation behavior for this allocation
    pub flags: VirtualAllocationCreateFlags,
}

/// Parameters of created VirtualBlock, to be passed to VirtualBlock::new()
#[derive(Default)]
pub struct VirtualBlockCreateInfo<'a> {
    /// Total size of the virtual block.
    ///
    /// Sizes can be expressed in bytes or any units you want as long as you are consistent in using them.
    /// For example, if you allocate from some array of structures, 1 can mean single instance of entire structure.
    pub size: vk::DeviceSize,

    pub flags: VirtualBlockCreateFlags,
    /// Custom CPU memory allocation callbacks. Optional.
    /// When specified, they will be used for all CPU-side memory allocations.
    pub allocation_callbacks: Option<&'a vk::AllocationCallbacks<'a>>,
}

/// Parameters of `VirtualAllocation` objects, that can be retrieved using `VirtualBlock::get_allocation_info`.
#[derive(Debug, Clone, Copy)]
pub struct VirtualAllocationInfo {
    /// Offset of the allocation.
    ///
    /// Offset at which the allocation was made.
    pub offset: vk::DeviceSize,
    /// Size of the allocation.
    ///
    /// Same value as passed in VirtualAllocationCreateInfo::size.
    pub size: vk::DeviceSize,
    /// Custom pointer associated with the allocation
    ///
    /// It can change after call to vmaSetAllocationUserData() for this allocation.
    pub user_data: usize,
}

impl From<&ffi::VmaVirtualAllocationInfo> for VirtualAllocationInfo {
    fn from(info: &ffi::VmaVirtualAllocationInfo) -> Self {
        Self {
            offset: info.offset,
            size: info.size,
            user_data: info.pUserData as _,
        }
    }
}
impl From<ffi::VmaVirtualAllocationInfo> for VirtualAllocationInfo {
    fn from(info: ffi::VmaVirtualAllocationInfo) -> Self {
        (&info).into()
    }
}

impl From<&VirtualAllocationCreateInfo> for ffi::VmaVirtualAllocationCreateInfo {
    fn from(info: &VirtualAllocationCreateInfo) -> Self {
        ffi::VmaVirtualAllocationCreateInfo {
            size: info.size,
            alignment: info.alignment,
            flags: info.flags.bits(),
            pUserData: info.user_data as _,
        }
    }
}

impl From<VirtualAllocationCreateInfo> for ffi::VmaVirtualAllocationCreateInfo {
    fn from(info: VirtualAllocationCreateInfo) -> Self {
        (&info).into()
    }
}
