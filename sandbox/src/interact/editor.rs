use std::collections::BTreeMap;

use anyhow::*;
use cgmath::Vector2;
use corrode::{
    api::{physics_entity_at_pos, remove_physics_entity, EngineApi},
    input_system::{
        InputButton::{MouseLeft, MouseMiddle, MouseRight},
        State::{Activated, Deactivated, Held},
    },
    renderer::{create_device_image_with_usage, render_pass::DrawPass},
};
use egui::TextureId;
use rand::Rng;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBuffer},
    format::Format,
    image::ImageUsage,
    sync::GpuFuture,
};

use crate::{
    app::InputAction,
    interact::{
        dragger::EditorDragger,
        painter::EditorPainter,
        placer::{get_object_image_files, EditorPlacer},
        saver::EditorSaveLoader,
        CanvasDrawState, DrawTransition,
    },
    matter::{MatterDefinition, MATTER_SAND, MATTER_WOOD},
    sim::{world_pos_to_canvas_pos, Simulation},
    utils::get_map_directory_names,
    CELL_UNIT_SIZE,
};

/// Radius of the brush. 0.5 for one pixel
const BRUSH_RADIUS: f32 = 4.0;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum EditorMode {
    Paint,
    Place,
    ObjectPaint,
    Drag,
}

pub struct Editor {
    pub mode: EditorMode,
    pub draw_state: CanvasDrawState,

    pub matter_texture_ids: BTreeMap<u32, TextureId>,

    pub painter: EditorPainter,
    pub dragger: EditorDragger,
    pub placer: EditorPlacer,
    pub saver: EditorSaveLoader,
}

impl Editor {
    pub fn new() -> Result<Editor> {
        let obj_images = get_object_image_files()?;
        let map_file_names = get_map_directory_names()?;
        Ok(Editor {
            mode: EditorMode::Paint,
            draw_state: CanvasDrawState::new(),

            matter_texture_ids: BTreeMap::new(),

            painter: EditorPainter {
                matter: MATTER_SAND,
                radius: BRUSH_RADIUS,
                is_square: false,
            },
            dragger: EditorDragger {
                dragged_object: None,
            },
            placer: EditorPlacer {
                object_matter: MATTER_WOOD,
                place_object: obj_images.keys().next().cloned(),
                obj_image_assets: obj_images,
                object_image_texture_ids: BTreeMap::new(),
                bitmap_image: None,
            },
            saver: EditorSaveLoader {
                map_name: "New".to_string(),
                map_file_names,
            },
        })
    }
}

impl Editor {
    pub fn update_matter_gui_textures(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &Simulation,
    ) {
        for (_key, texture) in self.matter_texture_ids.iter() {
            api.gui.unregister_user_image(*texture);
        }
        self.register_matter_gui_images(api, simulation);
    }

    pub fn register_gui_images(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &Simulation,
    ) {
        self.register_matter_gui_images(api, simulation);
        for (key, val) in self.placer.obj_image_assets.iter() {
            let texture_id = api.gui.register_user_image_from_bytes(
                &val.data,
                (val.width as u64, val.height as u64),
                api.renderer.image_format(),
            );
            self.placer
                .object_image_texture_ids
                .insert(key.clone(), texture_id);
        }
    }

    fn register_matter_gui_images(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &Simulation,
    ) {
        let material_texture_dimensions = (24, 24);
        simulation
            .matter_definitions
            .definitions
            .iter()
            .for_each(|matter| {
                let image_byte_data = gui_texture_rgba_data(matter, material_texture_dimensions);
                let texture_id = api.gui.register_user_image_from_bytes(
                    &image_byte_data,
                    (
                        material_texture_dimensions.0 as u64,
                        material_texture_dimensions.1 as u64,
                    ),
                    api.renderer.image_format(),
                );
                self.matter_texture_ids.insert(matter.id, texture_id);
            });
    }

    pub fn update(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
        is_running: &mut bool,
        is_step: &mut bool,
    ) -> Result<()> {
        self.handle_inputs(api, simulation, is_running, is_step)?;
        if !*is_running {
            return Ok(());
        }
        // Obj dragging...
        if let Some(dragged_obj_data) = self.dragger.dragged_object {
            self.dragger.drag_object(api, &dragged_obj_data);
        }
        Ok(())
    }

    fn handle_inputs(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
        is_running: &mut bool,
        is_step: &mut bool,
    ) -> Result<()> {
        let EngineApi {
            ecs_world,
            physics_world,
            main_camera,
            inputs,
            ..
        } = api;
        let input = &mut inputs[0];
        let camera = main_camera;

        if input.is_action_held(InputAction::PaintMode) {
            self.mode = EditorMode::Paint;
        } else if input.is_action_held(InputAction::PlaceMode) {
            self.mode = EditorMode::Place;
        } else if input.is_action_held(InputAction::DragMode) {
            self.mode = EditorMode::Drag;
        } else if input.is_action_held(InputAction::ObjectPaintMode) {
            self.mode = EditorMode::ObjectPaint;
        }
        if input.is_action_activated(InputAction::ToggleFullScreen) {
            api.renderer.toggle_fullscreen();
        }

        let mouse_world_pos = camera.screen_to_world_pos(input.mouse_position_normalized());
        let mouse_canvas_pos = world_pos_to_canvas_pos(mouse_world_pos)
            .cast::<i32>()
            .unwrap();

        let mut draw_end_state = None;
        // Handle draw state
        if self.mode == EditorMode::Paint || self.mode == EditorMode::ObjectPaint {
            if input.button_state(MouseLeft) == Some(Activated) {
                draw_end_state = self.draw_state.transition(
                    DrawTransition::Start(mouse_canvas_pos, self.painter.radius),
                    self.painter.is_square,
                );
            }
            if input.button_state(MouseLeft) == Some(Held) {
                draw_end_state = self.draw_state.transition(
                    DrawTransition::Draw(mouse_canvas_pos, self.painter.radius),
                    self.painter.is_square,
                );
            }
            if input.button_state(MouseLeft) == Some(Deactivated) {
                draw_end_state = self.draw_state.transition(
                    DrawTransition::End(mouse_canvas_pos, self.painter.radius),
                    self.painter.is_square,
                );
            }
        }

        // Matter painting
        if self.mode == EditorMode::Paint && self.draw_state.started() {
            if self.painter.is_square {
                self.painter
                    .paint_square_line(simulation, &self.draw_state.get_line())?;
            } else {
                self.painter
                    .paint_round_line(simulation, &self.draw_state.get_line())?;
            }
        }

        if self.mode == EditorMode::ObjectPaint {
            if let Some(end_state) = &draw_end_state {
                self.placer.place_painted_object(
                    ecs_world,
                    physics_world,
                    simulation,
                    end_state,
                )?;
            } else if self.draw_state.started() {
                self.placer
                    .update_in_place_paint_object(simulation, &self.draw_state);
            }
        }

        // Object placement
        if self.mode == EditorMode::Place && input.button_state(MouseLeft) == Some(Activated) {
            self.placer
                .place_object(ecs_world, physics_world, simulation, mouse_world_pos)?;
        }

        // Object removal
        if (self.mode == EditorMode::Place || self.mode == EditorMode::ObjectPaint)
            && input.button_state(MouseRight) == Some(Activated)
        {
            if let Some((rb, entity)) = physics_entity_at_pos(physics_world, mouse_world_pos) {
                if rb.is_dynamic() {
                    remove_physics_entity(ecs_world, physics_world, entity);
                }
            }
        }

        // Object dragging
        if self.mode == EditorMode::Drag
            && (input.button_state(MouseLeft) == Some(Activated)
                || input.button_state(MouseLeft) == Some(Held))
        {
            if self.dragger.dragged_object.is_none() {
                self.dragger
                    .set_dragged_object(ecs_world, physics_world, mouse_world_pos);
            }
        } else {
            self.dragger.dragged_object = None;
        }

        // Simulation pausing & unpausing
        if input.is_action_activated(InputAction::Pause) {
            *is_running = !*is_running;
        }
        if input.is_action_activated(InputAction::Step) {
            *is_step = true;
        }

        // Editor movement
        if input.button_state(MouseMiddle) == Some(Activated)
            || input.button_state(MouseMiddle) == Some(Held)
        {
            let delta = input.mouse_delta();
            if delta.x != 0.0 || delta.y != 0.0 {
                camera.translate(Vector2::new(-delta.x, delta.y) * 50.0 / 2000.0);
            }
        }

        let mouse = input.mouse_position_normalized();
        if mouse.x > 0.2 && mouse.x < 0.8 && mouse.y > 0.2 && mouse.y < 0.8 {
            // Editor zoom
            let scroll = input.mouse_scroll();
            let zoom = 1.1;
            if scroll > 0.0 {
                camera.zoom(zoom);
            } else if scroll < 0.0 {
                camera.zoom(1.0 / zoom);
            }
        }
        Ok(())
    }

    pub fn draw_in_place_object_image(
        &self,
        draw_pass: &mut DrawPass,
        format: Format,
    ) -> Result<()> {
        let device = draw_pass.device();
        let bitmap_image = self.placer.bitmap_image.as_ref().unwrap();
        let color_data = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            bitmap_image.data.clone(),
        )?;
        let image = create_device_image_with_usage(
            draw_pass.queue().clone(),
            [bitmap_image.width, bitmap_image.height],
            format,
            ImageUsage {
                sampled: true,
                storage: true,
                transfer_destination: true,
                ..ImageUsage::none()
            },
        )?;
        // Copy data to image
        let mut builder = AutoCommandBufferBuilder::primary(
            device.clone(),
            draw_pass.queue().family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        builder.copy_buffer_to_image(color_data, image.image().clone())?;
        let command_buffer = builder.build()?;
        let finished = command_buffer.execute(draw_pass.queue().clone())?;
        let _fut = finished.then_signal_fence_and_flush()?;

        //---> render
        let world_pos = self.draw_state.pixels_world_pos();
        let world_width = *CELL_UNIT_SIZE * bitmap_image.width as f32 * 0.5;
        let world_height = *CELL_UNIT_SIZE * bitmap_image.height as f32 * 0.5;
        draw_pass.draw_texture(
            world_pos,
            world_width,
            world_height,
            0.0,
            image,
            false,
            true,
        )
    }
}

pub fn gui_texture_rgba_data(matter: &MatterDefinition, dimensions: (usize, usize)) -> Vec<u8> {
    (0..(dimensions.0 * dimensions.1))
        .map(|_| variated_color(matter.color.to_be_bytes()))
        .flatten()
        .collect()
}

pub fn variated_color(color: [u8; 4]) -> [u8; 4] {
    let p = rand::thread_rng().gen::<f32>();
    let r = color[0] as f32 / 255.0;
    let g = color[1] as f32 / 255.0;
    let b = color[2] as f32 / 255.0;
    let variation = -0.1 + 0.2 * p;
    let r = ((r + variation).clamp(0.0, 1.0) * 255.0) as u8;
    let g = ((g + variation).clamp(0.0, 1.0) * 255.0) as u8;
    let b = ((b + variation).clamp(0.0, 1.0) * 255.0) as u8;
    let a = color[3];
    [r, g, b, a]
}
