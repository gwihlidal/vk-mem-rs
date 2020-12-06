# Changes

## 0.2.3 (Unreleased)

* Removed `Result` return values from functions that always returned `Ok(())`

## 0.2.2 (2020-03-28)

* Fixed Windows bindgen regression.

## 0.2.1 (2020-03-25)

* Added `Eq`, `Hash` and further derives to enum `MemoryUsage`.
* Added `Copy` to `Allocation`.
* Updated VMA vendoring to commit hash `e73e988dafba80633cd9d8d1abb1ae1af0439bae`.
* Updated dependencies and bindings.

## 0.2.0 (2019-11-16)

* Updated VMA vendoring to commit hash `a020fb81cb67b376fb33228475f22d0d9c29f9fd`.
* Implemented `vk_mem::Allocation::null()` for symmetry with vk::Image::null().

## 0.1.9 (2019-10-29)

* Removed unnecessary mut specifiers.
* Implemented `std::error::Error` for `vk_mem::Error`.
* Disabled usage of `failure` by default.
* Updated VMA vendoring to commit hash `6ac1d3a4b732f50aef3a884ef7020cce53007065`.
* Bumped all dependencies to latest versions.
* Removed clone from `Allocator`, as it was unsafe and invalid.

## 0.1.8 (2019-07-14)

* Allow the failure crate to be disabled through a feature toggle.
* Removed the parallel feature for cc.
* Removed max ash version (only require >= minimum).
* Added a way to cleanup Allocator without dropping it.
* Added a note to `create_image` describing VMA panic behavior in some circumstances.
* Updated VMA vendoring to commit hash `195016b0348038537dbd73d45d6ccaf795bfb367`.
* Regenerated bindings and added function pointer wrapping for `bind_buffer_memory2` and `bind_image_memory2`.

## 0.1.7 (2019-04-29)

* Removed max ash version from dependencies.

## 0.1.6 (2019-03-25)

* Fixed AllocationCreateInfo Default implementation.
* Added windows-gnu to build.rs, and add cross-compilation information to README.md.

## 0.1.5 (2019-03-12)

* Support both ash 0.27.1 and 0 0.28.0.
* Updated vendor to latest version of VMA (fixes, optimizations).
* Added CREATE_DONT_BIND allocation create flag.

## 0.1.4 (2019-03-05)

* Added Sync+Send to Allocation and AllocationInfo.
* Bumped ash and failure deps to latest, updated tests to comply with latest ash.
* Removed unnecessary heap allocation.

## 0.1.3 (2018-12-17)

**Updated to AMD VMA 2.2 release!**

Notable new features: defragmentation of GPU memory, buddy algorithm, convenience functions for sparse binding.

Major changes:

* New, more powerful defragmentation:
  * `DefragmentationInfo2`
  * `Allocator::defragmentation_begin`
  * `Allocator::defragmentation_end`
* Added support for defragmentation of GPU memory.
* Defragmentation of CPU memory now uses `memmove` internally, so it can move data to overlapping regions.
* Defragmentation of CPU memory is now available for memory types that are `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` but not `ash::vk::MemoryPropertyFlags::HOST_COHERENT`.
* Major internal changes in defragmentation algorithm.
* Old interface (structure `DefragmentationInfo`, function `Allocator::defragment`) is now deprecated.
* Added buddy algorithm, available for custom pools:
  * `AllocatorPoolCreateFlags::BUDDY_ALGORITHM`
* Added convenience functions for multiple allocations and deallocations at once, intended for sparse binding resources:
  * `Allocator::allocate_memory_pages`
  * `Allocator::free_memory_pages`
* Added function that tries to resize existing allocation in place:
  * `Allocator::resize_allocation`
* Added flags for allocation strategy
  * New flags:
    * `AllocationCreateFlags::STRATEGY_BEST_FIT`
    * `AllocationCreateFlags::STRATEGY_WORST_FIT`
    * `AllocationCreateFlags::STRATEGY_FIRST_FIT`
  * Their aliases:
    * `AllocationCreateFlags::STRATEGY_MIN_MEMORY`
    * `AllocationCreateFlags::STRATEGY_MIN_TIME`
    * `AllocationCreateFlags::STRATEGY_MIN_FRAGMENTATION`

Minor changes:

* Changed behavior of allocation functions to return `ash::vk::Result::ERROR_VALIDATION_FAILED_EXT` when trying to allocate memory of size 0, create buffer with size 0, or image with one of the dimensions 0.
* Internal optimization: using read-write mutex on some platforms.
* Many additions and fixes in documentation. Many compatibility fixes for various compilers. Other internal bugfixes, optimizations, refactoring, added more internal validation...

## 0.1.2 (2018-12-11)

* Minor documentation tweak.

## 0.1.1 (2018-12-11)

* Major refactors.
* Full documentation pass.

## 0.1.0 (2018-12-11)

* First release.
