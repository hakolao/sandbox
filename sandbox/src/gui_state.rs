use std::ops::BitAnd;

use cgmath::Vector2;
use corrode::api::{physics_entity_at_pos, EngineApi};
use egui::{Grid, ImageButton, Ui, Vec2};

use crate::{
    app::InputAction,
    interact::{Editor, EditorMode, EditorPlacer},
    matter::{
        Direction, MatterCharacteristic, MatterDefinition, MatterDefinitions, MatterState,
        ALL_CHARACTERISTICS, ALL_DIRECTIONS, MATTER_EMPTY,
    },
    object::{Angle, Position},
    settings::AppSettings,
    sim::{canvas_pos_to_world_pos, Simulation},
    utils::{u32_rgba_to_u8_rgba, u8_rgba_to_u32_rgba, CanvasMouseState},
    SIM_CANVAS_SIZE,
};

fn get_selected_characteristics(
    current_characteristics: MatterCharacteristic,
) -> Vec<(MatterCharacteristic, &'static str, &'static str, bool)> {
    ALL_CHARACTERISTICS
        .into_iter()
        .map(|(char, text, guide)| {
            let is_selected = current_characteristics.bitand(char).bits() != 0;
            (char, text, guide, is_selected)
        })
        .collect()
}

fn get_selected_directions(current_directions: Direction) -> Vec<(Direction, &'static str, bool)> {
    ALL_DIRECTIONS
        .into_iter()
        .map(|(char, text)| {
            let is_selected = current_directions.bitand(char).bits() != 0;
            (char, text, is_selected)
        })
        .collect()
}

pub struct GuiState {
    pub show_guide_view: bool,
    pub show_info_view: bool,
    pub show_edit_view: bool,
    pub show_load_view: bool,
    pub show_settings_view: bool,
    pub show_new_matter_view: bool,
    add_matter: MatterDefinition,
}

impl GuiState {
    pub fn new() -> Self {
        GuiState {
            show_guide_view: false,
            show_info_view: false,
            show_edit_view: true,
            show_load_view: false,
            show_new_matter_view: false,
            show_settings_view: false,
            add_matter: MatterDefinition::zero(),
        }
    }

    pub fn layout(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
        editor: &mut Editor,
        settings: &mut AppSettings,
        is_running_simulation: bool,
        is_debug: &mut bool,
        frame_time: f64,
        render_time: f64,
        sim_time: f64,
    ) {
        egui::TopBottomPanel::top("Test").show(&api.gui.context(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_label(self.show_edit_view, "Editor")
                    .clicked()
                    .then(|| {
                        self.show_edit_view = !self.show_edit_view;
                    });
                ui.selectable_label(self.show_settings_view, "Settings")
                    .clicked()
                    .then(|| {
                        self.show_settings_view = !self.show_settings_view;
                    });
                ui.selectable_label(self.show_new_matter_view, "Edit Matters")
                    .clicked()
                    .then(|| {
                        self.show_new_matter_view = !self.show_new_matter_view;
                    });
                ui.selectable_label(self.show_load_view, "Load / Save Map")
                    .clicked()
                    .then(|| {
                        self.show_load_view = !self.show_load_view;
                    });
                ui.selectable_label(self.show_guide_view, "Guide")
                    .clicked()
                    .then(|| {
                        self.show_guide_view = !self.show_guide_view;
                    });
                ui.selectable_label(self.show_info_view, "Info")
                    .clicked()
                    .then(|| {
                        self.show_info_view = !self.show_info_view;
                    });
            })
        });
        self.add_settings_window(api, simulation, settings, is_debug);
        self.add_editor_window(api, simulation, editor);
        self.add_info_window(
            api,
            simulation,
            is_running_simulation,
            frame_time,
            render_time,
            sim_time,
        );
        self.add_load_save_window(api, simulation, editor, settings);
        self.add_new_matter_window(api, simulation, editor);
        self.add_guide_view(api);
        if *is_debug {
            self.add_query_tooltip(api, simulation);
        }
    }

    pub fn add_new_matter_window(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
        editor: &mut Editor,
    ) {
        let GuiState {
            show_new_matter_view,
            ..
        } = self;
        if let Some(def) = simulation
            .matter_definitions
            .definitions
            .iter()
            .find(|d| d.name == self.add_matter.name)
        {
            self.add_matter.id = def.id;
        } else {
            self.add_matter.id = simulation.matter_definitions.definitions.len() as u32;
        }
        self.add_matter.id = simulation.matter_definitions.definitions.len() as u32;
        let rgba = u32_rgba_to_u8_rgba(self.add_matter.color);
        let mut color = [rgba[0], rgba[1], rgba[2]];
        let color_before = color;
        let selected_characteristics =
            get_selected_characteristics(self.add_matter.characteristics);
        let reactions = self.add_matter.reactions;
        let ctx = api.gui.context();
        egui::Window::new("Edit Matters")
            .open(show_new_matter_view)
            .default_width(200.0)
            .default_height(600.0)
            .vscroll(true)
            .show(&ctx, |ui| {
                ui.group(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.add_matter.name);
                    ui.label("Color");
                    ui.color_edit_button_srgb(&mut color);
                    ui.label("Weight")
                        .on_hover_text("Weight affects fall order in liquids");
                    ui.add(egui::Slider::new(&mut self.add_matter.weight, 0.0..=5.0));
                    egui::ComboBox::from_label("Matter State")
                        .selected_text(format!("{:?}", self.add_matter.state.to_string()))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.add_matter.state,
                                MatterState::Powder,
                                "Powder",
                            );
                            ui.selectable_value(
                                &mut self.add_matter.state,
                                MatterState::Liquid,
                                "Liquid",
                            );
                            ui.selectable_value(
                                &mut self.add_matter.state,
                                MatterState::Solid,
                                "Solid",
                            );
                            ui.selectable_value(
                                &mut self.add_matter.state,
                                MatterState::SolidGravity,
                                "Solid Gravity",
                            );
                            ui.selectable_value(
                                &mut self.add_matter.state,
                                MatterState::Gas,
                                "Gas",
                            );
                            ui.selectable_value(
                                &mut self.add_matter.state,
                                MatterState::Energy,
                                "Energy",
                            );
                        });
                    ui.label("Dispersion");
                    ui.add(egui::Slider::new(&mut self.add_matter.dispersion, 0..=10))
                        .on_hover_text("Spreading speed for liquids or gases");
                    ui.collapsing("Characteristics", |ui| {
                        for (val, text, guide, is_selected) in selected_characteristics.iter() {
                            ui.selectable_label(*is_selected, text)
                                .on_hover_text(guide)
                                .clicked()
                                .then(|| {
                                    if *is_selected {
                                        self.add_matter.characteristics.remove(*val);
                                    } else {
                                        self.add_matter.characteristics.insert(*val);
                                    }
                                });
                        }
                    });
                    ui.collapsing("Reactions", |ui| {
                        for (index, reaction) in reactions.iter().enumerate() {
                            ui.collapsing(format!("{}: Reacts with", index), |ui| {
                                for (val, text, guide, is_selected) in
                                    get_selected_characteristics(reaction.reacts).iter()
                                {
                                    ui.selectable_label(*is_selected, text)
                                        .on_hover_text(guide)
                                        .clicked()
                                        .then(|| {
                                            if *is_selected {
                                                self.add_matter.reactions[index]
                                                    .reacts
                                                    .remove(*val);
                                            } else {
                                                self.add_matter.reactions[index]
                                                    .reacts
                                                    .insert(*val);
                                            }
                                        });
                                }
                            });
                            ui.collapsing(format!("{}: Reacts direction", index), |ui| {
                                for (val, text, is_selected) in
                                    get_selected_directions(reaction.direction).iter()
                                {
                                    ui.selectable_label(*is_selected, text).clicked().then(|| {
                                        if *is_selected {
                                            self.add_matter.reactions[index].direction.remove(*val);
                                        } else {
                                            self.add_matter.reactions[index].direction.insert(*val);
                                        }
                                    });
                                }
                            });
                            ui.add(egui::Slider::new(
                                &mut self.add_matter.reactions[index].probability,
                                0.0..=1.0,
                            ))
                            .on_hover_text("Probability");
                            egui::ComboBox::from_label(format!("{}: Becomes", index))
                                .selected_text(format!(
                                    "{:?}",
                                    simulation.matter_definitions.definitions
                                        [self.add_matter.reactions[index].becomes as usize]
                                        .name
                                ))
                                .show_ui(ui, |ui| {
                                    for (id, definition) in
                                        simulation.matter_definitions.definitions.iter().enumerate()
                                    {
                                        ui.selectable_value(
                                            &mut self.add_matter.reactions[index].becomes,
                                            id as u32,
                                            &definition.name,
                                        );
                                    }
                                });
                            ui.separator();
                        }
                    });
                    ui.separator();
                    if let Some(def) = simulation
                        .matter_definitions
                        .definitions
                        .iter()
                        .find(|d| d.name == self.add_matter.name)
                    {
                        self.add_matter.id = def.id;
                        ui.button(format!("Update {}", self.add_matter.name))
                            .clicked()
                            .then(|| {
                                simulation
                                    .add_matter_to_definitions(self.add_matter.clone())
                                    .unwrap();
                                editor.update_matter_gui_textures(api, simulation);
                            });
                    } else {
                        ui.button("Add").clicked().then(|| {
                            simulation
                                .add_matter_to_definitions(self.add_matter.clone())
                                .unwrap();
                            editor.update_matter_gui_textures(api, simulation);
                        });
                    }
                });
                ui.group(|ui| {
                    add_matter_edit_palette(ui, api, simulation, editor, &mut self.add_matter);
                });
            });
        if color_before != color {
            self.add_matter.color = u8_rgba_to_u32_rgba(color[0], color[1], color[2], 255);
        }
    }

    pub fn add_info_window(
        &mut self,
        api: &EngineApi<InputAction>,
        simulation: &Simulation,
        is_running_simulation: bool,
        frame_time_average: f64,
        render_time_average: f64,
        sim_time_average: f64,
    ) {
        let GuiState {
            show_info_view, ..
        } = self;
        let ctx = api.gui.context();
        egui::Window::new("Info")
            .open(show_info_view)
            .default_width(200.0)
            .show(&ctx, |ui| {
                ui.label("Macro level time averages:");
                ui.separator();
                ui.label(format!("FPS: {:.3}", api.time.avg_fps()));
                ui.label(format!("dt: {:.3}", frame_time_average));
                ui.label(format!("Render: {:.3}", render_time_average));
                ui.label(format!("Simulation: {:.3}", sim_time_average));
                ui.separator();
                ui.label("Sim breakdown:");
                ui.separator();
                ui.label(format!(
                    "Obj write to grid: {:.3}",
                    simulation.obj_write_timer.time_average_ms()
                ));
                ui.label(format!(
                    "CA simulation: {:.3}",
                    simulation.ca_timer.time_average_ms()
                ));
                ui.label(format!(
                    "Obj deformation: {:.3}",
                    simulation.obj_read_timer.time_average_ms()
                ));
                ui.label(format!(
                    "Boundary creation: {:.3}",
                    simulation.boundary_timer.time_average_ms()
                ));
                ui.label(format!(
                    "Physics: {:.3}",
                    simulation.physics_timer.time_average_ms()
                ));
                ui.separator();
                ui.label(format!("Running: {}", is_running_simulation));
                ui.label(format!("Num entities : {}", api.ecs_world.len()));
            });
    }

    pub fn add_load_save_window(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
        editor: &mut Editor,
        settings: &AppSettings,
    ) {
        let GuiState {
            show_load_view, ..
        } = self;
        let ctx = api.gui.context();
        egui::Window::new("Maps")
            .open(show_load_view)
            .default_width(100.0)
            .show(&ctx, |ui| {
                ui.label("Load map");
                ui.separator();
                add_loadable_maps(ui, editor, api, simulation);
                ui.label("New map");
                ui.separator();
                ui.button("New")
                    .clicked()
                    .then(|| editor.saver.new_map(api, simulation));
                ui.label("Save map");
                ui.separator();
                ui.text_edit_singleline(&mut editor.saver.map_name);
                ui.button("Save")
                    .clicked()
                    .then(|| editor.saver.save_map(api, simulation, settings));
            });
    }

    pub fn add_guide_view(&mut self, api: &mut EngineApi<InputAction>) {
        let GuiState {
            show_guide_view, ..
        } = self;
        let ctx = api.gui.context();
        egui::Window::new("Guide")
            .open(show_guide_view)
            .default_width(200.0)
            .show(&ctx, |ui| {
                ui.label("Keys:");
                ui.separator();
                ui.label("Key 1: Paint matter mode");
                ui.label("Key 2: Place object mode");
                ui.label("Key 3: Paint object mode");
                ui.label("Key 4: Drag object mode");
                ui.label("Key F: Toggle Fullscreen");
                ui.label("Key Space: Pause Simulation");
                ui.label("Key Enter: Step Simulation");
                ui.separator();
                ui.label("Mouse:");
                ui.separator();
                ui.label("Mouse Left: Paint / Place / Drag object");
                ui.label("Mouse Right: Remove object (in place / paint object mode)");
                ui.label("Mouse Middle: Move camera)");
                ui.label("Mouse Scroll: Zoom)");
                ui.separator();
                ui.label("Matters");
                ui.separator();
                ui.label(
                    "Use Edit matter window to update matters. Saving will save them to \
                     assets/matter_definitions.json which is read by default",
                );
                ui.separator();
                ui.label("Launch app with LARGE=1 to test 1024 sized grid (experimental & slow)");
            });
    }

    pub fn add_settings_window(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &mut Simulation,
        settings: &mut AppSettings,
        is_debug: &mut bool,
    ) {
        let GuiState {
            show_settings_view,
            ..
        } = self;
        let ctx = api.gui.context();
        egui::Window::new("Settings")
            .open(show_settings_view)
            .default_width(250.0)
            .show(&ctx, |ui| {
                ui.checkbox(is_debug, "Debug")
                    .on_hover_text("Render debug information like physics colliders & grid");
                ui.separator();
                ui.label("Performance Settings");
                ui.group(|ui| {
                    ui.label(&format!("Sim size: {}", *SIM_CANVAS_SIZE));
                    ui.label("Device");
                    ui.label(&format!("Name: {:?}", api.renderer.device_name()));
                    ui.label(&format!("Type: {:?}", api.renderer.device_type()));
                    ui.label(&format!("Mem: {:.2} gb", api.renderer.max_mem_gb()));
                    ui.separator();
                    ui.label("Simulation fps");
                    ui.selectable_value(&mut settings.sim_fps, 30.0, "30.0")
                        .on_hover_text("Simulation is run 30 times per second");
                    ui.selectable_value(&mut settings.sim_fps, 60.0, "60.0")
                        .on_hover_text("Simulation is run 60 times per second");
                    ui.separator();
                    ui.label("Simulation dispersion steps");
                    ui.add(egui::Slider::new(&mut settings.dispersion_steps, 1..=10))
                        .on_hover_text(
                            "How fast the compute shader disperses cellular automata liquids \
                             (Higher means more calculation)",
                        );
                    ui.separator();
                    ui.label("Simulation movement steps");
                    ui.add(egui::Slider::new(&mut settings.movement_steps, 1..=3))
                        .on_hover_text(
                            "How many movement steps is taken for falling, rising & sliding \
                             cellular automata",
                        );
                    ui.separator();
                    ui.checkbox(&mut settings.print_performance, "Print performance")
                        .on_hover_text("Whether performance is printed in terminal");
                });
                ui.separator();
                let is_chunked = settings.chunked_simulation;
                ui.checkbox(&mut settings.chunked_simulation, "Chunked Sim Movement")
                    .on_hover_text(
                        "Whether simulation is allowed to move (Sim area still stays the same, \
                         but position varies). Chunks are loaded and unloaded at run time to gpu \
                         memory. Saving in this mode will save all chunks to maps",
                    );
                // Reset simulation camera position to zero if we toggle off chunked simulation
                if is_chunked != settings.chunked_simulation && !settings.chunked_simulation {
                    simulation.camera_pos = Vector2::new(0.0, 0.0);
                }
            });
    }

    pub fn add_editor_window(
        &mut self,
        api: &mut EngineApi<InputAction>,
        simulation: &Simulation,
        editor: &mut Editor,
    ) {
        let GuiState {
            show_edit_view, ..
        } = self;
        let ctx = api.gui.context();
        egui::Window::new("Editor")
            .open(show_edit_view)
            .vscroll(true)
            .default_width(200.0)
            .default_height(800.0)
            .show(&ctx, |ui| {
                ui.label(format!("Mode {:?}", editor.mode));
                ui.selectable_value(&mut editor.mode, EditorMode::Paint, "Paint Matter (1)")
                    .on_hover_text("Paint matter with mouse");
                ui.selectable_value(&mut editor.mode, EditorMode::Place, "Place Object (2)")
                    .on_hover_text("Place objects at mouse position");
                ui.selectable_value(
                    &mut editor.mode,
                    EditorMode::ObjectPaint,
                    "Paint Object (3)",
                )
                .on_hover_text("Paint custom objects at mouse position");
                ui.selectable_value(&mut editor.mode, EditorMode::Drag, "Drag Object (4)")
                    .on_hover_text("Drag existing objects at mouse position");
                if editor.mode == EditorMode::Paint {
                    ui.label("Brush Radius");
                    ui.add(egui::Slider::new(&mut editor.painter.radius, 0.5..=30.0));
                    ui.checkbox(&mut editor.painter.is_square, "Square brush");
                    ui.separator();
                    ui.label(format!(
                        "Matter ({})",
                        &simulation.matter_definitions.definitions[editor.painter.matter as usize]
                            .name
                    ));
                    ui.separator();
                    add_matter_palette(ui, simulation, editor);
                } else if editor.mode == EditorMode::Place {
                    ui.separator();
                    if let Some(object) = &editor.placer.place_object {
                        ui.label(format!("Object ({})", object));
                        add_object_palette(ui, editor);
                    } else {
                        ui.label("Object (None)");
                        ui.label("Add .png images to assets/object_images");
                    }
                    ui.separator();
                    ui.label(format!(
                        "Object Matter ({})",
                        &simulation.matter_definitions.definitions
                            [editor.placer.object_matter as usize]
                            .name
                    ));
                    ui.separator();
                    add_object_matter_palette(ui, editor, &simulation.matter_definitions);
                } else if editor.mode == EditorMode::ObjectPaint {
                    ui.label("Brush Radius");
                    ui.add(egui::Slider::new(&mut editor.painter.radius, 0.5..=10.0));
                    ui.checkbox(&mut editor.painter.is_square, "Is square");
                    ui.label(format!(
                        "Object Matter ({})",
                        &simulation.matter_definitions.definitions
                            [editor.placer.object_matter as usize]
                            .name
                    ));
                    add_object_matter_palette(ui, editor, &simulation.matter_definitions);
                } else {
                    ui.label("Move object by dragging");
                }
            });
    }

    pub fn add_query_tooltip(&mut self, api: &EngineApi<InputAction>, simulation: &Simulation) {
        let matter_data = &simulation.matter_definitions.definitions;
        let ctx = api.gui.context();
        let canvas_mouse_state = CanvasMouseState::new(&api.main_camera, &api.inputs[0]);
        if let Some(matter) = simulation
            .query_matter(canvas_mouse_state.mouse_on_canvas)
            .unwrap()
        {
            let matter = &matter_data[matter as usize];
            let obj = physics_entity_at_pos(
                &api.physics_world,
                canvas_pos_to_world_pos(canvas_mouse_state.mouse_on_canvas),
            );
            let obj_data = if let Some(o) = obj {
                if o.0.is_dynamic() {
                    let pos = *api.ecs_world.get::<Position>(o.1).unwrap();
                    let angle = *api.ecs_world.get::<Angle>(o.1).unwrap();
                    Some((o, pos.0, angle.0))
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(((_rb, id), obj_pos, obj_angle)) = obj_data {
                let matter: Option<&String> = simulation
                    .object_pixel_query
                    .as_ref()
                    .map(|obj_matter| &matter_data[obj_matter.0 as usize].name);
                egui::containers::show_tooltip_at_pointer(
                    &ctx,
                    egui::Id::new("Hover tooltip"),
                    |ui| {
                        ui.label(format!(
                            "Obj: (\n Pixel: {:?}\n Id: {:?}\n Pos: {:?}\n Angle: {} rad\n)\n{}",
                            matter, id, obj_pos, obj_angle, canvas_mouse_state,
                        ));
                    },
                );
            } else {
                egui::containers::show_tooltip_at_pointer(
                    &ctx,
                    egui::Id::new("Hover tooltip"),
                    |ui| {
                        ui.label(format!(
                            "Matter: ({}, {})\n{}",
                            matter.name, matter.state, canvas_mouse_state,
                        ));
                    },
                );
            }
        }
    }
}

fn add_matter_palette(ui: &mut Ui, simulation: &Simulation, editor: &mut Editor) {
    let button_size = Vec2::new(24.0, 24.0);
    let grouped_matters = get_grouped_matters(&simulation.matter_definitions.definitions);
    let num_cols = 4;
    for m_group in grouped_matters.iter() {
        let state = m_group[0].state;
        ui.label(state.to_string());
        ui.separator();
        Grid::new(state.to_string()).show(ui, |ui| {
            let mut cols = 0;
            for m in m_group.iter() {
                let texture_id = editor
                    .matter_texture_ids
                    .get(&m.id)
                    .expect("Material texture id not found");
                let btn = ImageButton::new(*texture_id, button_size);
                ui.horizontal(|ui| {
                    if ui.add(btn).on_hover_text(&m.name).clicked() {
                        editor.painter.matter = m.id;
                    }
                    ui.label(&m.name);
                });
                cols += 1;
                if cols == num_cols {
                    ui.end_row();
                    cols = 0;
                }
            }
        });
    }
}

fn add_matter_edit_palette(
    ui: &mut Ui,
    api: &mut EngineApi<InputAction>,
    simulation: &mut Simulation,
    editor: &mut Editor,
    add_matter: &mut MatterDefinition,
) {
    let img_size = Vec2::new(24.0, 24.0);
    let matters: Vec<MatterDefinition> = simulation.matter_definitions.definitions.clone();
    ui.horizontal(|ui| {
        Grid::new("Edit matter palette").show(ui, |ui| {
            for m in matters.iter() {
                let texture_id = editor
                    .matter_texture_ids
                    .get(&m.id)
                    .expect("Material texture id not found");
                let img = egui::Image::new(*texture_id, img_size);
                ui.add(img);
                ui.label(&m.name);
                ui.button("🖊").clicked().then(|| {
                    *add_matter = m.clone();
                });
                if m.id != MATTER_EMPTY {
                    ui.button("❌").clicked().then(|| {
                        simulation.remove_matter_definition(m.id).unwrap();
                        editor.update_matter_gui_textures(api, simulation);
                    });
                }
                ui.end_row();
            }
        });
    });

    ui.separator();
    ui.button("Save Matters").clicked().then(|| {
        simulation.save_matter_definitions();
    });
}

fn add_object_palette(ui: &mut Ui, editor: &mut Editor) {
    let EditorPlacer {
        place_object: object,
        object_image_texture_ids,
        ..
    } = &mut editor.placer;
    let button_size = Vec2::new(48.0, 48.0);
    let num_cols = 2;
    Grid::new("Objects").show(ui, |ui| {
        let mut cols = 0;
        for (key, val) in object_image_texture_ids.iter() {
            let btn = ImageButton::new(*val, button_size);
            ui.horizontal(|ui| {
                if ui.add(btn).on_hover_text(key).clicked() {
                    *object = Some(key.clone());
                }
                ui.label(key);
            });
            cols += 1;
            if cols == num_cols {
                ui.end_row();
                cols = 0;
            }
        }
    });
}

fn add_loadable_maps(
    ui: &mut Ui,
    editor: &mut Editor,
    api: &mut EngineApi<InputAction>,
    simulation: &mut Simulation,
) {
    let file_names = editor.saver.map_file_names.clone();
    for map in file_names.iter() {
        ui.horizontal(|ui| {
            ui.button(map).clicked().then(|| {
                editor.saver.load_map(api, simulation, map).unwrap();
                api.main_camera.translate(-api.main_camera.pos());
            });
            ui.button("❌")
                .clicked()
                .then(|| editor.saver.delete_map(map));
        });
        ui.end_row();
    }
}

fn add_object_matter_palette(ui: &mut Ui, editor: &mut Editor, matter_data: &MatterDefinitions) {
    let button_size = Vec2::new(24.0, 24.0);
    let matters: Vec<MatterDefinition> = matter_data
        .definitions
        .iter()
        .filter(|m| m.state == MatterState::Solid || m.state == MatterState::SolidGravity)
        .cloned()
        .collect();
    let matters = get_grouped_matters(&matters);

    let num_cols = 2;
    for m_group in matters.iter() {
        let state = m_group[0].state;
        ui.label(state.to_string());
        ui.separator();
        Grid::new("Object matters").show(ui, |ui| {
            let mut cols = 0;
            for m in m_group.iter() {
                let texture_id = editor
                    .matter_texture_ids
                    .get(&m.id)
                    .expect("Material texture id not found");
                let btn = ImageButton::new(*texture_id, button_size);
                ui.horizontal(|ui| {
                    if ui.add(btn).on_hover_text(&m.name).clicked() {
                        editor.placer.object_matter = m.id;
                    }
                    ui.label(&m.name);
                });
                cols += 1;
                if cols == num_cols {
                    ui.end_row();
                    cols = 0;
                }
            }
        });
    }
}

fn get_grouped_matters(matters: &[MatterDefinition]) -> Vec<Vec<MatterDefinition>> {
    let mut matters: Vec<MatterDefinition> = matters.to_vec();
    matters.sort_unstable_by_key(|m| m.state);
    let mut grouped_matters = vec![];
    let mut is_next_group = true;
    let mut last_state = matters.first().unwrap().state;
    for matter in matters.iter() {
        if matter.state != last_state {
            is_next_group = true;
        }
        if is_next_group {
            grouped_matters.push(vec![matter.clone()]);
            last_state = matter.state;
            is_next_group = false;
        } else {
            grouped_matters.last_mut().unwrap().push(matter.clone());
        }
    }
    grouped_matters
}
