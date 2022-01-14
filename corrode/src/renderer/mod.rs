pub use camera::*;
pub use cpu_buffers::*;
pub use mesh::*;
pub use renderer::*;
pub use vertices::*;

mod camera;
mod cpu_buffers;
mod mesh;
pub mod pipelines;
pub mod render_pass;
mod renderer;
mod vertices;

#[cfg(test)]
mod render_test_helper {
    use std::sync::Arc;

    use vulkano::{
        device::{physical::PhysicalDevice, Device, DeviceExtensions, Features, Queue},
        instance::{
            debug::{DebugCallback, MessageSeverity, MessageType},
            Instance, InstanceExtensions,
        },
        Version,
    };

    pub fn test_setup() -> (Arc<Device>, Arc<Queue>, DebugCallback) {
        let layers: Vec<&str> = vec![];
        #[cfg(all(target_os = "macos", debug_assertions))]
        let layers = vec!["VK_LAYER_KHRONOS_validation"];
        #[cfg(all(not(target_os = "macos"), debug_assertions))]
        let layers = vec!["VK_LAYER_LUNARG_standard_validation"];
        let _instance = Instance::new(
            None,
            Version::V1_2,
            &InstanceExtensions {
                ext_debug_utils: true,
                ..vulkano_win::required_extensions()
            },
            layers,
        )
        .unwrap();
        let severity = MessageSeverity {
            error: true,
            warning: true,
            information: true,
            verbose: true,
        };

        let ty = MessageType::all();

        let _debug_callback = DebugCallback::new(&_instance, severity, ty, |msg| {
            let severity = if msg.severity.error {
                "error"
            } else if msg.severity.warning {
                "warning"
            } else if msg.severity.information {
                "information"
            } else if msg.severity.verbose {
                "verbose"
            } else {
                panic!("no-impl");
            };

            let ty = if msg.ty.general {
                "general"
            } else if msg.ty.validation {
                "validation"
            } else if msg.ty.performance {
                "performance"
            } else {
                panic!("no-impl");
            };

            println!(
                "{} {} {}: {}",
                msg.layer_prefix.unwrap_or("unknown"),
                ty,
                severity,
                msg.description
            );
        })
        .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        let physical = PhysicalDevice::enumerate(&_instance).next().unwrap();
        let queue_family = physical.queue_families().next().unwrap();
        let (device, mut queues) = Device::new(
            physical,
            &Features::none(),
            &physical.required_extensions().union(&device_extensions),
            [(queue_family, 0.5)].iter().cloned(),
        )
        .unwrap();
        let queue = queues.next().unwrap();
        (device, queue, _debug_callback)
    }
}
