extern crate ash;
extern crate vk_mem;

use ash::extensions::DebugReport;
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
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

fn main() {
    println!("Demo start");

    let app_name = ::std::ffi::CString::new("Vulkan Memory Demo").unwrap();
    let app_info = ash::vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(0)
        .engine_name(&app_name)
        .engine_version(0)
        .api_version(ash::vk_make_version!(1, 0, 0));

    let layer_names = [::std::ffi::CString::new("VK_LAYER_LUNARG_standard_validation").unwrap()];
    let layers_names_raw: Vec<*const i8> = layer_names
        .iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect();

    let extension_names_raw = extension_names();
    let create_info = ash::vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_layer_names(&layers_names_raw)
        .enabled_extension_names(&extension_names_raw);

    let entry = ash::Entry::new().unwrap();
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
    let debug_call_back = unsafe {
        debug_report_loader
            .create_debug_report_callback_ext(&debug_info, None)
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

    let device_create_info = ash::vk::DeviceCreateInfo::builder().queue_create_infos(&queue_info);

    let device: ash::Device = unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .unwrap()
    };

    // DO STUFF
    {
        let create_info = vk_mem::AllocatorCreateInfo {
            physical_device,
            device: device.handle(),
        };

        let mut allocator = vk_mem::Allocator::new(&create_info);

        let (buffer, allocation) = allocator.create_buffer(
            ash::vk::BufferCreateInfo::builder()
                .size(65536)
                .usage(
                    ash::vk::BufferUsageFlags::VERTEX_BUFFER
                        | ash::vk::BufferUsageFlags::TRANSFER_DST,
                )
                .build(),
        );

        //allocator.check_corruption(ash::vk::MemoryPropertyFlags::DEVICE_LOCAL);
        //allocator.check_corruption(ash::vk::MemoryPropertyFlags::all());

        allocator.destroy_buffer(buffer, &allocation);
    }

    unsafe {
        device.device_wait_idle().unwrap();
        device.destroy_device(None);
        debug_report_loader.destroy_debug_report_callback_ext(debug_call_back, None);
        instance.destroy_instance(None);
    }
}
