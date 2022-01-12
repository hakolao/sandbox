use std::sync::Arc;

use anyhow::*;
use cgmath::Vector2;
use corrode::renderer::{Camera2D, Line};
use hecs::Entity;
use rapier2d::geometry::Collider;
use vulkano::buffer::CpuAccessibleBuffer;

use crate::{
    matter::MatterDefinitions,
    object::{
        collider_from_polylines, collider_sensor_from_polylines, douglas_peucker_simplify,
        form_contour_vertices, PixelData, TempPixel,
    },
    simulation::Simulation,
    utils::{rotate_radians, u32_rgba_to_u8_rgba, u8_rgba_to_u32_rgba, BitmapImage},
    BITMAP_PIXEL_TO_CANVAS_RATIO, BITMAP_RATIO, CANVAS_CHUNK_SIZE, HALF_CANVAS, HALF_CELL,
    SIM_CANVAS_SIZE, WORLD_UNIT_SIZE,
};

/// Convert normalized mouse position to position on the pixel canvas
#[allow(unused)]
pub fn mouse_to_canvas_pos(normalized_mouse: Vector2<f32>, camera: &Camera2D) -> Vector2<f32> {
    world_pos_to_canvas_pos(camera.screen_to_world_pos(normalized_mouse))
}

#[allow(unused)]
/// Convert canvas integer position to world position
pub fn canvas_pos_to_world_pos(canvas_pos: Vector2<i32>) -> Vector2<f32> {
    let mut world_pos = Vector2::new(canvas_pos.x as f32 + 0.5, canvas_pos.y as f32 + 0.5);
    world_pos /= *SIM_CANVAS_SIZE as f32 / WORLD_UNIT_SIZE;
    world_pos
}

pub fn world_pos_to_canvas_pos(world_pos: Vector2<f32>) -> Vector2<f32> {
    let mut canvas_pos = world_pos;
    canvas_pos *= *SIM_CANVAS_SIZE as f32 / WORLD_UNIT_SIZE;
    Vector2::new(canvas_pos.x.round(), canvas_pos.y.round())
}

pub fn is_inside_sim_canvas(canvas_pos: Vector2<i32>, camera_canvas_pos: Vector2<i32>) -> bool {
    let pos = canvas_pos + *HALF_CANVAS - camera_canvas_pos;
    pos.x >= 0 && pos.x < *SIM_CANVAS_SIZE as i32 && pos.y >= 0 && pos.y < *SIM_CANVAS_SIZE as i32
}

pub fn world_pos_inside_canvas(world_pos: Vector2<f32>, camera_world_pos: Vector2<f32>) -> bool {
    let canvas_pos = world_pos_to_canvas_pos(world_pos);
    let camera_canvas_pos = world_pos_to_canvas_pos(camera_world_pos);
    is_inside_sim_canvas(
        canvas_pos.cast::<i32>().unwrap(),
        camera_canvas_pos.cast::<i32>().unwrap(),
    )
}

/// Returns the chunk index as well as index inside the chunk...
pub fn sim_chunk_canvas_index(
    canvas_pos: Vector2<i32>,
    chunk_start: Vector2<i32>,
) -> (usize, usize) {
    let diff = canvas_pos - chunk_start;
    let chunk_diff = diff / *SIM_CANVAS_SIZE as i32;
    let index = ((diff.y % *SIM_CANVAS_SIZE as i32) * *SIM_CANVAS_SIZE as i32
        + (diff.x % *SIM_CANVAS_SIZE as i32)) as usize;
    let chunk_index = (chunk_diff.y * 2 + chunk_diff.x) as usize;
    (chunk_index, index)
}

pub fn sim_canvas_index(canvas_pos: Vector2<i32>, camera_canvas_pos: Vector2<i32>) -> usize {
    let pos = canvas_pos + *HALF_CANVAS - camera_canvas_pos;
    (pos.y * *SIM_CANVAS_SIZE as i32 + pos.x) as usize
}

pub(crate) fn create_boundary_object_data(
    pos_offset: Vector2<f32>,
    bitmap: &Vec<f64>,
    sensor: bool,
) -> Vec<(Vector2<f32>, f32, Collider)> {
    form_contour_vertices(
        bitmap,
        *SIM_CANVAS_SIZE / *BITMAP_RATIO,
        *SIM_CANVAS_SIZE / *BITMAP_RATIO,
        *BITMAP_PIXEL_TO_CANVAS_RATIO,
    )
    .iter()
    .filter_map(|c| {
        let contour = douglas_peucker_simplify(c.to_vec(), 0.0001);
        if contour.len() < 3 {
            return None;
        }
        let collider = if sensor {
            collider_sensor_from_polylines(&contour)
        } else {
            collider_from_polylines(&contour)
        };
        let pos = pos_offset;
        let angle = 0.0;
        Some((pos, angle, collider))
    })
    .collect()
}

pub fn get_collider_lines(collider: &Collider, color: [f32; 4]) -> Vec<Line> {
    let mut lines = vec![];
    if let Some(comp) = collider.shape().as_compound() {
        comp.shapes().iter().for_each(|s| {
            if let Some(poly) = s.1.as_convex_polygon() {
                let points = poly.points();
                for i in 0..points.len() {
                    if i != points.len() - 1 {
                        let p1 = rotate_radians(
                            Vector2::new(points[i][0], points[i][1]),
                            collider.rotation().angle(),
                        );
                        let p2 = rotate_radians(
                            Vector2::new(points[i + 1][0], points[i + 1][1]),
                            collider.rotation().angle(),
                        );
                        let offset = collider.translation().xy();
                        let offset = Vector2::new(offset[0], offset[1]);
                        lines.push(Line(
                            Vector2::new(p1.x as f32, p1.y as f32) + offset,
                            Vector2::new(p2.x as f32, p2.y as f32) + offset,
                            color,
                        ));
                    } else {
                        let p1 = rotate_radians(
                            Vector2::new(points[i][0], points[i][1]),
                            collider.rotation().angle(),
                        );
                        let p2 = rotate_radians(
                            Vector2::new(points[0][0], points[0][1]),
                            collider.rotation().angle(),
                        );
                        let offset = collider.translation().xy();
                        let offset = Vector2::new(offset[0], offset[1]);
                        lines.push(Line(
                            Vector2::new(p1.x as f32, p1.y as f32) + offset,
                            Vector2::new(p2.x as f32, p2.y as f32) + offset,
                            color,
                        ));
                    }
                }
            }
        })
    }

    if let Some(polyline) = collider.shape().as_polyline() {
        for segment in polyline.segments() {
            let p1 = rotate_radians(
                Vector2::new(segment.a[0], segment.a[1]),
                collider.rotation().angle(),
            );
            let p2 = rotate_radians(
                Vector2::new(segment.b[0], segment.b[1]),
                collider.rotation().angle(),
            );
            let offset = collider.translation().xy();
            let offset = Vector2::new(offset[0], offset[1]);
            lines.push(Line(
                Vector2::new(p1.x as f32, p1.y as f32) + offset,
                Vector2::new(p2.x as f32, p2.y as f32) + offset,
                color,
            ));
        }
    }

    if let Some(trimesh) = collider.shape().as_trimesh() {
        let offset = collider.translation().xy();
        let offset = Vector2::new(offset[0], offset[1]);
        for t in trimesh.triangles() {
            let p1 = rotate_radians(Vector2::new(t.a[0], t.a[1]), collider.rotation().angle());
            let p2 = rotate_radians(Vector2::new(t.b[0], t.b[1]), collider.rotation().angle());
            lines.push(Line(
                Vector2::new(p1.x as f32, p1.y as f32) + offset,
                Vector2::new(p2.x as f32, p2.y as f32) + offset,
                color,
            ));
            let p1 = rotate_radians(Vector2::new(t.b[0], t.b[1]), collider.rotation().angle());
            let p2 = rotate_radians(Vector2::new(t.c[0], t.c[1]), collider.rotation().angle());
            lines.push(Line(
                Vector2::new(p1.x as f32, p1.y as f32) + offset,
                Vector2::new(p2.x as f32, p2.y as f32) + offset,
                color,
            ));
            let p1 = rotate_radians(Vector2::new(t.c[0], t.c[1]), collider.rotation().angle());
            let p2 = rotate_radians(Vector2::new(t.a[0], t.a[1]), collider.rotation().angle());
            lines.push(Line(
                Vector2::new(p1.x as f32, p1.y as f32) + offset,
                Vector2::new(p2.x as f32, p2.y as f32) + offset,
                color,
            ));
        }
    }

    lines
}

/// https://datagenetics.com/blog/august32013/index.html
///     |1  -tan(𝜃/2) |  |1        0|  |1  -tan(𝜃/2) |
///     |0      1     |  |sin(𝜃)   1|  |0      1     |
fn shear(angle: f32, pos: Vector2<i32>) -> Vector2<i32> {
    let mut angle = angle;
    let mut pos = Vector2::new(pos.x as f32, pos.y as f32);
    // Distortion fix ----
    let one_thirty_five = 3.0 * std::f32::consts::PI / 4.0;
    let one_eighty = std::f32::consts::PI;
    let angle_abs = angle.abs();
    if angle_abs < one_eighty && angle_abs > one_thirty_five {
        pos.x *= -1.0;
        pos.y *= -1.0;
        angle += one_eighty;
        if angle >= 2.0 * std::f32::consts::PI {
            angle -= std::f32::consts::PI;
        }
    }
    // ---
    let alpha = -1.0 * (angle / 2.0).tan();
    let beta = angle.sin();
    // Shear 1
    let x = (pos.x + pos.y * alpha).round();
    // Shear 2
    let y = (x * beta + pos.y).round();
    // Shear 3
    let x = (x + y * alpha).round();
    Vector2::new(x as i32, y as i32)
}

pub fn get_alive_pixels(
    pixel_data: &PixelData,
    pos: Vector2<f32>,
    angle: f32,
    entity: Entity,
) -> Vec<TempPixel> {
    let pixels = &pixel_data.pixels;
    let w = pixel_data.width as i32;
    let h = pixel_data.height as i32;
    let obj_canvas_pos = world_pos_to_canvas_pos(pos);
    let half_w = (((w as f32 + 1.0) / 2.0) - 1.0).round() as i32;
    let half_h = (((h as f32 + 1.0) / 2.0) - 1.0).round() as i32;
    (0..(h * w))
        .into_iter()
        .filter_map(|pixel_index| {
            let x = pixel_index % w;
            let y = pixel_index / w;
            if pixels[pixel_index as usize].is_alive {
                let pixel_pos_relative_to_center = Vector2::new(x - half_w, y - half_h);
                let new_pos = shear(angle, pixel_pos_relative_to_center);
                let canvas_pos = new_pos + obj_canvas_pos.cast::<i32>().unwrap();
                let pixel = pixel_data.pixels[pixel_index as usize];
                let rgba_index = pixel.color_index * 4;
                let r = pixel_data.image.data[rgba_index];
                let g = pixel_data.image.data[rgba_index + 1];
                let b = pixel_data.image.data[rgba_index + 2];
                let a = pixel_data.image.data[rgba_index + 3];
                Some(TempPixel {
                    pixel_index: pixel_index as usize,
                    canvas_pos,
                    matter: pixel.matter,
                    color: u8_rgba_to_u32_rgba(a, b, g, r),
                    entity,
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn write_matter_image_to_canvas_chunk(
    matter_image: &BitmapImage,
    matter_definitions: &MatterDefinitions,
    chunk_in: Arc<CpuAccessibleBuffer<[u32]>>,
    chunk_out: Arc<CpuAccessibleBuffer<[u32]>>,
) -> Result<()> {
    let mut matter_grid_in = chunk_in.write()?;
    let mut matter_grid_out = chunk_out.write()?;
    for y in 0..matter_image.height as usize {
        for x in 0..matter_image.width as usize {
            let index = y * matter_image.width as usize + x;
            // ToDo: Matter definitions could be a hash map or something to speed up "find"
            let matter = if let Some(m) = matter_definitions.definitions.iter().find(|m| {
                let r = matter_image.data[index * 4 + 0];
                let g = matter_image.data[index * 4 + 1];
                let b = matter_image.data[index * 4 + 2];
                let a = matter_image.data[index * 4 + 3];
                let color = u8_rgba_to_u32_rgba(r, g, b, a);
                m.color == color
            }) {
                m.id
            } else {
                matter_definitions.empty
            };
            let flipped_y_index =
                ((*CANVAS_CHUNK_SIZE) as usize - y - 1) * (*CANVAS_CHUNK_SIZE) as usize + x;
            matter_grid_in[flipped_y_index] = matter;
            matter_grid_out[flipped_y_index] = matter;
        }
    }
    Ok(())
}

pub fn write_canvas_chunk_to_matter_image(
    matter_definitions: &MatterDefinitions,
    chunk: Arc<CpuAccessibleBuffer<[u32]>>,
) -> Result<BitmapImage> {
    let matter_grid = chunk.read()?;
    let mut image = BitmapImage::empty(*CANVAS_CHUNK_SIZE, *CANVAS_CHUNK_SIZE);
    for y in 0..(*CANVAS_CHUNK_SIZE) as usize {
        for x in 0..(*CANVAS_CHUNK_SIZE) as usize {
            let index = y * (*CANVAS_CHUNK_SIZE) as usize + x;
            let flipped_y_index =
                ((*CANVAS_CHUNK_SIZE) as usize - 1 - y) * (*CANVAS_CHUNK_SIZE) as usize + x;
            let matter = matter_grid[flipped_y_index];
            let color = u32_rgba_to_u8_rgba(matter_definitions.definitions[matter as usize].color);
            image.data[index * 4 + 0] = color[0];
            image.data[index * 4 + 1] = color[1];
            image.data[index * 4 + 2] = color[2];
            image.data[index * 4 + 3] = color[3];
        }
    }
    Ok(image)
}

pub fn log_world_performance(simulation: &Simulation) {
    println!("  World functions:");
    println!(
        "  Obj write: {:.3}",
        simulation.obj_write_timer.time_average_ms()
    );
    println!("  CA sim: {:.3}", simulation.ca_timer.time_average_ms());
    println!(
        "  Obj deform: {:.3}",
        simulation.obj_read_timer.time_average_ms()
    );
    println!(
        "  Boundary create: {:.3}",
        simulation.boundary_timer.time_average_ms()
    );
    println!(
        "  Physics step: {:.3}",
        simulation.physics_timer.time_average_ms()
    );
}

pub fn chunk_lines(chunk: Vector2<i32>, chunk_color: [f32; 4]) -> Vec<Line> {
    vec![
        Line(
            0.5 * Vector2::new(-WORLD_UNIT_SIZE, WORLD_UNIT_SIZE)
                + chunk.cast::<f32>().unwrap() * WORLD_UNIT_SIZE
                - *HALF_CELL,
            0.5 * Vector2::new(WORLD_UNIT_SIZE, WORLD_UNIT_SIZE)
                + chunk.cast::<f32>().unwrap() * WORLD_UNIT_SIZE
                - *HALF_CELL,
            chunk_color,
        ),
        Line(
            0.5 * Vector2::new(-WORLD_UNIT_SIZE, -WORLD_UNIT_SIZE)
                + chunk.cast::<f32>().unwrap() * WORLD_UNIT_SIZE
                - *HALF_CELL,
            0.5 * Vector2::new(WORLD_UNIT_SIZE, -WORLD_UNIT_SIZE)
                + chunk.cast::<f32>().unwrap() * WORLD_UNIT_SIZE
                - *HALF_CELL,
            chunk_color,
        ),
        Line(
            0.5 * Vector2::new(-WORLD_UNIT_SIZE, -WORLD_UNIT_SIZE)
                + chunk.cast::<f32>().unwrap() * WORLD_UNIT_SIZE
                - *HALF_CELL,
            0.5 * Vector2::new(-WORLD_UNIT_SIZE, WORLD_UNIT_SIZE)
                + chunk.cast::<f32>().unwrap() * WORLD_UNIT_SIZE
                - *HALF_CELL,
            chunk_color,
        ),
        Line(
            0.5 * Vector2::new(WORLD_UNIT_SIZE, -WORLD_UNIT_SIZE)
                + chunk.cast::<f32>().unwrap() * WORLD_UNIT_SIZE
                - *HALF_CELL,
            0.5 * Vector2::new(WORLD_UNIT_SIZE, WORLD_UNIT_SIZE)
                + chunk.cast::<f32>().unwrap() * WORLD_UNIT_SIZE
                - *HALF_CELL,
            chunk_color,
        ),
    ]
}
