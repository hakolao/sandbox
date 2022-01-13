// Turn off console on windows
#![windows_subsystem = "windows"]
#![feature(total_cmp)]
#![feature(slice_group_by)]
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

mod app;
mod editor;
mod gui_state;
mod matter;
mod object;
mod render;
mod settings;
mod simulation;
mod utils;

use std::{env::current_dir, path::PathBuf};

use anyhow::*;
use cgmath::Vector2;
use corrode::{
    engine::{Corrode, EngineOptions, RenderOptions},
    input_system::InputButton::Key,
    logger::initialize_logger,
};
use simplelog::LevelFilter;
use winit::event::VirtualKeyCode;

use crate::app::{App, InputAction};

/// This is an example for using doc comment attributes
/// Canvas plane scale (1.0 means our world is between -1.0 and 1.0)
pub const WORLD_UNIT_SIZE: f32 = 10.0;
/// Kernel size x & y
pub const KERNEL_SIZE: u32 = 32;
/// Max number of matters
pub const MAX_NUM_MATTERS: u32 = 256;
pub const GPU_CHUNKS_NUM_SIDE: u32 = 6;
pub const MAX_GPU_CHUNKS: u32 = GPU_CHUNKS_NUM_SIDE * GPU_CHUNKS_NUM_SIDE;
pub const INIT_DISPERSION_STEPS: u32 = 10;
pub const INIT_MOVEMENT_STEPS: u32 = 3;
pub const CELL_OFFSETS_NINE: [Vector2<i32>; 9] = [
    Vector2::new(-1, 1),
    Vector2::new(0, 1),
    Vector2::new(1, 1),
    Vector2::new(-1, 0),
    Vector2::new(0, 0),
    Vector2::new(1, 0),
    Vector2::new(-1, -1),
    Vector2::new(0, -1),
    Vector2::new(1, -1),
];

lazy_static! {
    /// Number of cells in simulated canvas area
    pub static ref  SIM_CANVAS_SIZE: u32 = if std::env::var("LARGE").is_ok() { 1024 } else { 512 };
    pub static ref HALF_CANVAS: Vector2<i32> =
        Vector2::new((*SIM_CANVAS_SIZE / 2) as i32, (*SIM_CANVAS_SIZE / 2) as i32);
    /// Size of canvas chunk
    pub static ref  CANVAS_CHUNK_SIZE: u32 = *SIM_CANVAS_SIZE;
    /// Size of one cell in world units
    pub static ref  CELL_UNIT_SIZE: f32 = WORLD_UNIT_SIZE / *SIM_CANVAS_SIZE as f32;
    pub static ref HALF_CELL: Vector2<f32> = Vector2::new(*CELL_UNIT_SIZE * 0.5, *CELL_UNIT_SIZE * 0.5);
    /// Ratio of bitmap to canvas. If this is 4, bitmap size is (512 / 4) * (512 / 4)
    pub static ref  BITMAP_RATIO: u32 = if std::env::var("LARGE").is_ok() { 8 } else { 4 };
    /// Ratio with which we must adjust the vertices of solid utils to correctly position them
    pub static ref  BITMAP_PIXEL_TO_CANVAS_RATIO: f64 =
        WORLD_UNIT_SIZE as f64 / (*SIM_CANVAS_SIZE / *BITMAP_RATIO) as f64;
}

pub fn map_path() -> PathBuf {
    if *SIM_CANVAS_SIZE == 1024 {
        current_dir().unwrap().join("assets/maps/large")
    } else {
        current_dir().unwrap().join("assets/maps/small")
    }
}

fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    initialize_logger(LevelFilter::Debug)?;
    #[cfg(not(debug_assertions))]
    initialize_logger(LevelFilter::Info)?;

    Corrode::run(
        App::new()?,
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
