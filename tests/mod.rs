extern crate ash;
extern crate vk_mem;

use ash::extensions::ext::DebugReport;
use std::os::raw::{c_char, c_void};

fn extension_names() -> Vec<*const i8> {
    vec![DebugReport::name().as_ptr()]
}

unsafe extern "system" fn vulkan_debug_callback(
    _: ash::vk::DebugReportFlagsEXT,
    _: ash::vk::DebugReportObjectTypeEXT,
    _: u64,
    _: usize,
    _: i32,
    _: *const c_char,
    p_message: *const c_char,
    _: *mut c_void,
) -> u32 {
    println!("{:?}", ::std::ffi::CStr::from_ptr(p_message));
    ash::vk::FALSE
}

pub struct TestHarness {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: ash::vk::PhysicalDevice,
    pub debug_callback: ash::vk::DebugReportCallbackEXT,
    pub debug_report_loader: ash::extensions::ext::DebugReport,
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_device(None);
            self.debug_report_loader
                .destroy_debug_report_callback(self.debug_callback, None);
            self.instance.destroy_instance(None);
        }
    }
}
impl TestHarness {
    pub fn new() -> Self {
        let app_name = ::std::ffi::CString::new("vk-mem testing").unwrap();
        let app_info = ash::vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(ash::vk::make_version(1, 0, 0));

        let layer_names = [::std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let extension_names_raw = extension_names();
        let create_info = ash::vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);


        let entry = unsafe { ash::Entry::load().unwrap() };
        let instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Instance creation error")
        };

        let debug_info = ash::vk::DebugReportCallbackCreateInfoEXT::builder()
            .flags(
                ash::vk::DebugReportFlagsEXT::ERROR
                    | ash::vk::DebugReportFlagsEXT::WARNING
                    | ash::vk::DebugReportFlagsEXT::PERFORMANCE_WARNING,
            )
            .pfn_callback(Some(vulkan_debug_callback));

        let debug_report_loader = DebugReport::new(&entry, &instance);
        let debug_callback = unsafe {
            debug_report_loader
                .create_debug_report_callback(&debug_info, None)
                .unwrap()
        };

        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Physical device error")
        };

        let (physical_device, queue_family_index) = unsafe {
            physical_devices
                .iter()
                .map(|physical_device| {
                    instance
                        .get_physical_device_queue_family_properties(*physical_device)
                        .iter()
                        .enumerate()
                        .filter_map(|(index, _)| Some((*physical_device, index)))
                        .nth(0)
                })
                .filter_map(|v| v)
                .nth(0)
                .expect("Couldn't find suitable device.")
        };

        let priorities = [1.0];

        let queue_info = [ash::vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index as u32)
            .queue_priorities(&priorities)
            .build()];

        let device_create_info =
            ash::vk::DeviceCreateInfo::builder().queue_create_infos(&queue_info);

        let device: ash::Device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .unwrap()
        };

        TestHarness {
            entry,
            instance,
            device,
            physical_device,
            debug_report_loader,
            debug_callback,
        }
    }

    pub fn create_allocator(&self) -> vk_mem::Allocator {
        let create_info = vk_mem::AllocatorCreateInfo::new(&self.instance, &self.device, &self.physical_device);
        vk_mem::Allocator::new(create_info).unwrap()
    }
}

#[test]
fn create_harness() {
    let _ = TestHarness::new();
}

#[test]
fn create_allocator() {
    let harness = TestHarness::new();
    let _ = harness.create_allocator();
}

#[test]
fn create_gpu_buffer() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();
    let allocation_info =  vk_mem::AllocationCreateInfo::new().usage(vk_mem::MemoryUsage::GpuOnly);

    unsafe {
        let (buffer, allocation, allocation_info) = allocator
            .create_buffer(
                &ash::vk::BufferCreateInfo::builder()
                    .size(16 * 1024)
                    .usage(
                        ash::vk::BufferUsageFlags::VERTEX_BUFFER
                            | ash::vk::BufferUsageFlags::TRANSFER_DST,
                    )
                    .build(),
                &allocation_info,
            )
            .unwrap();
        assert_eq!(allocation_info.get_mapped_data(), std::ptr::null_mut());
        allocator.destroy_buffer(buffer, allocation);
    }
}

#[test]
fn create_cpu_buffer_preferred() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();
    let allocation_info =  vk_mem::AllocationCreateInfo::new()
        .required_flags(ash::vk::MemoryPropertyFlags::HOST_VISIBLE)
        .preferred_flags(ash::vk::MemoryPropertyFlags::HOST_COHERENT
            | ash::vk::MemoryPropertyFlags::HOST_CACHED)
        .flags(vk_mem::AllocationCreateFlags::MAPPED);
    unsafe {
        let (buffer, allocation, allocation_info) = allocator
            .create_buffer(
                &ash::vk::BufferCreateInfo::builder()
                    .size(16 * 1024)
                    .usage(
                        ash::vk::BufferUsageFlags::VERTEX_BUFFER
                            | ash::vk::BufferUsageFlags::TRANSFER_DST,
                    )
                    .build(),
                &allocation_info,
            )
            .unwrap();
        assert_ne!(allocation_info.get_mapped_data(), std::ptr::null_mut());
        allocator.destroy_buffer(buffer, allocation);
    }
}

#[test]
fn create_gpu_buffer_pool() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();

    let buffer_info = ash::vk::BufferCreateInfo::builder()
        .size(16 * 1024)
        .usage(ash::vk::BufferUsageFlags::UNIFORM_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST)
        .build();

    let mut allocation_info = vk_mem::AllocationCreateInfo::new()
        .required_flags(ash::vk::MemoryPropertyFlags::HOST_VISIBLE)
        .preferred_flags(ash::vk::MemoryPropertyFlags::HOST_COHERENT
            | ash::vk::MemoryPropertyFlags::HOST_CACHED)
        .flags(vk_mem::AllocationCreateFlags::MAPPED);
    unsafe {
        let memory_type_index = allocator
            .find_memory_type_index_for_buffer_info(&buffer_info, &allocation_info)
            .unwrap();

        // Create a pool that can have at most 2 blocks, 128 MiB each.
        let pool_info = vk_mem::PoolCreateInfo::new()
            .memory_type_index(memory_type_index)
            .block_size(128 * 1024 * 1024)
            .max_block_count(2);

        let pool = allocator.create_pool(&pool_info).unwrap();
        allocation_info = allocation_info.pool(pool.clone());

        let (buffer, allocation, allocation_info) = allocator
            .create_buffer(&buffer_info, &allocation_info)
            .unwrap();
        assert_ne!(allocation_info.get_mapped_data(), std::ptr::null_mut());
        allocator.destroy_buffer(buffer, allocation);
        allocator.destroy_pool(pool);
    }
}

#[test]
fn test_gpu_stats() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();
    let allocation_info = vk_mem::AllocationCreateInfo::new()
        .usage(vk_mem::MemoryUsage::GpuOnly);

    unsafe {
        let stats_1 = allocator.calculate_stats().unwrap();
        assert_eq!(stats_1.total.blockCount, 0);
        assert_eq!(stats_1.total.allocationCount, 0);
        assert_eq!(stats_1.total.usedBytes, 0);

        let (buffer, allocation, _allocation_info) = allocator
            .create_buffer(
                &ash::vk::BufferCreateInfo::builder()
                    .size(16 * 1024)
                    .usage(
                        ash::vk::BufferUsageFlags::VERTEX_BUFFER
                            | ash::vk::BufferUsageFlags::TRANSFER_DST,
                    )
                    .build(),
                &allocation_info,
            )
            .unwrap();

        let stats_2 = allocator.calculate_stats().unwrap();
        assert_eq!(stats_2.total.blockCount, 1);
        assert_eq!(stats_2.total.allocationCount, 1);
        assert_eq!(stats_2.total.usedBytes, 16 * 1024);

        allocator.destroy_buffer(buffer, allocation);

        let stats_3 = allocator.calculate_stats().unwrap();
        assert_eq!(stats_3.total.blockCount, 1);
        assert_eq!(stats_3.total.allocationCount, 0);
        assert_eq!(stats_3.total.usedBytes, 0);
    }
}

#[test]
fn test_stats_string() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();

    let allocation_info = vk_mem::AllocationCreateInfo::new()
        .usage(vk_mem::MemoryUsage::GpuOnly);

    unsafe {
        let stats_1 = allocator.build_stats_string(true).unwrap();
        assert!(stats_1.len() > 0);

        let (buffer, allocation, _allocation_info) = allocator
            .create_buffer(
                &ash::vk::BufferCreateInfo::builder()
                    .size(16 * 1024)
                    .usage(
                        ash::vk::BufferUsageFlags::VERTEX_BUFFER
                            | ash::vk::BufferUsageFlags::TRANSFER_DST,
                    )
                    .build(),
                &allocation_info,
            )
            .unwrap();

        let stats_2 = allocator.build_stats_string(true).unwrap();
        assert!(stats_2.len() > 0);
        assert_ne!(stats_1, stats_2);

        allocator.destroy_buffer(buffer, allocation);

        let stats_3 = allocator.build_stats_string(true).unwrap();
        assert!(stats_3.len() > 0);
        assert_ne!(stats_3, stats_1);
        assert_ne!(stats_3, stats_2);
    }
}
