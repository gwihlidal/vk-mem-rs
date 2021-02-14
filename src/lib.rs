//! Easy to use, high performance memory manager for Vulkan.

#![allow(invalid_value)]

extern crate ash;
#[macro_use]
extern crate bitflags;
#[cfg(feature = "failure")]
extern crate failure;

pub mod error;
pub mod ffi;
pub use crate::error::{Error, ErrorKind, Result};
use ash::{version::InstanceV1_0, vk::Handle};
use std::mem;

/// Main allocator object
pub struct Allocator {
    /// Pointer to internal VmaAllocator instance
    pub(crate) internal: ffi::VmaAllocator,

    /// Vulkan instance handle
    #[allow(dead_code)]
    pub(crate) instance: ash::Instance,

    /// Vulkan device handle
    #[allow(dead_code)]
    pub(crate) device: ash::Device,
}

// Allocator is internally thread safe unless AllocatorCreateFlags::EXTERNALLY_SYNCHRONIZED is used (then you need to add synchronization!)
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

/// Represents custom memory pool
///
/// Fill structure `AllocatorPoolCreateInfo` and call `Allocator::create_pool` to create it.
/// Call `Allocator::destroy_pool` to destroy it.
#[derive(Debug, Clone)]
pub struct AllocatorPool {
    /// Pointer to internal VmaPool instance
    pub(crate) internal: ffi::VmaPool,
}

/// Construct `AllocatorPool` with default values
impl Default for AllocatorPool {
    fn default() -> Self {
        AllocatorPool {
            internal: unsafe { mem::zeroed() },
        }
    }
}

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
#[derive(Debug, Copy, Clone)]
pub struct Allocation {
    /// Pointer to internal VmaAllocation instance
    pub(crate) internal: ffi::VmaAllocation,
}

impl Allocation {
    pub fn null() -> Allocation {
        Allocation {
            internal: std::ptr::null_mut(),
        }
    }
}

unsafe impl Send for Allocation {}
unsafe impl Sync for Allocation {}

/// Parameters of `Allocation` objects, that can be retrieved using `Allocator::get_allocation_info`.
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Pointer to internal VmaAllocationInfo instance
    pub(crate) internal: ffi::VmaAllocationInfo,
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
        ash::vk::DeviceMemory::from_raw(self.internal.deviceMemory as u64)
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

bitflags! {
    /// Flags for configuring `Allocator` construction.
    pub struct AllocatorCreateFlags: u32 {
        /// No allocator configuration other than defaults.
        const NONE = 0x0000_0000;

        /// Allocator and all objects created from it will not be synchronized internally,
        /// so you must guarantee they are used from only one thread at a time or synchronized
        /// externally by you. Using this flag may increase performance because internal
        /// mutexes are not used.
        const EXTERNALLY_SYNCHRONIZED = 0x0000_0001;

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
        const KHR_DEDICATED_ALLOCATION = 0x0000_0002;
    }
}

/// Construct `AllocatorCreateFlags` with default values
impl Default for AllocatorCreateFlags {
    fn default() -> Self {
        AllocatorCreateFlags::NONE
    }
}

/// Description of an `Allocator` to be created.
pub struct AllocatorCreateInfo {
    /// Vulkan physical device. It must be valid throughout whole lifetime of created allocator.
    pub physical_device: ash::vk::PhysicalDevice,

    /// Vulkan device. It must be valid throughout whole lifetime of created allocator.
    pub device: ash::Device,

    /// Vulkan instance. It must be valid throughout whole lifetime of created allocator.
    pub instance: ash::Instance,

    /// Flags for created allocator.
    pub flags: AllocatorCreateFlags,

    /// Preferred size of a single `ash::vk::DeviceMemory` block to be allocated from large heaps > 1 GiB.
    /// Set to 0 to use default, which is currently 256 MiB.
    pub preferred_large_heap_block_size: usize,

    /// Maximum number of additional frames that are in use at the same time as current frame.
    ///
    /// This value is used only when you make allocations with `AllocationCreateFlags::CAN_BECOME_LOST` flag.
    ///
    /// Such allocations cannot become lost if:
    /// `allocation.lastUseFrameIndex >= allocator.currentFrameIndex - frameInUseCount`
    ///
    /// For example, if you double-buffer your command buffers, so resources used for
    /// rendering in previous frame may still be in use by the GPU at the moment you
    /// allocate resources needed for the current frame, set this value to 1.
    ///
    /// If you want to allow any allocations other than used in the current frame to
    /// become lost, set this value to 0.
    pub frame_in_use_count: u32,

    /// Either empty or an array of limits on maximum number of bytes that can be allocated
    /// out of particular Vulkan memory heap.
    ///
    /// If not empty, it must contain `ash::vk::PhysicalDeviceMemoryProperties::memory_heap_count` elements,
    /// defining limit on maximum number of bytes that can be allocated out of particular Vulkan
    /// memory heap.
    ///
    /// Any of the elements may be equal to `ash::vk::WHOLE_SIZE`, which means no limit on that
    /// heap. This is also the default in case of an empty slice.
    ///
    /// If there is a limit defined for a heap:
    ///
    /// * If user tries to allocate more memory from that heap using this allocator, the allocation
    /// fails with `ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY`.
    ///
    /// * If the limit is smaller than heap size reported in `ash::vk::MemoryHeap::size`, the value of this
    /// limit will be reported instead when using `Allocator::get_memory_properties`.
    ///
    /// Warning! Using this feature may not be equivalent to installing a GPU with smaller amount of
    /// memory, because graphics driver doesn't necessary fail new allocations with
    /// `ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY` result when memory capacity is exceeded. It may return success
    /// and just silently migrate some device memory" blocks to system RAM. This driver behavior can
    /// also be controlled using the `VK_AMD_memory_overallocation_behavior` extension.
    pub heap_size_limits: Option<Vec<ash::vk::DeviceSize>>,
}

/// Construct `AllocatorCreateInfo` with default values
///
/// Note that the default `device` and `instance` fields are filled with dummy
/// implementations that will panic if used. These fields must be overwritten.
impl Default for AllocatorCreateInfo {
    fn default() -> Self {
        extern "C" fn get_device_proc_addr(
            _: ash::vk::Instance,
            _: *const std::os::raw::c_char,
        ) -> *const std::os::raw::c_void {
            std::ptr::null()
        }
        extern "C" fn get_instance_proc_addr(
            _: ash::vk::Instance,
            _: *const std::os::raw::c_char,
        ) -> *const std::os::raw::c_void {
            get_device_proc_addr as *const _
        }
        let static_fn = ash::vk::StaticFn::load(|_| get_instance_proc_addr as *const _);
        let instance = unsafe { ash::Instance::load(&static_fn, ash::vk::Instance::null()) };
        let device = unsafe { ash::Device::load(&instance.fp_v1_0(), ash::vk::Device::null()) };
        AllocatorCreateInfo {
            physical_device: ash::vk::PhysicalDevice::null(),
            device,
            instance,
            flags: AllocatorCreateFlags::NONE,
            preferred_large_heap_block_size: 0,
            frame_in_use_count: 0,
            heap_size_limits: None,
        }
    }
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
}

bitflags! {
    /// Flags for configuring `AllocatorPool` construction.
    pub struct AllocatorPoolCreateFlags: u32 {
        const NONE = 0x0000_0000;

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
        const IGNORE_BUFFER_IMAGE_GRANULARITY = 0x0000_0002;

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
        const LINEAR_ALGORITHM = 0x0000_0004;

        /// Enables alternative, buddy allocation algorithm in this pool.
        ///
        /// It operates on a tree of blocks, each having size that is a power of two and
        /// a half of its parent's size. Comparing to default algorithm, this one provides
        /// faster allocation and deallocation and decreased external fragmentation,
        /// at the expense of more memory wasted (internal fragmentation).
        const BUDDY_ALGORITHM = 0x0000_0008;

        /// Bit mask to extract only `*_ALGORITHM` bits from entire set of flags.
        const ALGORITHM_MASK = 0x0000_0004 | 0x0000_0008;
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

/// Description of an `Allocation` to be created.
#[derive(Debug, Clone)]
pub struct AllocationCreateInfo {
    /// Intended usage of memory.
    ///
    /// You can leave `MemoryUsage::UNKNOWN` if you specify memory requirements
    /// in another way.
    ///
    /// If `pool` is not `None`, this member is ignored.
    pub usage: MemoryUsage,

    /// Flags for configuring the allocation
    pub flags: AllocationCreateFlags,

    /// Flags that must be set in a Memory Type chosen for an allocation.
    ///
    /// Leave 0 if you specify memory requirements in other way.
    ///
    /// If `pool` is not `None`, this member is ignored.
    pub required_flags: ash::vk::MemoryPropertyFlags,

    /// Flags that preferably should be set in a memory type chosen for an allocation.
    ///
    /// Set to 0 if no additional flags are prefered.
    ///
    /// If `pool` is not `None`, this member is ignored.
    pub preferred_flags: ash::vk::MemoryPropertyFlags,

    /// Bit mask containing one bit set for every memory type acceptable for this allocation.
    ///
    /// Value 0 is equivalent to `std::u32::MAX` - it means any memory type is accepted if
    /// it meets other requirements specified by this structure, with no further restrictions
    /// on memory type index.
    ///
    /// If `pool` is not `None`, this member is ignored.
    pub memory_type_bits: u32,

    /// Pool that this allocation should be created in.
    ///
    /// Specify `None` to allocate from default pool. If not `None`, members:
    /// `usage`, `required_flags`, `preferred_flags`, `memory_type_bits` are ignored.
    pub pool: Option<AllocatorPool>,

    /// Custom general-purpose pointer that will be stored in `Allocation`, can be read
    /// as `Allocation::get_user_data()` and changed using `Allocator::set_allocation_user_data`.
    ///
    /// If `AllocationCreateFlags::USER_DATA_COPY_STRING` is used, it must be either null or pointer to a
    /// null-terminated string. The string will be then copied to internal buffer, so it
    /// doesn't need to be valid after allocation call.
    pub user_data: Option<*mut ::std::os::raw::c_void>,
}

/// Construct `AllocationCreateInfo` with default values
impl Default for AllocationCreateInfo {
    fn default() -> Self {
        AllocationCreateInfo {
            usage: MemoryUsage::Unknown,
            flags: AllocationCreateFlags::NONE,
            required_flags: ash::vk::MemoryPropertyFlags::empty(),
            preferred_flags: ash::vk::MemoryPropertyFlags::empty(),
            memory_type_bits: 0,
            pool: None,
            user_data: None,
        }
    }
}

/// Description of an `AllocationPool` to be created.
#[derive(Debug, Clone)]
pub struct AllocatorPoolCreateInfo {
    /// Vulkan memory type index to allocate this pool from.
    pub memory_type_index: u32,

    /// Use combination of `AllocatorPoolCreateFlags`
    pub flags: AllocatorPoolCreateFlags,

    /// Size of a single `ash::vk::DeviceMemory` block to be allocated as part of this
    /// pool, in bytes.
    ///
    /// Specify non-zero to set explicit, constant size of memory blocks used by
    /// this pool.
    ///
    /// Leave 0 to use default and let the library manage block sizes automatically.
    /// Sizes of particular blocks may vary.
    pub block_size: usize,

    /// Minimum number of blocks to be always allocated in this pool, even if they stay empty.
    ///
    /// Set to 0 to have no preallocated blocks and allow the pool be completely empty.
    pub min_block_count: usize,

    /// Maximum number of blocks that can be allocated in this pool.
    ///
    /// Set to 0 to use default, which is no limit.
    ///
    /// Set to same value as `AllocatorPoolCreateInfo::min_block_count` to have fixed amount
    /// of memory allocated throughout whole lifetime of this pool.
    pub max_block_count: usize,

    /// Maximum number of additional frames that are in use at the same time as current frame.
    /// This value is used only when you make allocations with `AllocationCreateFlags::CAN_BECOME_LOST` flag.
    /// Such allocations cannot become lost if:
    ///   `allocation.lastUseFrameIndex >= allocator.currentFrameIndex - frameInUseCount`.
    ///
    /// For example, if you double-buffer your command buffers, so resources used for rendering
    /// in previous frame may still be in use by the GPU at the moment you allocate resources
    /// needed for the current frame, set this value to 1.
    ///
    /// If you want to allow any allocations other than used in the current frame to become lost,
    /// set this value to 0.
    pub frame_in_use_count: u32,
}

/// Construct `AllocatorPoolCreateInfo` with default values
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

#[derive(Debug)]
pub struct DefragmentationContext {
    pub(crate) internal: ffi::VmaDefragmentationContext,
    pub(crate) stats: Box<ffi::VmaDefragmentationStats>,
    pub(crate) changed: Vec<ash::vk::Bool32>,
}

/// Optional configuration parameters to be passed to `Allocator::defragment`
///
/// DEPRECATED.
#[derive(Debug, Copy, Clone)]
pub struct DefragmentationInfo {
    /// Maximum total numbers of bytes that can be copied while moving
    /// allocations to different places.
    ///
    /// Default is `ash::vk::WHOLE_SIZE`, which means no limit.
    pub max_bytes_to_move: usize,

    /// Maximum number of allocations that can be moved to different place.
    ///
    /// Default is `std::u32::MAX`, which means no limit.
    pub max_allocations_to_move: u32,
}

/// Construct `DefragmentationInfo` with default values
impl Default for DefragmentationInfo {
    fn default() -> Self {
        DefragmentationInfo {
            max_bytes_to_move: ash::vk::WHOLE_SIZE as usize,
            max_allocations_to_move: std::u32::MAX,
        }
    }
}

/// Parameters for defragmentation.
///
/// To be used with function `Allocator::defragmentation_begin`.
#[derive(Debug, Clone)]
pub struct DefragmentationInfo2<'a> {
    /// Collection of allocations that can be defragmented.
    ///
    /// Elements in the slice should be unique - same allocation cannot occur twice.
    /// It is safe to pass allocations that are in the lost state - they are ignored.
    /// All allocations not present in this slice are considered non-moveable during this defragmentation.
    pub allocations: &'a [Allocation],

    /// Either `None` or a slice of pools to be defragmented.
    ///
    /// All the allocations in the specified pools can be moved during defragmentation
    /// and there is no way to check if they were really moved as in `allocations_changed`,
    /// so you must query all the allocations in all these pools for new `ash::vk::DeviceMemory`
    /// and offset using `Allocator::get_allocation_info` if you might need to recreate buffers
    /// and images bound to them.
    ///
    /// Elements in the array should be unique - same pool cannot occur twice.
    ///
    /// Using this array is equivalent to specifying all allocations from the pools in `allocations`.
    /// It might be more efficient.
    pub pools: Option<&'a [AllocatorPool]>,

    /// Maximum total numbers of bytes that can be copied while moving allocations to different places using transfers on CPU side, like `memcpy()`, `memmove()`.
    ///
    /// `ash::vk::WHOLE_SIZE` means no limit.
    pub max_cpu_bytes_to_move: ash::vk::DeviceSize,

    /// Maximum number of allocations that can be moved to a different place using transfers on CPU side, like `memcpy()`, `memmove()`.
    ///
    /// `std::u32::MAX` means no limit.
    pub max_cpu_allocations_to_move: u32,

    /// Maximum total numbers of bytes that can be copied while moving allocations to different places using transfers on GPU side, posted to `command_buffer`.
    ///
    /// `ash::vk::WHOLE_SIZE` means no limit.
    pub max_gpu_bytes_to_move: ash::vk::DeviceSize,

    /// Maximum number of allocations that can be moved to a different place using transfers on GPU side, posted to `command_buffer`.
    ///
    /// `std::u32::MAX` means no limit.
    pub max_gpu_allocations_to_move: u32,

    /// Command buffer where GPU copy commands will be posted.
    ///
    /// If not `None`, it must be a valid command buffer handle that supports transfer queue type.
    /// It must be in the recording state and outside of a render pass instance.
    /// You need to submit it and make sure it finished execution before calling `Allocator::defragmentation_end`.
    ///
    /// Passing `None` means that only CPU defragmentation will be performed.
    pub command_buffer: Option<ash::vk::CommandBuffer>,
}

/// Statistics returned by `Allocator::defragment`
#[derive(Debug, Copy, Clone)]
pub struct DefragmentationStats {
    /// Total number of bytes that have been copied while moving allocations to different places.
    pub bytes_moved: usize,

    /// Total number of bytes that have been released to the system by freeing empty `ash::vk::DeviceMemory` objects.
    pub bytes_freed: usize,

    /// Number of allocations that have been moved to different places.
    pub allocations_moved: u32,

    /// Number of empty `ash::vk::DeviceMemory` objects that have been released to the system.
    pub device_memory_blocks_freed: u32,
}

impl Allocator {
    /// Constructor a new `Allocator` using the provided options.
    pub fn new(create_info: &AllocatorCreateInfo) -> Result<Self> {
        use ash::version::{DeviceV1_0, DeviceV1_1};
        let instance = create_info.instance.clone();
        let device = create_info.device.clone();
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
                vkBindBufferMemory2KHR: mem::transmute::<_, ffi::PFN_vkBindBufferMemory2KHR>(Some(
                    device.fp_v1_1().bind_buffer_memory2,
                )),
                vkBindImageMemory: mem::transmute::<_, ffi::PFN_vkBindImageMemory>(Some(
                    device.fp_v1_0().bind_image_memory,
                )),
                vkBindImageMemory2KHR: mem::transmute::<_, ffi::PFN_vkBindImageMemory2KHR>(Some(
                    device.fp_v1_1().bind_image_memory2,
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
                vkCmdCopyBuffer: mem::transmute::<_, ffi::PFN_vkCmdCopyBuffer>(Some(
                    device.fp_v1_0().cmd_copy_buffer,
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
                // TODO:
                vkGetPhysicalDeviceMemoryProperties2KHR: None,
                /*vkGetPhysicalDeviceMemoryProperties2KHR: mem::transmute::<
                    _,
                    ffi::PFN_vkGetPhysicalDeviceMemoryProperties2KHR,
                >(Some(
                    device.fp_v1_1().get_physical_device_memory_properties2,
                )),*/
            }
        };
        let ffi_create_info = ffi::VmaAllocatorCreateInfo {
            physicalDevice: create_info.physical_device.as_raw() as ffi::VkPhysicalDevice,
            device: create_info.device.handle().as_raw() as ffi::VkDevice,
            instance: instance.handle().as_raw() as ffi::VkInstance,
            flags: create_info.flags.bits(),
            frameInUseCount: create_info.frame_in_use_count,
            preferredLargeHeapBlockSize: create_info.preferred_large_heap_block_size as u64,
            pHeapSizeLimit: match &create_info.heap_size_limits {
                None => ::std::ptr::null(),
                Some(limits) => limits.as_ptr(),
            },
            pVulkanFunctions: &routed_functions,
            pAllocationCallbacks: ::std::ptr::null(), // TODO: Add support
            pDeviceMemoryCallbacks: ::std::ptr::null(), // TODO: Add support
            pRecordSettings: ::std::ptr::null(),      // TODO: Add support
            vulkanApiVersion: 0,                      // TODO: Make configurable
        };
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

    /// The allocator fetches `ash::vk::PhysicalDeviceProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub fn get_physical_device_properties(&self) -> Result<ash::vk::PhysicalDeviceProperties> {
        let mut ffi_properties: *const ffi::VkPhysicalDeviceProperties = unsafe { mem::zeroed() };
        Ok(unsafe {
            ffi::vmaGetPhysicalDeviceProperties(self.internal, &mut ffi_properties);
            mem::transmute::<ffi::VkPhysicalDeviceProperties, ash::vk::PhysicalDeviceProperties>(
                *ffi_properties,
            )
        })
    }

    /// The allocator fetches `ash::vk::PhysicalDeviceMemoryProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
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

    /// Given a memory type index, returns `ash::vk::MemoryPropertyFlags` of this memory type.
    ///
    /// This is just a convenience function; the same information can be obtained using
    /// `Allocator::get_memory_properties`.
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

    /// Sets index of the current frame.
    ///
    /// This function must be used if you make allocations with `AllocationCreateFlags::CAN_BECOME_LOST` and
    /// `AllocationCreateFlags::CAN_MAKE_OTHER_LOST` flags to inform the allocator when a new frame begins.
    /// Allocations queried using `Allocator::get_allocation_info` cannot become lost
    /// in the current frame.
    pub fn set_current_frame_index(&self, frame_index: u32) {
        unsafe {
            ffi::vmaSetCurrentFrameIndex(self.internal, frame_index);
        }
    }

    /// Retrieves statistics from current state of the `Allocator`.
    pub fn calculate_stats(&self) -> Result<ffi::VmaStats> {
        let mut vma_stats: ffi::VmaStats = unsafe { mem::zeroed() };
        unsafe {
            ffi::vmaCalculateStats(self.internal, &mut vma_stats);
        }
        Ok(vma_stats)
    }

    /// Builds and returns statistics in `JSON` format.
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

    /// Allocates Vulkan device memory and creates `AllocatorPool` object.
    pub fn create_pool(&self, pool_info: &AllocatorPoolCreateInfo) -> Result<AllocatorPool> {
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

    /// Destroys `AllocatorPool` object and frees Vulkan device memory.
    pub fn destroy_pool(&self, pool: &AllocatorPool) {
        unsafe {
            ffi::vmaDestroyPool(self.internal, pool.internal);
        }
    }

    /// Retrieves statistics of existing `AllocatorPool` object.
    pub fn get_pool_stats(&self, pool: &AllocatorPool) -> Result<ffi::VmaPoolStats> {
        let mut pool_stats: ffi::VmaPoolStats = unsafe { mem::zeroed() };
        unsafe {
            ffi::vmaGetPoolStats(self.internal, pool.internal, &mut pool_stats);
        }
        Ok(pool_stats)
    }

    /// Marks all allocations in given pool as lost if they are not used in current frame
    /// or AllocatorPoolCreateInfo::frame_in_use_count` back from now.
    ///
    /// Returns the number of allocations marked as lost.
    pub fn make_pool_allocations_lost(&self, pool: &mut AllocatorPool) -> Result<usize> {
        let mut lost_count: usize = 0;
        unsafe {
            ffi::vmaMakePoolAllocationsLost(self.internal, pool.internal, &mut lost_count);
        }
        Ok(lost_count as usize)
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
    pub fn check_pool_corruption(&self, pool: &AllocatorPool) -> Result<()> {
        let result =
            ffi_to_result(unsafe { ffi::vmaCheckPoolCorruption(self.internal, pool.internal) });
        match result {
            ash::vk::Result::SUCCESS => Ok(()),
            _ => Err(Error::vulkan(result)),
        }
    }

    /// General purpose memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    ///
    /// It is recommended to use `Allocator::allocate_memory_for_buffer`, `Allocator::allocate_memory_for_image`,
    /// `Allocator::create_buffer`, `Allocator::create_image` instead whenever possible.
    pub fn allocate_memory(
        &self,
        memory_requirements: &ash::vk::MemoryRequirements,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<(Allocation, AllocationInfo)> {
        let ffi_requirements = unsafe {
            mem::transmute::<ash::vk::MemoryRequirements, ffi::VkMemoryRequirements>(
                *memory_requirements,
            )
        };
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let mut allocation_info: AllocationInfo = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaAllocateMemory(
                self.internal,
                &ffi_requirements,
                &create_info,
                &mut allocation.internal,
                &mut allocation_info.internal,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok((allocation, allocation_info)),
            _ => Err(Error::vulkan(result)),
        }
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
    pub fn allocate_memory_pages(
        &self,
        memory_requirements: &ash::vk::MemoryRequirements,
        allocation_info: &AllocationCreateInfo,
        allocation_count: usize,
    ) -> Result<Vec<(Allocation, AllocationInfo)>> {
        let ffi_requirements = unsafe {
            mem::transmute::<ash::vk::MemoryRequirements, ffi::VkMemoryRequirements>(
                *memory_requirements,
            )
        };
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocations: Vec<ffi::VmaAllocation> =
            vec![unsafe { mem::zeroed() }; allocation_count];
        let mut allocation_info: Vec<ffi::VmaAllocationInfo> =
            vec![unsafe { mem::zeroed() }; allocation_count];
        let result = ffi_to_result(unsafe {
            ffi::vmaAllocateMemoryPages(
                self.internal,
                &ffi_requirements,
                &create_info,
                allocation_count,
                allocations.as_mut_ptr(),
                allocation_info.as_mut_ptr(),
            )
        });
        match result {
            ash::vk::Result::SUCCESS => {
                let it = allocations.iter().zip(allocation_info.iter());
                let allocations: Vec<(Allocation, AllocationInfo)> = it
                    .map(|(alloc, info)| {
                        (
                            Allocation { internal: *alloc },
                            AllocationInfo { internal: *info },
                        )
                    })
                    .collect();
                Ok(allocations)
            }
            _ => Err(Error::vulkan(result)),
        }
    }

    /// Buffer specialized memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    pub fn allocate_memory_for_buffer(
        &self,
        buffer: ash::vk::Buffer,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<(Allocation, AllocationInfo)> {
        let ffi_buffer = buffer.as_raw() as ffi::VkBuffer;
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let mut allocation_info: AllocationInfo = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaAllocateMemoryForBuffer(
                self.internal,
                ffi_buffer,
                &create_info,
                &mut allocation.internal,
                &mut allocation_info.internal,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok((allocation, allocation_info)),
            _ => Err(Error::vulkan(result)),
        }
    }

    /// Image specialized memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    pub fn allocate_memory_for_image(
        &self,
        image: ash::vk::Image,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<(Allocation, AllocationInfo)> {
        let ffi_image = image.as_raw() as ffi::VkImage;
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let mut allocation_info: AllocationInfo = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaAllocateMemoryForImage(
                self.internal,
                ffi_image,
                &create_info,
                &mut allocation.internal,
                &mut allocation_info.internal,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok((allocation, allocation_info)),
            _ => Err(Error::vulkan(result)),
        }
    }

    /// Frees memory previously allocated using `Allocator::allocate_memory`,
    /// `Allocator::allocate_memory_for_buffer`, or `Allocator::allocate_memory_for_image`.
    pub fn free_memory(&self, allocation: &Allocation) {
        unsafe {
            ffi::vmaFreeMemory(self.internal, allocation.internal);
        }
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
    pub fn free_memory_pages(&self, allocations: &[Allocation]) {
        let mut allocations_ffi: Vec<ffi::VmaAllocation> =
            allocations.iter().map(|x| x.internal).collect();
        unsafe {
            ffi::vmaFreeMemoryPages(
                self.internal,
                allocations_ffi.len(),
                allocations_ffi.as_mut_ptr(),
            );
        }
    }

    /// Tries to resize an allocation in place, if there is enough free memory after it.
    ///
    /// Tries to change allocation's size without moving or reallocating it.
    /// You can both shrink and grow allocation size.
    /// When growing, it succeeds only when the allocation belongs to a memory block with enough
    /// free space after it.
    ///
    /// Returns `ash::vk::Result::SUCCESS` if allocation's size has been successfully changed.
    /// Returns `ash::vk::Result::ERROR_OUT_OF_POOL_MEMORY` if allocation's size could not be changed.
    ///
    /// After successful call to this function, `AllocationInfo::get_size` of this allocation changes.
    /// All other parameters stay the same: memory pool and type, alignment, offset, mapped pointer.
    ///
    /// - Calling this function on allocation that is in lost state fails with result `ash::vk::Result::ERROR_VALIDATION_FAILED_EXT`.
    /// - Calling this function with `new_size` same as current allocation size does nothing and returns `ash::vk::Result::SUCCESS`.
    /// - Resizing dedicated allocations, as well as allocations created in pools that use linear
    ///   or buddy algorithm, is not supported. The function returns `ash::vk::Result::ERROR_FEATURE_NOT_PRESENT` in such cases.
    ///   Support may be added in the future.
    pub fn resize_allocation(&self, allocation: &Allocation, new_size: usize) -> Result<()> {
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
    pub fn get_allocation_info(&self, allocation: &Allocation) -> Result<AllocationInfo> {
        let mut allocation_info: AllocationInfo = unsafe { mem::zeroed() };
        unsafe {
            ffi::vmaGetAllocationInfo(
                self.internal,
                allocation.internal,
                &mut allocation_info.internal,
            )
        }
        Ok(allocation_info)
    }

    /// Returns `true` if allocation is not lost and atomically marks it as used in current frame.
    ///
    /// If the allocation has been created with `AllocationCreateFlags::CAN_BECOME_LOST` flag,
    /// this function returns `true` if it's not in lost state, so it can still be used.
    /// It then also atomically "touches" the allocation - marks it as used in current frame,
    /// so that you can be sure it won't become lost in current frame or next `frame_in_use_count` frames.
    ///
    /// If the allocation is in lost state, the function returns `false`.
    /// Memory of such allocation, as well as buffer or image bound to it, should not be used.
    /// Lost allocation and the buffer/image still need to be destroyed.
    ///
    /// If the allocation has been created without `AllocationCreateFlags::CAN_BECOME_LOST` flag,
    /// this function always returns `true`.
    pub fn touch_allocation(&self, allocation: &Allocation) -> Result<bool> {
        let result = unsafe { ffi::vmaTouchAllocation(self.internal, allocation.internal) };
        Ok(result == ash::vk::TRUE)
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
        allocation: &Allocation,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        ffi::vmaSetAllocationUserData(self.internal, allocation.internal, user_data);
    }

    /// Creates new allocation that is in lost state from the beginning.
    ///
    /// It can be useful if you need a dummy, non-null allocation.
    ///
    /// You still need to destroy created object using `Allocator::free_memory`.
    ///
    /// Returned allocation is not tied to any specific memory pool or memory type and
    /// not bound to any image or buffer. It has size = 0. It cannot be turned into
    /// a real, non-empty allocation.
    pub fn create_lost_allocation(&self) -> Result<Allocation> {
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        unsafe {
            ffi::vmaCreateLostAllocation(self.internal, &mut allocation.internal);
        }
        Ok(allocation)
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
    pub fn map_memory(&self, allocation: &Allocation) -> Result<*mut u8> {
        let mut mapped_data: *mut ::std::os::raw::c_void = ::std::ptr::null_mut();
        let result = ffi_to_result(unsafe {
            ffi::vmaMapMemory(self.internal, allocation.internal, &mut mapped_data)
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(mapped_data as *mut u8),
            _ => Err(Error::vulkan(result)),
        }
    }

    /// Unmaps memory represented by given allocation, mapped previously using `Allocator::map_memory`.
    pub fn unmap_memory(&self, allocation: &Allocation) {
        unsafe {
            ffi::vmaUnmapMemory(self.internal, allocation.internal);
        }
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
    pub fn flush_allocation(&self, allocation: &Allocation, offset: usize, size: usize) {
        unsafe {
            ffi::vmaFlushAllocation(
                self.internal,
                allocation.internal,
                offset as ffi::VkDeviceSize,
                size as ffi::VkDeviceSize,
            );
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
    pub fn invalidate_allocation(&self, allocation: &Allocation, offset: usize, size: usize) {
        unsafe {
            ffi::vmaInvalidateAllocation(
                self.internal,
                allocation.internal,
                offset as ffi::VkDeviceSize,
                size as ffi::VkDeviceSize,
            );
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
    pub fn check_corruption(&self, memory_types: ash::vk::MemoryPropertyFlags) -> Result<()> {
        let result =
            ffi_to_result(unsafe { ffi::vmaCheckCorruption(self.internal, memory_types.as_raw()) });
        match result {
            ash::vk::Result::SUCCESS => Ok(()),
            _ => Err(Error::vulkan(result)),
        }
    }

    /// Begins defragmentation process.
    ///
    /// Use this function instead of old, deprecated `Allocator::defragment`.
    ///
    /// Warning! Between the call to `Allocator::defragmentation_begin` and `Allocator::defragmentation_end`.
    ///
    /// - You should not use any of allocations passed as `allocations` or
    /// any allocations that belong to pools passed as `pools`,
    /// including calling `Allocator::get_allocation_info`, `Allocator::touch_allocation`, or access
    /// their data.
    ///
    /// - Some mutexes protecting internal data structures may be locked, so trying to
    /// make or free any allocations, bind buffers or images, map memory, or launch
    /// another simultaneous defragmentation in between may cause stall (when done on
    /// another thread) or deadlock (when done on the same thread), unless you are
    /// 100% sure that defragmented allocations are in different pools.
    ///
    /// - Information returned via stats and `info.allocations_changed` are undefined.
    /// They become valid after call to `Allocator::defragmentation_end`.
    ///
    /// - If `info.command_buffer` is not null, you must submit that command buffer
    /// and make sure it finished execution before calling `Allocator::defragmentation_end`.
    pub fn defragmentation_begin(
        &self,
        info: &DefragmentationInfo2,
    ) -> Result<DefragmentationContext> {
        let command_buffer = match info.command_buffer {
            Some(command_buffer) => command_buffer,
            None => ash::vk::CommandBuffer::null(),
        };
        let mut pools: Vec<ffi::VmaPool> = match info.pools {
            Some(ref pools) => pools.iter().map(|pool| pool.internal).collect(),
            None => Vec::new(),
        };
        let mut allocations: Vec<ffi::VmaAllocation> =
            info.allocations.iter().map(|x| x.internal).collect();
        let mut context = DefragmentationContext {
            internal: unsafe { mem::zeroed() },
            stats: Box::new(unsafe { mem::zeroed() }),
            changed: vec![ash::vk::FALSE; allocations.len()],
        };
        let ffi_info = ffi::VmaDefragmentationInfo2 {
            flags: 0, // Reserved for future use
            allocationCount: info.allocations.len() as u32,
            pAllocations: allocations.as_mut_ptr(),
            pAllocationsChanged: context.changed.as_mut_ptr(),
            poolCount: pools.len() as u32,
            pPools: pools.as_mut_ptr(),
            maxCpuBytesToMove: info.max_cpu_bytes_to_move,
            maxCpuAllocationsToMove: info.max_cpu_allocations_to_move,
            maxGpuBytesToMove: info.max_gpu_bytes_to_move,
            maxGpuAllocationsToMove: info.max_gpu_allocations_to_move,
            commandBuffer: command_buffer.as_raw() as ffi::VkCommandBuffer,
        };
        let result = ffi_to_result(unsafe {
            ffi::vmaDefragmentationBegin(
                self.internal,
                &ffi_info,
                &mut *context.stats,
                &mut context.internal,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok(context),
            _ => Err(Error::vulkan(result)),
        }
    }

    /// Ends defragmentation process.
    ///
    /// Use this function to finish defragmentation started by `Allocator::defragmentation_begin`.
    pub fn defragmentation_end(
        &self,
        context: &mut DefragmentationContext,
    ) -> Result<(DefragmentationStats, Vec<bool>)> {
        let result =
            ffi_to_result(unsafe { ffi::vmaDefragmentationEnd(self.internal, context.internal) });
        let changed: Vec<bool> = context.changed.iter().map(|change| *change == 1).collect();
        match result {
            ash::vk::Result::SUCCESS => Ok((
                DefragmentationStats {
                    bytes_moved: context.stats.bytesMoved as usize,
                    bytes_freed: context.stats.bytesFreed as usize,
                    allocations_moved: context.stats.allocationsMoved,
                    device_memory_blocks_freed: context.stats.deviceMemoryBlocksFreed,
                },
                changed,
            )),
            _ => Err(Error::vulkan(result)),
        }
    }

    /// Compacts memory by moving allocations.
    ///
    /// `allocations` is a slice of allocations that can be moved during this compaction.
    /// `defrag_info` optional configuration parameters.
    /// Returns statistics from the defragmentation, and an associated array to `allocations`
    /// which indicates which allocations were changed (if any).
    ///
    /// Possible error values:
    ///
    /// - `ash::vk::Result::INCOMPLETE` if succeeded but didn't make all possible optimizations because limits specified in
    ///   `defrag_info` have been reached, negative error code in case of error.
    ///
    /// This function works by moving allocations to different places (different
    /// `ash::vk::DeviceMemory` objects and/or different offsets) in order to optimize memory
    /// usage. Only allocations that are in `allocations` slice can be moved. All other
    /// allocations are considered nonmovable in this call. Basic rules:
    ///
    /// - Only allocations made in memory types that have
    ///   `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` and `ash::vk::MemoryPropertyFlags::HOST_COHERENT`
    ///   flags can be compacted. You may pass other allocations but it makes no sense -
    ///   these will never be moved.
    ///
    /// - Custom pools created with `AllocatorPoolCreateFlags::LINEAR_ALGORITHM` or `AllocatorPoolCreateFlags::BUDDY_ALGORITHM` flag are not
    ///   defragmented. Allocations passed to this function that come from such pools are ignored.
    ///
    /// - Allocations created with `AllocationCreateFlags::DEDICATED_MEMORY` or created as dedicated allocations for any
    ///   other reason are also ignored.
    ///
    /// - Both allocations made with or without `AllocationCreateFlags::MAPPED` flag can be compacted. If not persistently
    ///   mapped, memory will be mapped temporarily inside this function if needed.
    ///
    /// - You must not pass same `allocation` object multiple times in `allocations` slice.
    ///
    /// The function also frees empty `ash::vk::DeviceMemory` blocks.
    ///
    /// Warning: This function may be time-consuming, so you shouldn't call it too often
    /// (like after every resource creation/destruction).
    /// You can call it on special occasions (like when reloading a game level or
    /// when you just destroyed a lot of objects). Calling it every frame may be OK, but
    /// you should measure that on your platform.
    #[deprecated(
        since = "0.1.3",
        note = "This is a part of the old interface. It is recommended to use structure `DefragmentationInfo2` and function `Allocator::defragmentation_begin` instead."
    )]
    pub fn defragment(
        &self,
        allocations: &[Allocation],
        defrag_info: Option<&DefragmentationInfo>,
    ) -> Result<(DefragmentationStats, Vec<bool>)> {
        let mut ffi_allocations: Vec<ffi::VmaAllocation> = allocations
            .iter()
            .map(|allocation| allocation.internal)
            .collect();
        let mut ffi_change_list: Vec<ffi::VkBool32> = vec![0; ffi_allocations.len()];
        let ffi_info = match defrag_info {
            Some(info) => ffi::VmaDefragmentationInfo {
                maxBytesToMove: info.max_bytes_to_move as ffi::VkDeviceSize,
                maxAllocationsToMove: info.max_allocations_to_move,
            },
            None => ffi::VmaDefragmentationInfo {
                maxBytesToMove: ash::vk::WHOLE_SIZE,
                maxAllocationsToMove: std::u32::MAX,
            },
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
                        bytes_moved: ffi_stats.bytesMoved as usize,
                        bytes_freed: ffi_stats.bytesFreed as usize,
                        allocations_moved: ffi_stats.allocationsMoved,
                        device_memory_blocks_freed: ffi_stats.deviceMemoryBlocksFreed,
                    },
                    change_list,
                ))
            }
            _ => Err(Error::vulkan(result)),
        }
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
    pub fn bind_buffer_memory(
        &self,
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
    pub fn bind_image_memory(&self, image: ash::vk::Image, allocation: &Allocation) -> Result<()> {
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
    pub fn create_buffer(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<(ash::vk::Buffer, Allocation, AllocationInfo)> {
        let buffer_create_info = unsafe {
            mem::transmute::<ash::vk::BufferCreateInfo, ffi::VkBufferCreateInfo>(*buffer_info)
        };
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut buffer: ffi::VkBuffer = unsafe { mem::zeroed() };
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let mut allocation_info: AllocationInfo = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaCreateBuffer(
                self.internal,
                &buffer_create_info,
                &allocation_create_info,
                &mut buffer,
                &mut allocation.internal,
                &mut allocation_info.internal,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok((
                ash::vk::Buffer::from_raw(buffer as u64),
                allocation,
                allocation_info,
            )),
            _ => Err(Error::vulkan(result)),
        }
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
    pub fn destroy_buffer(&self, buffer: ash::vk::Buffer, allocation: &Allocation) {
        unsafe {
            ffi::vmaDestroyBuffer(
                self.internal,
                buffer.as_raw() as ffi::VkBuffer,
                allocation.internal,
            );
        }
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
    pub fn create_image(
        &self,
        image_info: &ash::vk::ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<(ash::vk::Image, Allocation, AllocationInfo)> {
        let image_create_info = unsafe {
            mem::transmute::<ash::vk::ImageCreateInfo, ffi::VkImageCreateInfo>(*image_info)
        };
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut image: ffi::VkImage = unsafe { mem::zeroed() };
        let mut allocation: Allocation = unsafe { mem::zeroed() };
        let mut allocation_info: AllocationInfo = unsafe { mem::zeroed() };
        let result = ffi_to_result(unsafe {
            ffi::vmaCreateImage(
                self.internal,
                &image_create_info,
                &allocation_create_info,
                &mut image,
                &mut allocation.internal,
                &mut allocation_info.internal,
            )
        });
        match result {
            ash::vk::Result::SUCCESS => Ok((
                ash::vk::Image::from_raw(image as u64),
                allocation,
                allocation_info,
            )),
            _ => Err(Error::vulkan(result)),
        }
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
    pub fn destroy_image(&self, image: ash::vk::Image, allocation: &Allocation) {
        unsafe {
            ffi::vmaDestroyImage(
                self.internal,
                image.as_raw() as ffi::VkImage,
                allocation.internal,
            );
        }
    }

    /// Destroys the internal allocator instance. After this has been called,
    /// no other functions may be called. Useful for ensuring a specific destruction
    /// order (for example, if an Allocator is a member of something that owns the Vulkan
    /// instance and destroys it in its own Drop).
    pub fn destroy(&mut self) {
        if !self.internal.is_null() {
            unsafe {
                ffi::vmaDestroyAllocator(self.internal);
                self.internal = std::ptr::null_mut();
            }
        }
    }
}

/// Custom `Drop` implementation to clean up internal allocation instance
impl Drop for Allocator {
    fn drop(&mut self) {
        self.destroy();
    }
}
