use std::sync::Arc;

use cgmath::Vector2;
use contour::contour_rings;

use crate::{
    object::{MatterPixel, PixelData},
    utils::BitmapImage,
    CELL_UNIT_SIZE, HALF_CELL,
};

pub fn form_pixel_data_with_contours_from_image(
    image: &Arc<BitmapImage>,
    matter: u32,
    empty_matter: u32,
) -> (PixelData, Vec<Vec<Vector2<f64>>>) {
    let mut bitmap = vec![1.0; (image.width * image.height) as usize];
    let mut pixel_data = PixelData::empty();
    pixel_data.image = image.clone();
    pixel_data.pixels =
        vec![MatterPixel::zero(empty_matter); (image.width * image.height) as usize];
    pixel_data.width = image.width;
    pixel_data.height = image.height;
    for y in 0..image.height {
        for x in 0..image.width {
            let index = (y * image.width + x) as usize;
            let flipped_y_index = ((image.height - y - 1) * image.width + x) as usize;
            let alpha = image.data[index * 4 + 3];
            if alpha == 0 {
                pixel_data.pixels[flipped_y_index] = MatterPixel {
                    matter: empty_matter,
                    color_index: index,
                    is_alive: false,
                };
                bitmap[flipped_y_index] = 0.0;
            } else {
                pixel_data.pixels[flipped_y_index] = MatterPixel {
                    matter,
                    color_index: index,
                    is_alive: true,
                };
            }
        }
    }
    let contours =
        form_contour_vertices(&bitmap, image.width, image.height, *CELL_UNIT_SIZE as f64);
    (pixel_data, contours)
}

/// Forms object contour vertices based on bitmap of 1.0s and 0.0s of given width and height.
pub fn form_contour_vertices(
    shape_bitmap: &Vec<f64>,
    width: u32,
    height: u32,
    cell_ratio_to_world: f64,
) -> Vec<Vec<Vector2<f64>>> {
    contour_rings(shape_bitmap, 0.5, width, height)
        .unwrap()
        .iter()
        .map(|r| {
            r.iter()
                .map(|p| {
                    Vector2::new(
                        0.5 * (p[0] * 2.0 - width as f64) * cell_ratio_to_world
                            - HALF_CELL.x as f64,
                        0.5 * (p[1] * 2.0 - height as f64) * cell_ratio_to_world
                            - HALF_CELL.y as f64,
                    )
                })
                .collect::<Vec<Vector2<f64>>>()
        })
        .collect::<Vec<Vec<Vector2<f64>>>>()
}

/// Calculates perpendicular squared distance of point from line
#[allow(unused)]
fn perpendicular_squared_distance(point: Vector2<f64>, line: (Vector2<f64>, Vector2<f64>)) -> f64 {
    let x_diff = line.1.x - line.0.x;
    let y_diff = line.1.y - line.0.y;
    let numerator =
        (y_diff * point.x - x_diff * point.y + line.1.x * line.0.y - line.1.y * line.0.x).abs();
    let numerator_squared = numerator * numerator;
    let denominator_squared = y_diff * y_diff + x_diff * x_diff;
    numerator_squared / denominator_squared
}

/// Using recursive Ramer-Douglas-Peucker algorithm https://en.wikipedia.org/wiki/Ramer%E2%80%93Douglas%E2%80%93Peucker_algorithm
/// Simplifies a set of consecutive vertices while max squared distance is above epsilon
#[allow(unused)]
pub fn douglas_peucker_simplify(vertices: Vec<Vector2<f64>>, epsilon: f64) -> Vec<Vector2<f64>> {
    let mut d_squared_max = 0.0;
    let mut farthest_point_index = 0;
    let end = vertices.len() - 1;
    if end < 3 {
        return vertices;
    }
    let line = (vertices[0], vertices[end - 1]);
    for i in 1..(end - 1) {
        let d_squared = perpendicular_squared_distance(vertices[i], line);
        if d_squared > d_squared_max {
            farthest_point_index = i;
            d_squared_max = d_squared;
        }
    }
    if d_squared_max > epsilon * epsilon {
        let rec_results1 =
            douglas_peucker_simplify(vertices[0..farthest_point_index].to_vec(), epsilon);
        let rec_results2 =
            douglas_peucker_simplify(vertices[farthest_point_index..(end + 1)].to_vec(), epsilon);

        [rec_results1, rec_results2[1..rec_results2.len()].to_vec()].concat()
    } else {
        vec![vertices[0], vertices[end]]
    }
}
