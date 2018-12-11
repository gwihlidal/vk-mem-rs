vk-mem
========

[![vk-mem on travis-ci.com](https://travis-ci.com/gwihlidal/vk-mem-rs.svg?branch=master)](https://travis-ci.com/gwihlidal/vk-mem-rs)
[![Latest version](https://img.shields.io/crates/v/vk-mem.svg)](https://crates.io/crates/vk-mem)
[![Documentation](https://docs.rs/vk-mem/badge.svg)](https://docs.rs/vk-mem)
[![](https://tokei.rs/b1/github/gwihlidal/vk-mem-rs)](https://github.com/gwihlidal/vk-mem-rs)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![APACHE2](https://img.shields.io/badge/license-APACHE2-blue.svg)

This crate provides an FFI layer and idiomatic rust wrappers for the excellent [AMD Vulkan Memory Allocator (VMA)](https://github.com/GPUOpen-LibrariesAndSDKs/VulkanMemoryAllocator) C/C++ library.

- [Documentation](https://docs.rs/vk-mem)
- [Release Notes](https://github.com/gwihlidal/vk-mem-rs/releases)
- [GPU Open Announce](https://gpuopen.com/gaming-product/vulkan-memory-allocator/)
- [GPU Open Update](https://gpuopen.com/vulkan-memory-allocator-2-1/)

For MoltenVK on macOS, you need to have the proper environment variables set. Something like:
```bash
export SDK_PATH=/path/to/vulkansdk-macos-1.1.92.0
export DYLD_LIBRARY_PATH=$SDK_PATH/macOS/lib
export VK_ICD_FILENAMES=$SDK_PATH/macOS/etc/vulkan/icd.d/MoltenVK_icd.json
export VK_LAYER_PATH=$SDK_PATH/macOS/etc/vulkan/explicit_layer.d
cargo test
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Credits and Special Thanks

- [Adam Sawicki](https://github.com/adam-sawicki-amd) (Author of C/C++ library)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

Contributions are always welcome; please look at the [issue tracker](https://github.com/gwihlidal/vk-mem-rs/issues) to see what
known improvements are documented.

## Code of Conduct

Contribution to the vk-mem crate is organized under the terms of the
Contributor Covenant, the maintainer of vk-mem, @gwihlidal, promises to
intervene to uphold that code of conduct.