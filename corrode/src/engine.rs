use core::result::Result::Ok;
use std::hash::Hash;

use anyhow::*;
use egui::epaint;
use vulkano::{device::physical::PhysicalDeviceType, sync::GpuFuture};
use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
};

use crate::{api::EngineApi, input_system::InputButton, renderer::Renderer, time::TimeTracker};

#[derive(Debug, Copy, Clone)]
pub struct DeviceOptions {
    pub device_type: PhysicalDeviceType,
    pub index: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct RenderOptions {
    pub title: &'static str,
    pub window_size: [u32; 2],
    /// Match framerate the framerate of your screen to reduce tearing
    pub v_sync: bool,
    /// Whether gui is drawn. This decides if `gui_content` is ran.
    pub is_gui: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            title: "Corrode-engine app",
            window_size: [1920, 1080],
            v_sync: true,
            is_gui: true,
        }
    }
}

/// The engine wrapper struct for running the engine functions
pub struct Corrode {}

pub struct EngineOptions {
    pub fixed_update_fps: f64,
    pub is_esc_quit: bool,
    pub render_options: RenderOptions,
}

impl Default for EngineOptions {
    fn default() -> Self {
        EngineOptions {
            fixed_update_fps: 60.0,
            is_esc_quit: true,
            render_options: RenderOptions::default(),
        }
    }
}

impl Corrode {
    /// Run the engine application for `engine_state`.
    /// This will start the main loop and run the functions from `Engine`.
    /// A renderer with a window is created which can be accessed through the state functions
    /// `fixed_update_fps` will determine how often `fixed_update` runs
    /// 1. `start`
    /// 2. `on_winit_event`
    /// 3. `update`
    /// 4. `fixed_update` (by default 60 times per second)
    /// 5.  `render` and optionally `gui_content`
    /// 6. `end_of_frame` (if you need something to occur last)
    /// 7. `shutdown`
    pub fn run<S: Engine<I> + 'static, I: Hash + Eq + Copy + 'static>(
        application: S,
        opts: EngineOptions,
        input_mappings: Vec<Vec<(I, InputButton)>>,
    ) -> Result<()> {
        Self::run_with_user_event::<S, (), I>(application, opts, input_mappings)
    }

    /// Same as `run` but with user defined event for winit
    pub fn run_with_user_event<
        S: Engine<I> + 'static,
        E: 'static,
        I: Hash + Eq + Copy + 'static,
    >(
        application: S,
        opts: EngineOptions,
        input_mappings: Vec<Vec<(I, InputButton)>>,
    ) -> Result<()> {
        let event_loop = EventLoop::<E>::with_user_event();
        Self::run_loop::<S, E, I>(event_loop, application, opts, input_mappings)
    }

    fn run_loop<S: Engine<I> + 'static, E: 'static, I: Hash + Eq + Copy + 'static>(
        mut event_loop: EventLoop<E>,
        mut application: S,
        opts: EngineOptions,
        input_mappings: Vec<Vec<(I, InputButton)>>,
    ) -> Result<()> {
        let mut internal_time = TimeTracker::new();
        let mut is_running = true;

        // Create renderer
        let renderer = Renderer::new(&event_loop, opts.render_options)?;
        // Create our context
        let mut root_api = EngineApi::new(input_mappings, renderer)?;
        let api = &mut root_api;
        // Force aspect ratio at start & window size for inputs
        api.main_camera
            .update_aspect_ratio(api.renderer.aspect_ratio());
        let ws = api.renderer.window_size();
        api.inputs
            .iter_mut()
            .for_each(|i| i.update_window_size(ws[0], ws[1]));
        application.start(&event_loop, api)?;
        loop {
            let mut event_err = None;
            event_loop.run_return(|event, _, control_flow| {
                *control_flow = ControlFlow::Wait;
                // Update gui
                api.gui.update(&event);

                // Handle window events
                match &event {
                    Event::WindowEvent {
                        event, ..
                    } => match event {
                        WindowEvent::CloseRequested => is_running = false,
                        WindowEvent::Resized(..) => {
                            api.renderer.resize();
                            api.main_camera
                                .update_aspect_ratio(api.renderer.aspect_ratio());
                        }
                        WindowEvent::ScaleFactorChanged {
                            ..
                        } => {
                            api.renderer.resize();
                            api.main_camera
                                .update_aspect_ratio(api.renderer.aspect_ratio());
                        }
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(keycode),
                                    state,
                                    ..
                                },
                            ..
                        } => match keycode {
                            VirtualKeyCode::Escape => {
                                if opts.is_esc_quit {
                                    is_running = false
                                }
                            }
                            _ => {
                                let _ = state;
                            }
                        },
                        _ => (),
                    },
                    Event::MainEventsCleared => *control_flow = ControlFlow::Exit,
                    _ => (),
                }

                // Pass winit event to app as well
                if let Err(error) = application.on_winit_event(&event, api) {
                    event_err = Some(error);
                }
                // Update input mappings with winit event
                api.inputs.iter_mut().for_each(|i| i.on_event(&event));
            });
            if let Some(err) = event_err {
                bail!(err);
            }
            if !is_running {
                break;
            }
            application.update(api)?;
            // Update fixed 60fps
            if internal_time.dt_sum_fixed() >= 1000.0 / opts.fixed_update_fps {
                application.fixed_update(api)?;
                internal_time.reset_fixed();
                api.time.reset_fixed();
            }
            // Render
            Corrode::render(&mut application, api, opts.render_options)?;
            // Reset inputs state after frame
            api.inputs.iter_mut().for_each(|i| i.reset());

            internal_time.update();
            api.time.update();
            // Run end of frame
            application.end_of_frame(api)?;
        }
        application.shutdown(api)?;
        Ok(())
    }

    /// Render using `draw_passes_fn` for world rendering (on camera views)
    /// and `gui_pass_fn` for gui render on window
    fn render<S: Engine<I> + 'static, I: Hash + Eq + Copy + 'static>(
        app: &mut S,
        api: &mut EngineApi<I>,
        opts: RenderOptions,
    ) -> Result<()> {
        // Start frame
        let before_pipeline_future = match api.renderer.start_frame() {
            Err(_e) => return Ok(()),
            Ok(future) => future,
        };
        // Render using user defined render pass(es).
        // Add gui.
        let after_future = {
            let after_pipeline_future = app.render(before_pipeline_future, api)?;
            if opts.is_gui {
                api.gui.begin_frame();
                app.gui_content(api)?;
                api.gui
                    .draw_on_image(after_pipeline_future, api.renderer.final_image())
            } else {
                after_pipeline_future
            }
        };
        // Finish
        api.renderer.finish_frame(after_future);
        Ok(())
    }

    pub fn simple_egui_fps_ui<I: Hash + Eq + Copy + 'static>(api: &mut EngineApi<I>) -> Result<()> {
        let EngineApi {
            gui,
            time,
            ..
        } = api;
        let ctx = gui.context();
        let mut fonts = epaint::text::FontDefinitions::default();
        fonts.family_and_size.insert(
            epaint::text::TextStyle::Body,
            (epaint::text::FontFamily::Proportional, 20.0),
        );
        ctx.set_fonts(fonts);
        egui::Area::new("fps")
            .fixed_pos(egui::pos2(10.0, 10.0))
            .show(&ctx, |ui| {
                ui.label(format!("{:.2}", time.avg_fps()));
            });
        Ok(())
    }
}

/// Engine state trait implementing all stages of a main loop for the engine
/// You can add some App state under the `self` if needed outside `World` or `Resources`
pub trait Engine<I: Hash + Eq + Copy + 'static> {
    /// Run at start
    /// Here you can e.g. create event loop proxy
    fn start<E>(&mut self, _event_loop: &EventLoop<E>, _api: &mut EngineApi<I>) -> Result<()> {
        Ok(())
    }
    /// Run on each event received from winit
    fn on_winit_event<E>(&mut self, _event: &Event<E>, _api: &mut EngineApi<I>) -> Result<()> {
        Ok(())
    }
    /// Run each frame
    fn update(&mut self, _api: &mut EngineApi<I>) -> Result<()> {
        Ok(())
    }
    /// Run each frame at fixed interval
    fn fixed_update(&mut self, _api: &mut EngineApi<I>) -> Result<()> {
        Ok(())
    }
    /// Fill your render pipeline here. This must return the Vulkano future representing the point
    /// when your rendering finishes. `before_future` represents the end of last frame.
    fn render<F>(
        &mut self,
        before_future: F,
        _api: &mut EngineApi<I>,
    ) -> Result<Box<dyn GpuFuture + 'static>>
    where
        F: GpuFuture + 'static,
    {
        Ok(before_future.boxed())
    }
    /// Run each frame after everyting else
    fn end_of_frame(&mut self, _api: &mut EngineApi<I>) -> Result<()> {
        Ok(())
    }
    /// Run at shutdown
    fn shutdown(&mut self, _api: &mut EngineApi<I>) -> Result<()> {
        Ok(())
    }
    /// Immediate mode gui. Fill this with egui calls. (Your gui state).
    fn gui_content(&mut self, _api: &mut EngineApi<I>) -> Result<()> {
        Ok(())
    }
}
