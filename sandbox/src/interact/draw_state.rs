use std::collections::HashSet;

use cgmath::{MetricSpace, Vector2};

use crate::sim::canvas_pos_to_world_pos;

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasDrawState {
    pub current: Option<Vector2<i32>>,
    pub prev: Option<Vector2<i32>>,
    pub pixels: HashSet<Vector2<i32>>,
    pub min: Option<Vector2<i32>>,
    pub max: Option<Vector2<i32>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrawTransition {
    Start(Vector2<i32>, f32),
    Draw(Vector2<i32>, f32),
    End(Vector2<i32>, f32),
}

impl CanvasDrawState {
    pub fn new() -> Self {
        CanvasDrawState {
            current: None,
            prev: None,
            pixels: HashSet::new(),
            min: None,
            max: None,
        }
    }

    pub fn transition(
        &mut self,
        draw_event: DrawTransition,
        is_square: bool,
    ) -> Option<CanvasDrawState> {
        match draw_event {
            DrawTransition::Start(v, size) => {
                self.pixels.clear();
                self.current = Some(v);
                if !is_square {
                    self.add_to_pixels_by_radius(v, size);
                } else {
                    self.add_to_pixels_by_square(v, size);
                }
                None
            }
            DrawTransition::Draw(v, size) => {
                self.prev = self.current;
                self.current = Some(v);
                let line = self.get_line();
                for pos in line {
                    if !is_square {
                        self.add_to_pixels_by_radius(pos, size);
                    } else {
                        self.add_to_pixels_by_square(pos, size);
                    }
                }
                None
            }
            DrawTransition::End(v, size) => {
                self.prev = self.current;
                self.current = Some(v);
                if !is_square {
                    self.add_to_pixels_by_radius(v, size);
                } else {
                    self.add_to_pixels_by_square(v, size);
                }
                let result = self.clone();
                self.prev = None;
                self.current = None;
                self.min = None;
                self.max = None;
                Some(result)
            }
        }
    }

    fn update_bounds(&mut self, pos: Vector2<i32>) {
        if self.min.is_none() {
            self.min = Some(pos)
        } else {
            let current_min = self.min.unwrap();
            self.min = Some(Vector2::new(
                pos.x.min(current_min.x),
                pos.y.min(current_min.y),
            ))
        }
        if self.max.is_none() {
            self.max = Some(pos)
        } else {
            let current_max = self.max.unwrap();
            self.max = Some(Vector2::new(
                pos.x.max(current_max.x),
                pos.y.max(current_max.y),
            ))
        }
    }

    fn add_to_pixels_by_radius(&mut self, pos: Vector2<i32>, radius: f32) {
        let y_start = pos.y - radius as i32;
        let y_end = pos.y + radius as i32;
        let x_start = pos.x - radius as i32;
        let x_end = pos.x + radius as i32;
        for y in y_start..=y_end {
            for x in x_start..=x_end {
                if Vector2::new(x as f32, y as f32)
                    .distance(Vector2::new(pos.x as f32, pos.y as f32))
                    .round()
                    <= radius
                {
                    let pos = Vector2::new(x, y);
                    self.pixels.insert(pos);
                    self.update_bounds(pos);
                }
            }
        }
    }

    fn add_to_pixels_by_square(&mut self, pos: Vector2<i32>, size: f32) {
        let y_start = pos.y - size as i32;
        let y_end = pos.y + size as i32;
        let x_start = pos.x - size as i32;
        let x_end = pos.x + size as i32;
        for y in y_start..y_end {
            for x in x_start..x_end {
                let pos = Vector2::new(x, y);
                self.pixels.insert(pos);
                self.update_bounds(pos);
            }
        }
    }

    pub fn get_line(&self) -> Vec<Vector2<i32>> {
        let current = self.current.unwrap();
        if let Some(prev_pos) = self.prev {
            line_drawing::Bresenham::new((prev_pos.x, prev_pos.y), (current.x, current.y))
                .into_iter()
                .map(|pos| Vector2::new(pos.0, pos.1))
                .collect()
        } else {
            vec![current]
        }
    }

    #[allow(unused)]
    pub fn started(&self) -> bool {
        self.current.is_some()
    }

    #[allow(unused)]
    pub fn idle(&self) -> bool {
        !self.started()
    }

    pub fn pixels_world_pos(&self) -> Vector2<f32> {
        let half_way = (self.max.unwrap() - self.min.unwrap()) / 2;
        let local_current = self.current.unwrap() - self.min.unwrap();
        canvas_pos_to_world_pos(self.current.unwrap() - (local_current - half_way))
    }
}
