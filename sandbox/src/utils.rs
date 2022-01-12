use core::fmt;
use std::{collections::BTreeSet, env::current_dir, fs, hash::Hash, path::PathBuf};

use anyhow::*;
use cgmath::Vector2;
use corrode::{input_system::InputSystem, renderer::Camera2D};
use image::{GenericImageView, RgbaImage};

use crate::{map_path, matter::MatterDefinitions, sim::world_pos_to_canvas_pos};

/// 32 bit bitmap image
#[derive(Debug, Clone)]
pub struct BitmapImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl BitmapImage {
    pub fn empty(width: u32, height: u32) -> BitmapImage {
        BitmapImage {
            data: vec![0; (width * height) as usize * 4],
            width,
            height,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CanvasMouseState {
    pub mouse_world_pos: Vector2<f32>,
    pub mouse_on_canvas_f32: Vector2<f32>,
    pub mouse_on_canvas: Vector2<i32>,
    pub prev_mouse_world_pos: Vector2<f32>,
    pub prev_mouse_on_canvas_f32: Vector2<f32>,
    pub prev_mouse_on_canvas: Vector2<i32>,
}

impl fmt::Display for CanvasMouseState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Mouse (\n world: ({:.3}, {:.3})\n canvas: ({}, {})\n)",
            self.mouse_world_pos.x,
            self.mouse_world_pos.y,
            self.mouse_on_canvas.x,
            self.mouse_on_canvas.y,
        )
    }
}

impl CanvasMouseState {
    pub fn new<I: Hash + Eq + Copy + 'static>(
        camera: &Camera2D,
        input_system: &InputSystem<I>,
    ) -> Self {
        let normalized_mouse = input_system.mouse_position_normalized();
        let last_normalized_mouse = input_system.last_mouse_position_normalized();

        let mouse_world_pos = camera.screen_to_world_pos(normalized_mouse);
        let mouse_on_canvas_f32 = world_pos_to_canvas_pos(mouse_world_pos);
        let mouse_on_canvas = mouse_on_canvas_f32.cast::<i32>().unwrap();

        let prev_mouse_world_pos = camera.screen_to_world_pos(last_normalized_mouse);
        let prev_mouse_on_canvas_f32 = world_pos_to_canvas_pos(prev_mouse_world_pos);
        let prev_mouse_on_canvas = prev_mouse_on_canvas_f32.cast::<i32>().unwrap();
        CanvasMouseState {
            mouse_world_pos,
            prev_mouse_world_pos,
            mouse_on_canvas,
            prev_mouse_on_canvas,
            mouse_on_canvas_f32,
            prev_mouse_on_canvas_f32,
        }
    }
}

pub fn rotate_radians(v: Vector2<f32>, rad: f32) -> Vector2<f32> {
    let ca = rad.cos();
    let sa = rad.sin();
    Vector2::new(ca * v.x - sa * v.y, sa * v.x + ca * v.y)
}

/// Loads an image as rgba array from file_bytes (whole file in memory as bytes)`
pub fn load_image_from_file_bytes(file_bytes: &[u8]) -> BitmapImage {
    let img = image::load_from_memory(file_bytes).expect("Failed to load image from bytes");
    let rgba = if let Some(rgba) = img.as_rgba8() {
        rgba.to_owned().to_vec()
    } else {
        // Convert rgb to rgba
        let rgb = img.as_rgb8().unwrap().to_owned();
        let mut raw_data = vec![];
        for val in rgb.chunks(3) {
            raw_data.push(val[0]);
            raw_data.push(val[1]);
            raw_data.push(val[2]);
            raw_data.push(255);
        }
        let new_rgba = RgbaImage::from_raw(rgb.width(), rgb.height(), raw_data).unwrap();
        new_rgba.to_vec()
    };
    let (width, height) = img.dimensions();
    BitmapImage {
        data: rgba,
        width,
        height,
    }
}

pub fn load_bitmap_image_from_path(path: PathBuf) -> Result<BitmapImage> {
    let contents = fs::read(path)?;
    let map_img = load_image_from_file_bytes(&contents);
    Ok(map_img)
}

pub fn u8_rgba_to_u32_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

pub fn u32_rgba_to_u32_abgr(num: u32) -> u32 {
    let r = num & 255;
    let g = (num >> 8) & 255;
    let b = (num >> 16) & 255;
    let a = (num >> 24) & 255;
    (r << 24) | (g << 16) | (b << 8) | a
}

pub fn u32_rgba_to_u8_rgba(num: u32) -> [u8; 4] {
    let r = num & 255;
    let g = (num >> 8) & 255;
    let b = (num >> 16) & 255;
    let a = (num >> 24) & 255;
    [a as u8, b as u8, g as u8, r as u8]
}

pub fn u32_rgba_to_f32_rgba(num: u32) -> [f32; 4] {
    let color_u8 = u32_rgba_to_u8_rgba(num);
    [
        color_u8[0] as f32 / 255.0,
        color_u8[1] as f32 / 255.0,
        color_u8[2] as f32 / 255.0,
        color_u8[3] as f32 / 255.0,
    ]
}

pub fn get_map_directory_names() -> Result<BTreeSet<String>> {
    let mut file_names = BTreeSet::new();
    let dir_path = map_path();
    fs::create_dir_all(dir_path.clone()).unwrap();
    for file in fs::read_dir(dir_path.clone()).unwrap() {
        let file = file.unwrap().file_name();
        let file_name = file.to_str().unwrap();
        let file_path = dir_path.join(file_name);
        if std::fs::metadata(file_path).unwrap().is_dir() {
            file_names.insert(file_name.to_string());
        }
    }
    Ok(file_names)
}

pub fn read_matter_definitions_file() -> Option<MatterDefinitions> {
    let matter_definitions_path = current_dir()
        .unwrap()
        .join("assets/matter_definitions.json");
    if let std::result::Result::Ok(data) = fs::read_to_string(matter_definitions_path) {
        Some(MatterDefinitions::deserialize(&data))
    } else {
        None
    }
}
