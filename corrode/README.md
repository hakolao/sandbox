# Corrode
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

Not really an engine, but rather a set of simple structs & behavior to nicely wrap your engine logic around it. You can replace most things from `api`
with your own implementation. Most importantly it provides a nice way to wrap your main loop logic in your game engine and provides help not having to implement a full renderer yourself.

See `api.rs` for what you can implement and see `sandbox` for an example on how to use the api.

The renderer uses `Vulkan` backend with [Vulkano](https://github.com/vulkano-rs/vulkano.git).

How to use:
```rust
// At its simplest, () = type of input actions (None) in this case
fn main() -> Result<()> {
    // App defines the contents of your application, EngineOptions provides options for `Corrode`
    // Last input refers to input mappings
    Corrode::run(App {}, EngineOptions::default(), vec![vec![]])
}

pub struct App {}

// In order to pass App to Corrode, you must implement `Engine`. And to add logic to your runtime
// Override the default functions under Engine (see examples for more)
impl Engine<()> for App {}
```

## Note
This works as a project for exploration of game engine architecture. Its renderer & pipeline are super simple, though you can create
your own pipelines too outside this. It might also be useful to get rid of the `api` and replace it with something like `bevy_ecs`.