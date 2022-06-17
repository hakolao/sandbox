# Sandbox project Documentation

Some of the code can get a bit complicated, but this documentation should explain how the app works better.

To dive into the cellular automata simulation and how it's done by the Sandbox, check out my [tutorial](https://www.okkohakola.com/posts/sandfall_tutorial/).

## Corrode
Corrode is a [game engine framework](../corrode/README.md) allowing a separation between application logic and the engine logic.

Currently, it provides utilities for rendering, physics, logging, time, inputs and so on. It exposes a simple API to be used with your application.
For a more extensive project, you'd also need audio, asset handling, asynchronous messaging and perhaps more graphical pipelines.
But this works for now.

All you need to do is to input options, input mappings and define the logic of your app inside the `Engine` functions.
See `sandbox/main.rs` for an example usage.

```rust
fn main() -> Result<()> {
    #[cfg(debug_assertions)]
        initialize_logger(LevelFilter::Debug)?;
    #[cfg(not(debug_assertions))]
        initialize_logger(LevelFilter::Info)?;

    Corrode::run(
        SandboxApp::new()?,
        EngineOptions {
            render_options: RenderOptions {
                v_sync: false,
                title: "Sandbox",
                ..RenderOptions::default()
            },
            ..EngineOptions::default()
        },
        vec![vec![
            (InputAction::Pause, Key(VirtualKeyCode::Space)),
            (InputAction::Step, Key(VirtualKeyCode::Return)),
            (InputAction::PaintMode, Key(VirtualKeyCode::Key1)),
            (InputAction::PlaceMode, Key(VirtualKeyCode::Key2)),
            (InputAction::ObjectPaintMode, Key(VirtualKeyCode::Key3)),
            (InputAction::DragMode, Key(VirtualKeyCode::Key4)),
            (InputAction::ToggleFullScreen, Key(VirtualKeyCode::F)),
        ]],
    )
}
```

## Sandbox

Currently `corrode` allows you to make an app which could be 2D or 3D or anything you like. It's general purpose. That's why you'll notice that there's quite a bit of _engine_ logic inside the Sandbox app too.

Sandbox uses the engine api functionality to split it's runtime (core loop) logic as follows.

- At `start` we'll create everything our app needs. Our simulation structs etc. and read files.
- `update` steps the cellular automata simulation.
- `render` draws individually what needs to be drawn. Our canvas, debug lines, dragged object line, painting circle and object images when we are about to place a drawn object.
The draw commands use the api exposed from the engine.
- `gui_content` uses `egui` commands to create our immediate mode GUI. This gets drawn under the hood of the `corrode`'s renderer using my own library [egui_winit_vulkano](https://github.com/hakolao/egui_winit_vulkano).

```rust
impl Engine<InputAction> for SandboxApp {
    fn start<E>(
        &mut self,
        _event_loop: &EventLoop<E>,
        api: &mut EngineApi<InputAction>,
    ) -> Result<()> {
        // Create structs and objects

        // Zoom to desired level
        // Read matter definitions
        // Create simulator
        // Register gui images (for editor windows in gui)
        // Update settings based on read information from renderer
        // Toggle fullscreen
        // Adjust gravity
    }

    fn update(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        // Update editor & handle inputs there
        // Step simulation
    }

    fn render<F>(
        &mut self,
        before_future: F,
        api: &mut EngineApi<InputAction>,
    ) -> Result<Box<dyn GpuFuture + 'static>>
        where
            F: GpuFuture + 'static,
    {
        // Render canvas first
        // Debug renders
        // Render line from dragged object
        // Render circle when painting
        // Draw painted object image
    }

    fn gui_content(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        //... Gui layout here using egui.
    }

    fn end_of_frame(&mut self, api: &mut EngineApi<InputAction>) -> Result<()> {
        //... What we wish to do at the end of each frame, in sandbox' case, update performance timers
    }
}
```

### Simulator

Simulator is the core of the `Sandbox` app responsible for organization the world, interaction with the world and of handling the pixel physics objects calculation.

Each frame it does the following:
1. Write objects to cellular automata grid
2. Step cellular automata (multiple steps if needed). Updates solid etc. physics boundaries
3. Remove object pixels from grid
4. Form contours for new deformed physics objects
5. Step physics simulation

```rust
    pub fn step(
        &mut self,
        api: &mut EngineApi<InputAction>,
        settings: AppSettings,
        canvas_mouse_state: &CanvasMouseState,
    ) -> Result<()> {
        //...
        // Update chunks if needed
        self.chunk_manager.update_chunks(self.camera_canvas_pos, &self.matter_definitions)?;
        //...
        self.write_pixel_objects_to_grid(api)?;
        //...
        self.ca_simulator.step(settings, self.camera_canvas_pos, &mut self.chunk_manager)?;
        //...
        self.update_objects_from_grid(api)?;
        //...
        self.update_physics_boundaries(api)?;
        //...
        api.physics_world
            .step(&api.thread_pool, |_collision_event| {});
        self.update_dynamic_physics_objects(api)?;
        //...
    }
```

- [CASimulator](https://github.com/hakolao/sandbox/blob/master/sandbox/src/sim/ca_simulator.rs) is the compute shader pipeline stepping the cellular automata style pixel simulation on the canvas. It takes the matter grid data, passes it to its compute shader pipeline and it's `step` function will run the simulation each frame. Simulation will call this. 
- [SimulationChunkManager](https://github.com/hakolao/sandbox/blob/master/sandbox/src/sim/simulation_chunk_manager.rs) is a chunk manager holding the world partially on the CPU side and loading chunks to GPU side. These chunks are passed to CA simulation and used to interact with the grid data. Map files are loaded to the grid here.
- The rest of the files in the sim folder are various utils and calculation.

### Interact
Interaction in sandbox app are done in various ways. You can
- Paint matter
- Paint pixel objects of specific matter
- Place pixel objects from images of specific matter
- Drag pixel objects that exist in the world and throw them around

[interact](https://github.com/hakolao/sandbox/blob/master/sandbox/src/interact) holds the functionality for these and is called on `update` stage of the app.

The interaction with the world is done through `Simulator`'s API.

### Matter
Matter is split into matter types defined by `MatterDefinition`.

```rust
pub struct MatterDefinition {
    pub id: u32,
    pub name: String,
    pub color: u32,
    pub weight: f32,
    pub state: MatterState,
    pub dispersion: u32,
    /// What are the characteristics of matter?
    /// - Water: "Cools", "Rusts"
    /// - Acid: "Corrodes".
    /// Think of it like: "What does this do to others?"
    pub characteristics: MatterCharacteristic,
    /// How does matter react to neighbor characteristics?
    /// - Example: "Water becomes ice on probability x if touches one that freezes".
    /// - Example: "Acid might become empty on probability x if touches a material it corroded (corroding)".
    /// Probability will affect the speed at which matter changes
    pub reactions: [MatterReaction; MAX_TRANSITIONS as usize],
}
```

These define the behavior of the matter during `CASimulator`'s step. A lot of the logic and how the data is used can be found in the shaders and `CASimulator`.

Matter data can be modified at run time via GUI.

### Object
Objects are pixel objects that contain pixel data of matter and color which are passed to the grid during simulation.

During each frame after the `CASimulator`'s step, we'll check if the pixel objects pixel data was changed (e.g. by acid destroying it) and we then have to calculate the deformed objects.
This is done with [Connected Component Labeling](https://en.wikipedia.org/wiki/Connected-component_labeling). If the object's _alive pixels_ are now consisting
of multiple parts, we destroy the object and create new images from that. Their physics colliders are created using the [VHACD](https://github.com/kmammou/v-hacd) algorithm with 
`rapier` library's `ColliderBuilder::convex_decomposition_with_params`.

The code in [object](https://github.com/hakolao/sandbox/tree/master/sandbox/src/object) directory holds the functionality that is used to do above, however it's organized by the `Simulator`.

