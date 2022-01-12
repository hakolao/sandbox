use cgmath::{Matrix4, Point3, SquareMatrix, Transform, Vector2, Vector3};

// c1r1: y flipped for vulkan
#[rustfmt::skip]
pub const OPENGL_TO_VULKAN_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Camera2D {
    pos: Vector2<f32>,
    aspect_ratio: f32,
    near: f32,
    far: f32,
    zoom: f32,
}

impl Camera2D {
    pub fn new(pos: Vector2<f32>, aspect_ratio: f32, zoom: f32) -> Camera2D {
        Camera2D {
            pos,
            aspect_ratio,
            near: 0.001,
            far: 10000.0,
            zoom,
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    #[allow(unused)]
    pub fn zoom_to_fit_canvas(&mut self, canvas_world_size: f32) {
        self.reset_zoom();
        self.zoom(2.0 / canvas_world_size);
    }

    #[allow(unused)]
    pub fn zoom_to_fit_horizontal_pixels(
        &mut self,
        canvas_size_pixels: u32,
        canvas_world_size: f32,
        pixels_visible_horizontal: u32,
        ar: f32,
    ) {
        self.reset_zoom();
        let multiplier = pixels_visible_horizontal as f32 / canvas_size_pixels as f32;
        self.zoom(2.0 / (multiplier * canvas_world_size / ar));
    }

    #[allow(unused)]
    pub fn zoom_to_fit_vertical_pixels(
        &mut self,
        canvas_size_pixels: u32,
        canvas_world_size: f32,
        pixels_visible_vertical: u32,
        ar: f32,
    ) {
        self.reset_zoom();
        let multiplier = pixels_visible_vertical as f32 / canvas_size_pixels as f32;
        self.zoom(2.0 / (multiplier * canvas_world_size * ar));
    }

    pub fn zoom(&mut self, zoom: f32) {
        self.zoom *= zoom;
    }

    pub fn reset_zoom(&mut self) {
        self.zoom(1.0 / self.zoom);
    }

    pub fn pos(&self) -> Vector2<f32> {
        self.pos
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    pub fn zoom_level(&self) -> f32 {
        self.zoom
    }

    /// Updates camera position
    pub fn set_pos(&mut self, world_pos: Vector2<f32>) {
        self.pos = world_pos;
    }

    /// Translates camera position
    pub fn translate(&mut self, translation: Vector2<f32>) {
        self.pos += translation;
    }

    /// A view matrix
    pub fn view_mat(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(
            Point3::new(self.pos.x, self.pos.y, 1.0),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(0.0, 1.0, 0.0),
        )
    }

    /// Builds a projection matrix
    pub fn projection_mat(&self) -> Matrix4<f32> {
        OPENGL_TO_VULKAN_MATRIX
            * cgmath::ortho(
                -self.aspect_ratio / self.zoom,
                self.aspect_ratio / self.zoom,
                -1.0 / self.zoom,
                1.0 / self.zoom,
                self.near,
                self.far,
            )
    }

    /// A matrix4 that transforms world coordinates fo screen coordinates
    pub fn world_to_screen(&self) -> Matrix4<f32> {
        self.projection_mat() * self.view_mat()
    }

    /// A matrix4 that transforms screen coordinates fo world coordinates
    pub fn screen_to_world(&self) -> Option<Matrix4<f32>> {
        self.world_to_screen().invert()
    }

    /// Convert normalized window pos between [0.0, 1.0] to world coordinates
    pub fn screen_to_world_pos(&self, normalized_window_pos: Vector2<f32>) -> Vector2<f32> {
        self.world_to_screen()
            .inverse_transform_vector(Vector3::new(
                normalized_window_pos.x * 2.0 - 1.0,
                normalized_window_pos.y * 2.0 - 1.0,
                0.0,
            ))
            .unwrap()
            .truncate()
            + self.pos
    }
}

impl Default for Camera2D {
    fn default() -> Camera2D {
        Camera2D::new(Vector2::new(0.0, 0.0), 1.0, 1.0)
    }
}
