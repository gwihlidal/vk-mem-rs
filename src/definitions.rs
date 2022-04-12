use crate::ffi::VmaAllocationCreateInfo;
use crate::{ffi, AllocatorPool};
use ash::vk::PhysicalDevice;
use ash::{Device, Instance};
use bitflags::bitflags;
use std::ptr;

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
    GpuOnly,

    /// Memory will be mappable on host.
    /// It usually means CPU (system) memory.
    /// Guarantees to be `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` and `ash::vk::MemoryPropertyFlags::HOST_COHERENT`.
    /// CPU access is typically uncached. Writes may be write-combined.
    /// Resources created in this pool may still be accessible to the device, but access to them can be slow.
    /// It is roughly equivalent of `D3D12_HEAP_TYPE_UPLOAD`.
    ///
    /// Usage: Staging copy of resources used as transfer source.
    CpuOnly,

    /// Memory that is both mappable on host (guarantees to be `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`) and preferably fast to access by GPU.
    /// CPU access is typically uncached. Writes may be write-combined.
    ///
    /// Usage: Resources written frequently by host (dynamic), read by device. E.g. textures, vertex buffers,
    /// uniform buffers updated every frame or every draw call.
    CpuToGpu,

    /// Memory mappable on host (guarantees to be `ash::vk::MemoryPropertFlags::HOST_VISIBLE`) and cached.
    /// It is roughly equivalent of `D3D12_HEAP_TYPE_READBACK`.
    ///
    /// Usage:
    ///
    /// - Resources written by device, read by host - results of some computations, e.g. screen capture, average scene luminance for HDR tone mapping.
    /// - Any resources read or accessed randomly on host, e.g. CPU-side copy of vertex buffer used as source of transfer, but also used for collision detection.
    GpuToCpu,

    /// Lazily allocated GPU memory having (guarantees to be `ash::vk::MemoryPropertFlags::LAZILY_ALLOCATED`).
    /// Exists mostly on mobile platforms. Using it on desktop PC or other GPUs with no such memory type present will fail the allocation.
    ///
    /// Usage:
    ///
    /// -  Memory for transient attachment images (color attachments, depth attachments etc.), created with `VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT`.
    GpuLazy,
}

bitflags! {
    /// Flags for configuring `Allocator` construction.
    pub struct AllocatorCreateFlags: u32 {
        /// No allocator configuration other than defaults.
        const NONE = 0;

        /// Allocator and all objects created from it will not be synchronized internally,
        /// so you must guarantee they are used from only one thread at a time or synchronized
        /// externally by you. Using this flag may increase performance because internal
        /// mutexes are not used.
        const EXTERNALLY_SYNCHRONIZED = ffi::VmaAllocatorCreateFlagBits_VMA_ALLOCATOR_CREATE_EXTERNALLY_SYNCHRONIZED_BIT;

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
        const KHR_DEDICATED_ALLOCATION = ffi::VmaAllocatorCreateFlagBits_VMA_ALLOCATOR_CREATE_KHR_DEDICATED_ALLOCATION_BIT;

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
        const KHR_BIND_MEMORY2 = ffi::VmaAllocatorCreateFlagBits_VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT;

        /// Enables usage of VK_EXT_memory_budget extension.
        ///
        /// You may set this flag only if you found out that this device extension is supported,
        /// you enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// and you want it to be used internally by this library, along with another instance extension
        /// VK_KHR_get_physical_device_properties2, which is required by it (or Vulkan 1.1, where this extension is promoted).
        ///
        /// The extension provides query for current memory usage and budget, which will probably
        /// be more accurate than an estimation used by the library otherwise.
        const EXT_MEMORY_BUDGET = ffi::VmaAllocatorCreateFlagBits_VMA_ALLOCATOR_CREATE_EXT_MEMORY_BUDGET_BIT;

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
        const AMD_DEVICE_COHERENT_MEMORY = ffi::VmaAllocatorCreateFlagBits_VMA_ALLOCATOR_CREATE_AMD_DEVICE_COHERENT_MEMORY_BIT;

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
        const BUFFER_DEVICE_ADDRESS = ffi::VmaAllocatorCreateFlagBits_VMA_ALLOCATOR_CREATE_BUFFER_DEVICE_ADDRESS_BIT;

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
        const EXT_MEMORY_PRIORITY = ffi::VmaAllocatorCreateFlagBits_VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT;

    }
}

bitflags! {
    /// Flags for configuring `Allocation` construction.
    pub struct AllocationCreateFlags: u32 {
        /// Default configuration for allocation.
        const NONE = 0x0000_0000;

        /// Set this flag if the allocation should have its own memory block.
        ///
        /// Use it for special, big resources, like fullscreen images used as attachments.
        ///
        /// You should not use this flag if `AllocationCreateInfo::pool` is not `None`.
        const DEDICATED_MEMORY = 0x0000_0001;

        /// Set this flag to only try to allocate from existing `ash::vk::DeviceMemory` blocks and never create new such block.
        ///
        /// If new allocation cannot be placed in any of the existing blocks, allocation
        /// fails with `ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY` error.
        ///
        /// You should not use `AllocationCreateFlags::DEDICATED_MEMORY` and `AllocationCreateFlags::NEVER_ALLOCATE` at the same time. It makes no sense.
        ///
        /// If `AllocationCreateInfo::pool` is not `None`, this flag is implied and ignored.
        const NEVER_ALLOCATE = 0x0000_0002;

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
        const MAPPED = 0x0000_0004;

        /// Allocation created with this flag can become lost as a result of another
        /// allocation with `AllocationCreateFlags::CAN_MAKE_OTHER_LOST` flag, so you must check it before use.
        ///
        /// To check if allocation is not lost, call `Allocator::get_allocation_info` and check if
        /// `AllocationInfo::device_memory` is not null.
        ///
        /// You should not use this flag together with `AllocationCreateFlags::MAPPED`.
        const CAN_BECOME_LOST = 0x0000_0008;

        /// While creating allocation using this flag, other allocations that were
        /// created with flag `AllocationCreateFlags::CAN_BECOME_LOST` can become lost.
        const CAN_MAKE_OTHER_LOST = 0x0000_0010;

        /// Set this flag to treat `AllocationCreateInfo::user_data` as pointer to a
        /// null-terminated string. Instead of copying pointer value, a local copy of the
        /// string is made and stored in allocation's user data. The string is automatically
        /// freed together with the allocation. It is also used in `Allocator::build_stats_string`.
        const USER_DATA_COPY_STRING = 0x0000_0020;

        /// Allocation will be created from upper stack in a double stack pool.
        ///
        /// This flag is only allowed for custom pools created with `AllocatorPoolCreateFlags::LINEAR_ALGORITHM` flag.
        const UPPER_ADDRESS = 0x0000_0040;

        /// Create both buffer/image and allocation, but don't bind them together.
        /// It is useful when you want to bind yourself to do some more advanced binding, e.g. using some extensions.
        /// The flag is meaningful only with functions that bind by default, such as `Allocator::create_buffer`
        /// or `Allocator::create_image`. Otherwise it is ignored.
        const CREATE_DONT_BIND = 0x0000_0080;

        /// Allocation strategy that chooses smallest possible free range for the
        /// allocation.
        const STRATEGY_BEST_FIT = 0x0001_0000;

        /// Allocation strategy that chooses biggest possible free range for the
        /// allocation.
        const STRATEGY_WORST_FIT = 0x0002_0000;

        /// Allocation strategy that chooses first suitable free range for the
        /// allocation.
        ///
        /// "First" doesn't necessarily means the one with smallest offset in memory,
        /// but rather the one that is easiest and fastest to find.
        const STRATEGY_FIRST_FIT = 0x0004_0000;

        /// Allocation strategy that tries to minimize memory usage.
        const STRATEGY_MIN_MEMORY = 0x0001_0000;

        /// Allocation strategy that tries to minimize allocation time.
        const STRATEGY_MIN_TIME = 0x0004_0000;

        /// Allocation strategy that tries to minimize memory fragmentation.
        const STRATEGY_MIN_FRAGMENTATION = 0x0002_0000;

        /// A bit mask to extract only `*_STRATEGY` bits from entire set of flags.
        const STRATEGY_MASK = 0x0001_0000 | 0x0002_0000 | 0x0004_0000;
    }
}

bitflags! {
    /// Flags for configuring `AllocatorPool` construction.
    pub struct AllocatorPoolCreateFlags: u32 {
        const NONE = 0;

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
        const IGNORE_BUFFER_IMAGE_GRANULARITY = ffi::VmaPoolCreateFlagBits_VMA_POOL_CREATE_IGNORE_BUFFER_IMAGE_GRANULARITY_BIT;

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
        const LINEAR_ALGORITHM = ffi::VmaPoolCreateFlagBits_VMA_POOL_CREATE_LINEAR_ALGORITHM_BIT;

        /// Enables alternative, buddy allocation algorithm in this pool.
        ///
        /// It operates on a tree of blocks, each having size that is a power of two and
        /// a half of its parent's size. Comparing to default algorithm, this one provides
        /// faster allocation and deallocation and decreased external fragmentation,
        /// at the expense of more memory wasted (internal fragmentation).
        const BUDDY_ALGORITHM = ffi::VmaPoolCreateFlagBits_VMA_POOL_CREATE_BUDDY_ALGORITHM_BIT;

        /// \brief Enables alternative, Two-Level Segregated Fit (TLSF) allocation algorithm in this pool.
        ///
        /// This algorithm is based on 2-level lists dividing address space into smaller
        /// chunks. The first level is aligned to power of two which serves as buckets for requested
        /// memory to fall into, and the second level is lineary subdivided into lists of free memory.
        /// This algorithm aims to achieve bounded response time even in the worst case scenario.
        /// Allocation time can be sometimes slightly longer than compared to other algorithms
        /// but in return the application can avoid stalls in case of fragmentation, giving
        /// predictable results, suitable for real-time use cases.
        const TLSF_ALGORITHM_BIT = ffi::VmaPoolCreateFlagBits_VMA_POOL_CREATE_TLSF_ALGORITHM_BIT;

        /// Bit mask to extract only `*_ALGORITHM` bits from entire set of flags.
        const ALGORITHM_MASK = ffi::VmaPoolCreateFlagBits_VMA_POOL_CREATE_ALGORITHM_MASK;
    }
}

pub struct AllocatorCreateInfo<'a> {
    pub(crate) inner: ffi::VmaAllocatorCreateInfo,
    pub(crate) physical_device: &'a PhysicalDevice,
    pub(crate) device: &'a Device,
    pub(crate) instance: &'a Instance,
    pub(crate) marker: ::std::marker::PhantomData<&'a ()>,
}

impl<'a> AllocatorCreateInfo<'a> {
    pub fn new(
        instance: &'a ash::Instance,
        device: &'a ash::Device,
        physical_device: &'a ash::vk::PhysicalDevice,
    ) -> AllocatorCreateInfo<'a> {
        AllocatorCreateInfo {
            inner: ffi::VmaAllocatorCreateInfo {
                flags: 0,
                physicalDevice: *physical_device,
                instance: instance.handle(),
                device: device.handle(),
                preferredLargeHeapBlockSize: 0,
                pAllocationCallbacks: ptr::null(),
                pDeviceMemoryCallbacks: ptr::null(),
                pHeapSizeLimit: ptr::null(),
                pVulkanFunctions: ptr::null(),
                vulkanApiVersion: 0,
                pTypeExternalMemoryHandleTypes: ptr::null(),
            },
            physical_device,
            device,
            instance,
            marker: ::std::marker::PhantomData,
        }
    }

    pub fn preferred_large_heap_block_size(mut self, size: u64) -> Self {
        self.inner.preferredLargeHeapBlockSize = size;
        self
    }

    pub fn flags(mut self, flags: AllocationCreateFlags) -> Self {
        self.inner.flags = flags.bits;
        self
    }

    pub fn heap_size_limit(mut self, device_sizes: &'a [ash::vk::DeviceSize]) -> Self {
        unsafe {
            debug_assert!(
                self.instance
                    .get_physical_device_memory_properties(*self.physical_device)
                    .memory_heap_count
                    == device_sizes.len() as u32
            );
        }
        self.inner.pHeapSizeLimit = device_sizes.as_ptr();
        self
    }

    pub fn allocation_callback(mut self, allocation: &'a ash::vk::AllocationCallbacks) -> Self {
        self.inner.pAllocationCallbacks = allocation as *const _;
        self
    }

    pub fn vulkan_api_version(mut self, version: u32) -> Self {
        self.inner.vulkanApiVersion = version;
        self
    }

    pub fn external_memory_handles(
        mut self,
        external_memory_handles: &'a [ash::vk::ExternalMemoryHandleTypeFlagsKHR],
    ) -> Self {
        unsafe {
            debug_assert!(
                self.instance
                    .get_physical_device_memory_properties(*self.physical_device)
                    .memory_type_count
                    == external_memory_handles.len() as u32
            );
        }
        self.inner.pTypeExternalMemoryHandleTypes = external_memory_handles.as_ptr();
        self
    }
}

pub struct PoolCreateInfo<'a> {
    pub(crate) inner: ffi::VmaPoolCreateInfo,
    marker: ::std::marker::PhantomData<&'a ()>,
}

impl<'a> PoolCreateInfo<'a> {
    pub fn new() -> PoolCreateInfo<'a> {
        PoolCreateInfo {
            inner: ffi::VmaPoolCreateInfo {
                memoryTypeIndex: 0,
                flags: 0,
                blockSize: 0,
                minBlockCount: 0,
                maxBlockCount: 0,
                priority: 0.0,
                minAllocationAlignment: 0,
                pMemoryAllocateNext: ptr::null_mut(),
            },
            marker: ::std::marker::PhantomData,
        }
    }

    pub fn memory_type_index(mut self, index: u32) -> Self {
        self.inner.memoryTypeIndex = index;
        self
    }

    pub fn flags(mut self, flags: &AllocatorPoolCreateFlags) -> Self {
        self.inner.flags = flags.bits;
        self
    }

    pub fn block_size(mut self, block_size: u64) -> Self {
        self.inner.blockSize = block_size;
        self
    }

    pub fn min_block_count(mut self, min_block_count: usize) -> Self {
        self.inner.minBlockCount = min_block_count;
        self
    }

    pub fn max_block_count(mut self, max_block_count: usize) -> Self {
        self.inner.maxBlockCount = max_block_count;
        self
    }

    pub fn priority(mut self, priority: f32) -> Self {
        self.inner.priority = priority;
        self
    }

    pub fn min_allocation_alignment(mut self, alignment: u64) -> Self {
        self.inner.minAllocationAlignment = alignment;
        self
    }

    pub fn memory_allocate(mut self, next: &'a mut ash::vk::MemoryAllocateInfo) -> Self {
        self.inner.pMemoryAllocateNext = next as *mut ash::vk::MemoryAllocateInfo as *mut _;
        self
    }
}

pub struct AllocationCreateInfo<'a> {
    pub(crate) inner: ffi::VmaAllocationCreateInfo,
    marker: ::std::marker::PhantomData<&'a ()>,
}

impl<'a> AllocationCreateInfo<'a> {
    pub fn new() -> AllocationCreateInfo<'a> {
        AllocationCreateInfo {
            inner: VmaAllocationCreateInfo {
                flags: 0,
                usage: 0,
                requiredFlags: Default::default(),
                preferredFlags: Default::default(),
                memoryTypeBits: 0,
                pool: ptr::null_mut(),
                pUserData: ptr::null_mut(),
                priority: 0.0,
            },
            marker: ::std::marker::PhantomData,
        }
    }

    pub fn flags(mut self, flags: AllocationCreateFlags) -> Self {
        self.inner.flags = flags.bits();
        self
    }

    pub fn usage(mut self, usage: MemoryUsage) -> Self {
        self.inner.usage = match usage {
            MemoryUsage::Unknown => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_UNKNOWN,
            MemoryUsage::GpuOnly => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_ONLY,
            MemoryUsage::CpuOnly => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_CPU_ONLY,
            MemoryUsage::CpuToGpu => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_CPU_TO_GPU,
            MemoryUsage::GpuToCpu => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_TO_CPU,
            MemoryUsage::GpuLazy => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_LAZILY_ALLOCATED,
        };
        self
    }

    pub fn required_flags(mut self, flags: ash::vk::MemoryPropertyFlags) -> Self {
        self.inner.requiredFlags = flags;
        self
    }

    pub fn preferred_flags(mut self, flags: ash::vk::MemoryPropertyFlags) -> Self {
        self.inner.preferredFlags = flags;
        self
    }

    pub fn memory_type_bits(mut self, flags: u32) -> Self {
        self.inner.memoryTypeBits = flags;
        self
    }

    pub fn pool(mut self, pool: AllocatorPool) -> Self {
        self.inner.pool = pool;
        self
    }

    pub fn user_data(mut self, p_user_data: *mut ::std::os::raw::c_void) -> Self {
        self.inner.pUserData = p_user_data;
        self
    }

    pub fn priority(mut self, priority: f32) -> Self {
        self.inner.priority = priority;
        self
    }
}
