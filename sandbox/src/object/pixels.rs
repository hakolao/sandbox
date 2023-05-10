use std::sync::Arc;

use cgmath::Vector2;
use hecs::Entity;
use image::RgbaImage;

use crate::{object::MatterPixel, utils::BitmapImage};

#[derive(Debug, Copy, Clone)]
pub struct TempPixel {
    pub pixel_index: usize,
    pub canvas_pos: Vector2<i32>,
    pub matter: u32,
    pub color: u32,
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct PixelData {
    pub image: Arc<BitmapImage>,
    pub pixels: Vec<MatterPixel>,
    pub width: u32,
    pub height: u32,
}

impl PixelData {
    pub fn empty() -> PixelData {
        PixelData {
            image: Arc::new(BitmapImage {
                data: vec![],
                width: 0,
                height: 0,
            }),
            pixels: vec![],
            width: 0,
            height: 0,
        }
    }

    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn to_image(&self) -> RgbaImage {
        let mut rgba = vec![0; self.pixels.len() * 4];
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let pixel_index = y * self.width as usize + x;
                let invert_y_index = (self.height as usize - y - 1) * self.width as usize + x;
                let pixel = self.pixels[pixel_index];
                if pixel.is_alive {
                    let old_index = pixel.color_index * 4;
                    let new_index = invert_y_index * 4;
                    rgba[new_index] = self.image.data[old_index];
                    rgba[new_index + 1] = self.image.data[old_index + 1];
                    rgba[new_index + 2] = self.image.data[old_index + 2];
                    rgba[new_index + 3] = self.image.data[old_index + 3];
                }
            }
        }
        RgbaImage::from_raw(self.width, self.height, rgba).unwrap()
    }

    pub fn split_by_bitmap(
        empty_matter: u32,
        old_pixel_data: &PixelData,
        new_bitmap: &[f64],
        width: u32,
        height: u32,
        pixel_start: Vector2<i32>,
    ) -> PixelData {
        let mut pixel_data = PixelData {
            image: old_pixel_data.image.clone(),
            pixels: vec![MatterPixel::zero(empty_matter); (width * height) as usize],
            width,
            height,
        };
        let start_y = pixel_start.y;
        let end_y = pixel_start.y + height as i32;
        let start_x = pixel_start.x;
        let end_x = pixel_start.x + width as i32;
        for y in (start_y..end_y).rev() {
            for x in start_x..end_x {
                let old_index = (y * old_pixel_data.width as i32 + x) as usize;
                let x = x - pixel_start.x;
                let y = y - pixel_start.y;
                let pixel_index = (y * width as i32 + x) as usize;
                pixel_data.pixels[pixel_index] = old_pixel_data.pixels[old_index];
                pixel_data.pixels[pixel_index].is_alive = new_bitmap[pixel_index] == 1.0;
            }
        }
        pixel_data
    }
}
