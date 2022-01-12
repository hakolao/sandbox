use cgmath::Vector2;

fn depth_first_label_mark(
    bitmap: &[f64],
    labels: &mut Vec<u32>,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    current_label: u32,
    min_x: &mut i32,
    min_y: &mut i32,
    max_x: &mut i32,
    max_y: &mut i32,
) {
    if x < 0 || x == width as i32 || y < 0 || y == height as i32 {
        return;
    };
    let index = (y * width as i32 + x) as usize;
    if labels[index] != 0 || bitmap[index] == 0.0 {
        return;
    }
    labels[index] = current_label;
    // Find maxes
    *min_x = (*min_x).min(x);
    *min_y = (*min_y).min(y);
    *max_x = (*max_x).max(x);
    *max_y = (*max_y).max(y);
    for (neigh_x, neigh_y) in &[
        (x - 1, y - 1),
        (x, y - 1),
        (x + 1, y - 1),
        (x + 1, y),
        (x + 1, y + 1),
        (x, y + 1),
        (x - 1, y + 1),
        (x - 1, y),
    ] {
        depth_first_label_mark(
            bitmap,
            labels,
            width,
            height,
            *neigh_x,
            *neigh_y,
            current_label,
            min_x,
            min_y,
            max_x,
            max_y,
        );
    }
}

/// Go through inputted bitmap & extract new bitmaps of connected pixels (components)
pub fn extract_connected_components_from_bitmap(
    bitmap: &[f64],
    width: u32,
    height: u32,
    // (new_bitmap, width, height, start_pos relative to original)
) -> Vec<(Vec<f64>, u32, u32, Vector2<i32>)> {
    let mut results = vec![];
    let mut labels = vec![0; (width * height) as usize];
    let mut current_label = 0;
    let mut min_x = width as i32 - 1;
    let mut min_y = height as i32 - 1;
    let mut max_x = 0;
    let mut max_y = 0;
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            if labels[index] == 0 && bitmap[index] == 1.0 {
                current_label += 1;
                depth_first_label_mark(
                    bitmap,
                    &mut labels,
                    width,
                    height,
                    x as i32,
                    y as i32,
                    current_label,
                    &mut min_x,
                    &mut min_y,
                    &mut max_x,
                    &mut max_y,
                );
                // Based on labeling, extract our bitmap to results
                let new_width = (max_x - min_x) as u32 + 1;
                let new_height = (max_y - min_y) as u32 + 1;
                let mut new_bitmap = vec![0.0; (new_width * new_height) as usize];
                for new_y in 0..new_height {
                    for new_x in 0..new_width {
                        let index = (new_y * new_width + new_x) as usize;
                        let old_index = ((min_y + new_y as i32) * width as i32
                            + (min_x + new_x as i32))
                            as usize;
                        if labels[old_index] == current_label {
                            new_bitmap[index] = 1.0;
                        }
                    }
                }
                let min = Vector2::new(min_x, min_y);
                results.push((new_bitmap, new_width, new_height, min));
                // Reset
                min_x = width as i32 - 1;
                min_y = height as i32 - 1;
                max_x = 0;
                max_y = 0;
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_area_extraction() {
        #[rustfmt::skip]
        let input = vec![
            1.0, 0.0, 0.0, 1.0,
            1.0, 1.0, 0.0, 1.0,
            1.0, 1.0, 0.0, 1.0,
            1.0, 0.0, 0.0, 1.0,
        ];
        let input_width = 4;
        let input_height = 4;
        #[rustfmt::skip]
        let expected_first = vec![
            1.0, 0.0,
            1.0, 1.0,
            1.0, 1.0,
            1.0, 0.0,
        ];
        let expected_first_width = 2;
        let expected_first_height = 4;
        #[rustfmt::skip]
        let expected_second = vec![
            1.0,
            1.0,
            1.0,
            1.0,
        ];
        let expected_second_width = 1;
        let expected_second_height = 4;

        let result = extract_connected_components_from_bitmap(&input, input_width, input_height);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            (
                expected_first,
                expected_first_width,
                expected_first_height,
                Vector2::new(0, 0)
            )
        );
        assert_eq!(
            result[1],
            (
                expected_second,
                expected_second_width,
                expected_second_height,
                Vector2::new(3, 0)
            )
        );
    }
}
