use corrode::renderer::Renderer;
use vulkano::device::physical::PhysicalDeviceType;

use crate::{INIT_DISPERSION_STEPS, INIT_MOVEMENT_STEPS, SIM_CANVAS_SIZE};

#[derive(Debug, Clone, Copy)]
pub struct AppSettings {
    pub dispersion_steps: u32,
    pub movement_steps: u32,
    pub sim_fps: f32,
    pub print_performance: bool,
    pub chunked_simulation: bool,
}

impl AppSettings {
    pub fn new() -> AppSettings {
        let dispersion_steps = INIT_DISPERSION_STEPS;
        let movement_steps = INIT_MOVEMENT_STEPS;
        let sim_fps = 60.0;
        AppSettings {
            dispersion_steps,
            movement_steps,
            sim_fps,
            print_performance: false,
            chunked_simulation: false,
        }
    }

    pub fn update_based_on_device_info_and_env(&mut self, renderer: &Renderer) {
        let max_mem_gb = renderer.max_mem_gb();
        let device_type = renderer.device_type();
        if device_type != PhysicalDeviceType::DiscreteGpu {
            info!("Reduce default settings (No discrete gpu)");
            self.dispersion_steps = 4;
            self.movement_steps = 1;
            self.sim_fps = 30.0;
        } else if max_mem_gb < 2.0 {
            info!("Reduce default settings (< 2.0 gb gpu mem)");
            self.dispersion_steps = 4;
            self.movement_steps = 2;
        } else if max_mem_gb < 1.0 {
            info!("Reduce default settings (< 1.0 gb gpu mem)");
            self.dispersion_steps = 3;
            self.movement_steps = 1;
        };
        if *SIM_CANVAS_SIZE == 1024 {
            self.dispersion_steps = 4;
            self.movement_steps = 1;
            self.sim_fps = 30.0;
        }
    }
}
