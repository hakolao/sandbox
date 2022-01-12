use std::sync::Arc;

use anyhow::*;
use cgmath::{Matrix2, Rad, Vector2};
use vulkano::{
    buffer::{BufferAccess, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
        SecondaryCommandBuffer, SubpassContents,
    },
    device::{Device, Queue},
    format::Format,
    image::{ImageAccess, ImageViewAbstract},
    pipeline::graphics::input_assembly::Index,
    render_pass::{Framebuffer, RenderPass, Subpass},
    sync::GpuFuture,
};

use crate::renderer::{
    line_vertices,
    pipelines::{
        BasicDrawPipeline, CircleDrawPipeline, LineDrawPipeline, TextureDrawPipeline,
        WireframeDrawPipeline,
    },
    textured_vertex_cpu_buffers_with_indices, Camera2D, Line, Mesh,
};

pub struct Pipelines {
    line: LineDrawPipeline,
    texture: TextureDrawPipeline,
    #[allow(unused)]
    wireframe: WireframeDrawPipeline,
    basic: BasicDrawPipeline,
    circle: CircleDrawPipeline,
}

/// System that contains the necessary facilities for rendering a single frame.
pub struct RenderPassDeferred {
    gfx_queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    pipelines: Pipelines,
}

impl RenderPassDeferred {
    pub fn new(gfx_queue: Arc<Queue>, final_output_format: Format) -> Result<RenderPassDeferred> {
        let render_pass = vulkano::ordered_passes_renderpass!(gfx_queue.device().clone(),
            attachments: {
                final_color: {
                    load: Clear,
                    store: Store,
                    format: final_output_format,
                    samples: 1,
                }
            },
            // ToDo: Add more passes when needed
            passes: [
                {
                    color: [final_color],
                    depth_stencil: {},
                    input: []
                }
            ]
        )?;
        let deferred_subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        let pipelines = Pipelines {
            line: LineDrawPipeline::new(gfx_queue.clone(), deferred_subpass.clone())?,
            texture: TextureDrawPipeline::new(gfx_queue.clone(), deferred_subpass.clone())?,
            wireframe: WireframeDrawPipeline::new(gfx_queue.clone(), deferred_subpass.clone())?,
            basic: BasicDrawPipeline::new(gfx_queue.clone(), deferred_subpass.clone())?,
            circle: CircleDrawPipeline::new(gfx_queue.clone(), deferred_subpass)?,
        };

        Ok(RenderPassDeferred {
            gfx_queue,
            render_pass: render_pass as Arc<_>,
            pipelines,
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        self.gfx_queue.device()
    }

    pub fn queue(&self) -> &Arc<Queue> {
        &self.gfx_queue
    }

    #[inline]
    pub fn deferred_subpass(&self) -> Subpass {
        Subpass::from(self.render_pass.clone(), 0).unwrap()
    }

    pub fn frame<F>(
        &mut self,
        clear_color: [f32; 4],
        before_future: F,
        final_image: Arc<dyn ImageViewAbstract + 'static>,
        camera: Camera2D,
    ) -> Result<Frame>
    where
        F: GpuFuture + 'static,
    {
        let _img_dims = final_image.image().dimensions().width_height();
        // Update other buffers sizes here if img dims changed...
        let framebuffer = Framebuffer::start(self.render_pass.clone())
            .add(final_image)?
            .build()?;
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )?;
        command_buffer_builder.begin_render_pass(
            framebuffer.clone(),
            SubpassContents::SecondaryCommandBuffers,
            vec![clear_color.into()],
        )?;
        Ok(Frame {
            system: self,
            before_main_cb_future: Some(before_future.boxed()),
            framebuffer,
            num_pass: 0,
            command_buffer_builder: Some(command_buffer_builder),
            camera,
        })
    }
}

pub struct Frame<'a> {
    system: &'a mut RenderPassDeferred,
    num_pass: u8,
    before_main_cb_future: Option<Box<dyn GpuFuture>>,
    framebuffer: Arc<Framebuffer>,
    command_buffer_builder: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    camera: Camera2D,
}

impl<'a> Frame<'a> {
    pub fn next_pass<'f>(&'f mut self) -> Result<Option<Pass<'f, 'a>>> {
        Ok(
            match {
                let current_pass = self.num_pass;
                self.num_pass += 1;
                current_pass
            } {
                0 => Some(Pass::Deferred(DrawPass {
                    frame: self,
                })),
                1 => {
                    // ToDo; Once you add more subpasses, remember to go to those...
                    // self.command_buffer_builder
                    //     .as_mut()
                    //     .unwrap()
                    //     .next_subpass(SubpassContents::SecondaryCommandBuffers)?;
                    self.command_buffer_builder
                        .as_mut()
                        .unwrap()
                        .end_render_pass()?;
                    let command_buffer = self.command_buffer_builder.take().unwrap().build()?;

                    let after_main_cb = self
                        .before_main_cb_future
                        .take()
                        .unwrap()
                        .then_execute(self.system.gfx_queue.clone(), command_buffer)?;
                    Some(Pass::Finished(after_main_cb.boxed()))
                }
                _ => None,
            },
        )
    }

    /// Appends a command that executes a secondary command buffer that performs drawing.
    #[inline]
    pub fn execute<C>(&mut self, command_buffer: C) -> Result<()>
    where
        C: SecondaryCommandBuffer + Send + Sync + 'static,
    {
        self.command_buffer_builder
            .as_mut()
            .unwrap()
            .execute_commands(command_buffer)?;
        Ok(())
    }
}

/// Struct provided to the user that allows them to customize or handle the pass.
pub enum Pass<'f, 's: 'f> {
    Deferred(DrawPass<'f, 's>),
    Finished(Box<dyn GpuFuture>),
}

/// Allows the user to draw objects on the scene.
pub struct DrawPass<'f, 's: 'f> {
    frame: &'f mut Frame<'s>,
}

impl<'f, 's: 'f> DrawPass<'f, 's> {
    /// Appends a command that executes a secondary command buffer that performs drawing.
    #[inline]
    pub fn execute<C>(&mut self, command_buffer: C) -> Result<()>
    where
        C: SecondaryCommandBuffer + Send + Sync + 'static,
    {
        self.frame
            .command_buffer_builder
            .as_mut()
            .unwrap()
            .execute_commands(command_buffer)?;
        Ok(())
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        self.frame.system.gfx_queue.device()
    }

    #[inline]
    pub fn queue(&self) -> &Arc<Queue> {
        &self.frame.system.gfx_queue
    }

    /// Returns the dimensions in pixels of the viewport.
    #[inline]
    pub fn viewport_dimensions(&self) -> [u32; 2] {
        let dims = self.frame.framebuffer.dimensions();
        [dims[0], dims[1]]
    }

    /// Returns the camera can be used to turn world coordinates into 2D coordinates on the framebuffer.
    #[allow(dead_code)]
    #[inline]
    pub fn camera(&self) -> Camera2D {
        self.frame.camera
    }

    pub fn draw_circle(&mut self, pos: Vector2<f32>, radius: f32, color: [f32; 4]) -> Result<()> {
        let dims = self.frame.framebuffer.dimensions();
        let cb = self.frame.system.pipelines.circle.draw(
            [dims[0], dims[1]],
            self.camera().world_to_screen(),
            pos,
            radius,
            color,
        )?;
        self.execute(cb)
    }

    pub fn draw_line(&mut self, line: Line) -> Result<()> {
        self.draw_lines(&[line])
    }

    pub fn draw_lines(&mut self, lines: &[Line]) -> Result<()> {
        let (vertices, indices) = line_vertices(lines);
        let (vertices_buf, indices_buf) =
            textured_vertex_cpu_buffers_with_indices(self.device(), vertices, indices, false)?;
        let dims = self.frame.framebuffer.dimensions();
        let cb = self.frame.system.pipelines.line.draw_indexed(
            [dims[0], dims[1]],
            self.camera().world_to_screen(),
            vertices_buf,
            indices_buf,
        )?;
        self.execute(cb)
    }

    pub fn draw_lines_from_buffers_indexed<
        V,
        Vb: BufferAccess + TypedBufferAccess<Content = [V]> + Send + Sync + 'static,
        Ib: BufferAccess + TypedBufferAccess<Content = [I]> + Send + Sync + 'static,
        I: Index + 'static,
    >(
        &mut self,
        vertices: Arc<Vb>,
        indices: Arc<Ib>,
    ) -> Result<()> {
        let dims = self.frame.framebuffer.dimensions();
        let cb = self.frame.system.pipelines.line.draw_indexed(
            [dims[0], dims[1]],
            self.camera().world_to_screen(),
            vertices,
            indices,
        )?;
        self.execute(cb)
    }

    pub fn draw_lines_from_buffers<
        V,
        Vb: BufferAccess + TypedBufferAccess<Content = [V]> + Send + Sync + 'static,
    >(
        &mut self,
        vertices: Arc<Vb>,
    ) -> Result<()> {
        let dims = self.frame.framebuffer.dimensions();
        let cb = self.frame.system.pipelines.line.draw(
            [dims[0], dims[1]],
            self.camera().world_to_screen(),
            vertices,
        )?;
        self.execute(cb)
    }

    pub fn draw_texture(
        &mut self,
        pos: Vector2<f32>,
        width: f32,
        height: f32,
        rotation: f32,
        texture: Arc<dyn ImageViewAbstract + 'static>,
        invert_y: bool,
        is_alpha: bool,
    ) -> Result<()> {
        let dims = self.frame.framebuffer.dimensions();
        let cb = self.frame.system.pipelines.texture.draw_texture_on_quad(
            [dims[0], dims[1]],
            self.camera().world_to_screen(),
            pos,
            width,
            height,
            Matrix2::from_angle(Rad(rotation)),
            texture,
            invert_y,
            is_alpha,
        )?;
        self.execute(cb)
    }

    pub fn draw_mesh_with_texture(
        &mut self,
        mesh: &Mesh,
        pos: Vector2<f32>,
        angle: f32,
        texture: Arc<dyn ImageViewAbstract + 'static>,
        is_alpha: bool,
    ) -> Result<()> {
        let vertices = mesh.vertices.clone();
        let indices = mesh.indices.clone();
        let dims = self.frame.framebuffer.dimensions();
        let cb = self.frame.system.pipelines.texture.draw_mesh(
            [dims[0], dims[1]],
            self.camera().world_to_screen(),
            pos,
            Matrix2::from_angle(Rad(angle)),
            texture,
            vertices,
            indices,
            is_alpha,
        )?;
        self.execute(cb)
    }

    pub fn draw_mesh(&mut self, mesh: &Mesh, pos: Vector2<f32>, angle: f32) -> Result<()> {
        let vertices = mesh.vertices.clone();
        let indices = mesh.indices.clone();
        let dims = self.frame.framebuffer.dimensions();
        let cb = self.frame.system.pipelines.basic.draw_mesh(
            [dims[0], dims[1]],
            self.camera().world_to_screen(),
            pos,
            Matrix2::from_angle(Rad(angle)),
            vertices,
            indices,
        )?;
        self.execute(cb)
    }
}
