use anyhow::*;
use corrode::{
    api::EngineApi,
    engine::Engine,
    renderer::{render_pass::Pass, Line},
    time::PerformanceTimer,
};
use vulkano::sync::GpuFuture;
use winit::event_loop::EventLoop;

use crate::{
    editor::{Editor, EditorMode},
    gui_state::GuiState,
    matter::{default_matter_definitions, validate_matter_definitions},
    object::{Angle, Position},
    render::{draw_canvas, draw_chunk_debug_info, draw_contours, draw_debug_bounds, draw_grid},
    settings::AppSettings,
    simulation::{log_world_performance, Simulation},
    utils::{read_matter_definitions_file, u32_rgba_to_f32_rgba, CanvasMouseState},
    SIM_CANVAS_SIZE, WORLD_UNIT_SIZE,
};

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum InputAction {
    Pause,
    Step,
    PaintMode,
    PlaceMode,
    DragMode,
    ObjectPaintMode,
    ToggleFullScreen,
}

pub struct App {
    simulation: Option<Simulation>,
    editor: Editor,
    gui_state: GuiState,
    settings: AppSettings,
    is_running_simulation: bool,
    is_step: bool,
    is_debug: bool,
    time_since_last_step: f64,
    time_since_last_perf: f64,
    simulation_timer: PerformanceTimer,
    render_timer: PerformanceTimer,
    frame_timer: PerformanceTimer,
}

impl App {
    pub fn new() -> Result<App> {
        Ok(App {
            simulation: None,
            editor: Editor::new()?,
            gui_state: GuiState::new(),
            settings: AppSettings::new(),
            is_running_simulation: true,
            is_step: false,
            is_debug: false,
            time_since_last_step: 0.0,
            time_since_last_perf: 0.0,
            simulation_timer: PerformanceTimer::new(),
            render_timer: PerformanceTimer::new(),
            frame_timer: PerformanceTimer::new(),
        })
    }

    pub fn should_step(&self) -> bool {
        self.time_since_last_step > (1000.0 / self.settings.sim_fps) as f64
    }

    pub fn should_print_perf(&self) -> bool {
        self.time_since_last_perf > 5000.0 && self.settings.print_performance
    }

    pub fn log_performance(&mut self, api: &EngineApi<InputAction>) {
        info!("Performance:");
        println!(
            "  FPS: {:.3}, dt: {:.3}, render: {:.3}, sim: {:.3}",
            api.time.avg_fps(),
            self.frame_timer.time_average_ms(),
            self.render_timer.time_average_ms(),
            self.simulation_timer.time_average_ms(),
        );
        log_world_performance(&self.simulation.as_ref().unwrap());
    }

    pub fn step(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        self.simulation_timer.start();
        let canvas_mouse_state = CanvasMouseState::new(&api.main_camera, &api.inputs[0]);
        self.simulation
            .as_mut()
            .unwrap()
            .step(api, self.settings, &canvas_mouse_state)?;
        self.simulation_timer.time_it();
        self.time_since_last_step = 0.0;
        Ok(())
    }
}

impl Engine<InputAction> for App {
    fn start<E>(
        &mut self,
        _event_loop: &EventLoop<E>,
        api: &mut EngineApi<InputAction>,
    ) -> Result<()> {
        api.main_camera.zoom_to_fit_canvas(WORLD_UNIT_SIZE);
        let matter_definitions = if let Some(defs) = read_matter_definitions_file() {
            defs
        } else {
            default_matter_definitions()
        };
        validate_matter_definitions(&matter_definitions);
        self.simulation = Some(Simulation::new(
            api.renderer.compute_queue(),
            matter_definitions,
            api.renderer.image_format(),
        )?);
        self.editor
            .register_gui_images(api, self.simulation.as_ref().unwrap());
        self.settings
            .update_based_on_device_info_and_env(&api.renderer);
        api.renderer.toggle_fullscreen();
        Ok(())
    }

    fn update(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        self.editor.update(
            api,
            self.simulation.as_mut().unwrap(),
            &mut self.is_running_simulation,
            &mut self.is_step,
        )?;
        if self.should_step() {
            if self.is_running_simulation {
                self.step(api)?;
            } else if self.is_step {
                self.step(api)?;
                self.is_step = false;
            }
        }
        if self.should_print_perf() {
            self.log_performance(api);
            self.time_since_last_perf = 0.0;
        }
        self.time_since_last_step += api.time.dt();
        self.time_since_last_perf += api.time.dt();
        Ok(())
    }

    fn render<F>(
        &mut self,
        before_future: F,
        api: &mut EngineApi<InputAction>,
    ) -> Result<Box<dyn GpuFuture + 'static>>
    where
        F: GpuFuture + 'static,
    {
        self.render_timer.start();
        let EngineApi {
            ecs_world,
            physics_world,
            main_camera,
            renderer,
            ..
        } = api;
        let simulation = self.simulation.as_ref().unwrap();
        let canvas_mouse_state = CanvasMouseState::new(main_camera, &api.inputs[0]);
        let image_target = renderer.final_image();
        let image_format = renderer.image_format();
        let render_pass = &mut renderer.render_passes.deferred;
        let bg_color = [0.0; 4];
        let mut frame =
            render_pass.frame(bg_color, before_future, image_target.clone(), *main_camera)?;
        let mut after_future = None;
        while let Some(pass) = frame.next_pass()? {
            after_future = match pass {
                Pass::Deferred(mut dp) => {
                    draw_canvas(simulation, &mut dp)?;
                    if self.is_debug {
                        draw_contours(ecs_world, physics_world, simulation, &mut dp)?;
                        draw_grid(simulation, &mut dp, [0.5; 4])?;
                        draw_debug_bounds(simulation, &mut dp, [0.0, 1.0, 0.0, 1.0])?;
                        if self.settings.chunked_simulation {
                            draw_chunk_debug_info(simulation, &mut dp, [0.0, 1.0, 1.0, 1.0], [
                                0.0, 0.0, 1.0, 1.0,
                            ])?;
                        }
                    }
                    if let Some((obj_id, _)) = self.editor.dragger.dragged_object {
                        ecs_world
                            .query_one::<(&Position, &Angle)>(obj_id)
                            .ok()
                            .and_then(|mut query| {
                                let (pos, angle) = query.get().unwrap();
                                let drag_pos =
                                    self.editor.dragger.drag_point(pos.0, angle.0).unwrap();
                                dp.draw_line(Line(drag_pos, canvas_mouse_state.mouse_world_pos, [
                                    1.0, 0.0, 0.0, 1.0,
                                ]))
                                .ok()
                            });
                    }

                    if self.editor.mode == EditorMode::Paint
                        || self.editor.mode == EditorMode::ObjectPaint
                    {
                        let pos = canvas_mouse_state.mouse_world_pos;
                        let radius = 0.5 * self.editor.painter.radius * WORLD_UNIT_SIZE
                            / *SIM_CANVAS_SIZE as f32;
                        let matter_definitions = &simulation.matter_definitions.definitions;
                        let mut color_f32 = if self.editor.mode == EditorMode::Paint {
                            u32_rgba_to_f32_rgba(
                                matter_definitions[self.editor.painter.matter as usize].color,
                            )
                        } else {
                            u32_rgba_to_f32_rgba(
                                matter_definitions[self.editor.placer.object_matter as usize].color,
                            )
                        };
                        color_f32[3] = 0.5;
                        dp.draw_circle(pos, radius, color_f32)?;
                    }

                    if self.editor.mode == EditorMode::ObjectPaint {
                        if self.editor.draw_state.started() {
                            self.editor
                                .draw_in_place_object_image(&mut dp, image_format)?;
                        }
                    }

                    None
                }
                Pass::Finished(af) => Some(af),
            };
        }
        let after_drawing = after_future.unwrap().then_signal_fence_and_flush()?.boxed();
        Ok(after_drawing)
    }

    fn gui_content(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        let App {
            simulation: simulator,
            gui_state,
            is_running_simulation,
            is_debug,
            editor,
            settings,
            ..
        } = self;
        gui_state.layout(
            api,
            simulator.as_mut().unwrap(),
            editor,
            settings,
            *is_running_simulation,
            is_debug,
            self.frame_timer.time_average_ms(),
            self.render_timer.time_average_ms(),
            self.simulation_timer.time_average_ms(),
        );

        Ok(())
    }

    fn end_of_frame(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        // Render timer was started in the beginning of render function, there's basically nothing between
        // end of frame and render...
        self.render_timer.time_it();
        self.frame_timer.push_dt_ms(api.time.dt());
        Ok(())
    }
}
