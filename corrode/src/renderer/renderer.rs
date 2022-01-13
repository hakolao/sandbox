use core::result::Result::Ok;
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::*;
use egui_winit_vulkano::texture_from_file;
#[cfg(target_os = "macos")]
use vulkano::instance::InstanceCreationError;
use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceExtensions, Features, Queue,
    },
    format::Format,
    image::{
        view::ImageView, ImageAccess, ImageCreateFlags, ImageDimensions, ImageUsage,
        ImageViewAbstract, StorageImage, SwapchainImage,
    },
    instance::{
        debug::{DebugCallback, MessageSeverity, MessageType},
        Instance, InstanceExtensions,
    },
    swapchain,
    swapchain::{
        AcquireError, ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform,
        Swapchain, SwapchainCreationError,
    },
    sync,
    sync::{FlushError, GpuFuture},
    Version,
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoop,
    window::{Fullscreen, Window, WindowBuilder},
};

use crate::{
    engine::RenderOptions,
    renderer::render_pass::{RenderPassDeferred, RenderPassPlaceOverFrame},
};

// Create vk instance
pub fn create_vk_instance() -> Arc<Instance> {
    // Add instance extensions based on needs
    let instance_extensions = InstanceExtensions {
        ext_debug_utils: true,
        ..vulkano_win::required_extensions()
    };
    // Create instance
    #[cfg(target_os = "macos")]
    {
        let layers = if std::env::var("VULKAN_VALIDATION").is_ok() {
            vec!["VK_LAYER_KHRONOS_validation"]
        } else {
            vec![]
        };
        match Instance::new(None, Version::V1_2, &instance_extensions, layers) {
            Err(e) => {
                match e {
                    InstanceCreationError::LoadingError(le) => {
                        error!("{:?}, Did you install vulkanSDK from https://vulkan.lunarg.com/sdk/home?", le);
                        Err(le).expect("")
                    }
                    _ => Err(e).expect("Failed to create instance"),
                }
            }
            Ok(i) => i,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let layers = if std::env::var("VULKAN_VALIDATION").is_ok() {
            vec!["VK_LAYER_LUNARG_standard_validation"]
        } else {
            vec![]
        };
        Instance::new(None, Version::V1_2, &instance_extensions, layers)
            .expect("Failed to create instance")
    }
}

// Create vk debug call back (to exists outside renderer)
pub fn create_vk_debug_callback(instance: &Arc<Instance>) -> DebugCallback {
    // Create debug callback for printing vulkan errors and warnings
    let severity = if std::env::var("VULKAN_VALIDATION").is_ok() {
        MessageSeverity {
            error: true,
            warning: true,
            information: true,
            verbose: true,
        }
    } else {
        MessageSeverity::none()
    };

    let ty = MessageType::all();
    DebugCallback::new(instance, severity, ty, |msg| {
        let severity = if msg.severity.error {
            "error"
        } else if msg.severity.warning {
            "warning"
        } else if msg.severity.information {
            "information"
        } else if msg.severity.verbose {
            "verbose"
        } else {
            panic!("no-impl");
        };

        let ty = if msg.ty.general {
            "general"
        } else if msg.ty.validation {
            "validation"
        } else if msg.ty.performance {
            "performance"
        } else {
            panic!("no-impl");
        };

        info!(
            "{} {} {}: {}",
            msg.layer_prefix.unwrap_or("unknown"),
            ty,
            severity,
            msg.description
        );
    })
    .unwrap()
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct ImageTextureId(pub u32);

/// Default render passes, their input format is the same as swapchains, thus if you need a different
/// format (e.g. intermediate target), create your own render passes
pub struct DefaultRenderPasses {
    pub deferred: RenderPassDeferred,
    pub place_over_frame: RenderPassPlaceOverFrame,
}

/// Final render target onto which whole app is rendered
pub type FinalImageView = Arc<ImageView<SwapchainImage<Window>>>;
/// Multipurpose image view
pub type DeviceImageView = Arc<ImageView<StorageImage>>;

/// Renderer that handles all gpu side rendering
pub struct Renderer {
    _instance: Arc<Instance>,
    _debug_callback: DebugCallback,
    device: Arc<Device>,
    surface: Arc<Surface<Window>>,
    graphics_queue: Arc<Queue>,
    compute_queue: Arc<Queue>,
    swap_chain: Arc<Swapchain<Window>>,
    image_index: usize,
    final_views: Vec<FinalImageView>,
    /// Image view that is to be rendered with our pipeline.
    /// (bool refers to whether it should get resized with swapchain resize)
    interim_image_views: HashMap<usize, (DeviceImageView, bool)>,
    // Texture cache for textures and their descriptor sets
    image_textures: HashMap<ImageTextureId, Arc<dyn ImageViewAbstract + 'static>>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    pub render_passes: DefaultRenderPasses,
    _clear_color: [f32; 4],
    is_fullscreen: bool,
    device_name: String,
    device_type: PhysicalDeviceType,
    max_mem_gb: f32,
}

impl Renderer {
    /// Creates a new GPU renderer for window with given parameters
    pub fn new<E>(event_loop: &EventLoop<E>, opts: RenderOptions) -> Result<Self> {
        info!("Creating renderer for window size {:?}", opts.window_size);
        let instance = create_vk_instance();
        let debug_callback = create_vk_debug_callback(&instance);
        // Get desired device
        let physical_device = PhysicalDevice::enumerate(&instance)
            .min_by_key(|p| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
            })
            .unwrap();
        let device_name = physical_device.properties().device_name.to_string();
        #[cfg(target_os = "windows")]
        let max_mem_gb = physical_device.properties().max_memory_allocation_count as f32 * 9.31e-4;
        #[cfg(not(target_os = "windows"))]
        let max_mem_gb = physical_device.properties().max_memory_allocation_count as f32 * 9.31e-10;
        info!(
            "Using device {}, type: {:?}, mem: {:.2} gb",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
            max_mem_gb,
        );
        let device_type = physical_device.properties().device_type;
        // Create rendering surface along with window
        let surface = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::new(
                opts.window_size[0],
                opts.window_size[1],
            ))
            .with_title(opts.title)
            .build_vk_surface(event_loop, instance.clone())
            .context("Failed to create vulkan surface & window")?;

        // Create device
        let (device, graphics_queue, compute_queue) =
            Self::create_device(physical_device, surface.clone())?;
        // Create swap chain & frame(s) to which we'll render
        let (swap_chain, final_images) = Self::create_swap_chain(
            surface.clone(),
            physical_device,
            device.clone(),
            graphics_queue.clone(),
            if opts.v_sync {
                PresentMode::Fifo
            } else {
                PresentMode::Immediate
            },
        )?;
        let previous_frame_end = Some(sync::now(device.clone()).boxed());
        let is_fullscreen = swap_chain.surface().window().fullscreen().is_some();
        let image_format = final_images.first().unwrap().format();
        info!("Swapchain format {:?}", image_format);
        let render_passes = DefaultRenderPasses {
            deferred: RenderPassDeferred::new(graphics_queue.clone(), image_format)?,
            place_over_frame: RenderPassPlaceOverFrame::new(graphics_queue.clone(), image_format)?,
        };

        Ok(Self {
            _instance: instance,
            _debug_callback: debug_callback,
            device,
            surface,
            graphics_queue,
            compute_queue,
            swap_chain,
            image_index: 0,
            final_views: final_images,
            interim_image_views: HashMap::new(),
            image_textures: HashMap::new(),
            previous_frame_end,
            recreate_swapchain: false,
            render_passes,
            _clear_color: [0.0; 4],
            is_fullscreen,
            device_name,
            device_type,
            max_mem_gb,
        })
    }

    /*================
    STATIC FUNCTIONS
    =================*/

    /// Creates vulkan device with required queue families and required extensions
    fn create_device(
        physical: PhysicalDevice,
        surface: Arc<Surface<Window>>,
    ) -> Result<(Arc<Device>, Arc<Queue>, Arc<Queue>)> {
        let (gfx_index, queue_family_graphics) = physical
            .queue_families()
            .enumerate()
            .find(|&(_i, q)| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
            .context("couldn't find a graphical queue family")?;
        let compute_family_data = physical
            .queue_families()
            .enumerate()
            .find(|&(i, q)| i != gfx_index && q.supports_compute());

        // Add device extensions based on needs,
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        // Add device features
        let features = Features {
            fill_mode_non_solid: true,
            ..Features::none()
        };

        Ok(
            if let Some((_compute_index, queue_family_compute)) = compute_family_data {
                let (device, mut queues) = {
                    Device::new(
                        physical,
                        &features,
                        &physical.required_extensions().union(&device_extensions),
                        [(queue_family_graphics, 1.0), (queue_family_compute, 0.5)]
                            .iter()
                            .cloned(),
                    )
                    .context("failed to create device")?
                };
                let gfx_queue = queues.next().unwrap();
                let compute_queue = queues.next().unwrap();
                (device, gfx_queue, compute_queue)
            } else {
                let (device, mut queues) = {
                    Device::new(
                        physical,
                        &features,
                        &physical.required_extensions().union(&device_extensions),
                        [(queue_family_graphics, 1.0)].iter().cloned(),
                    )
                    .context("failed to create device")?
                };
                let gfx_queue = queues.next().unwrap();
                let compute_queue = gfx_queue.clone();
                (device, gfx_queue, compute_queue)
            },
        )
    }

    /// Creates swapchain and swapchain images
    fn create_swap_chain(
        surface: Arc<Surface<Window>>,
        physical: PhysicalDevice,
        device: Arc<Device>,
        queue: Arc<Queue>,
        present_mode: PresentMode,
    ) -> Result<(Arc<Swapchain<Window>>, Vec<FinalImageView>)> {
        let caps = surface.capabilities(physical).unwrap();
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        let dimensions: [u32; 2] = surface.window().inner_size().into();
        let (swap_chain, images) = Swapchain::start(device, surface)
            .num_images(caps.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .usage(ImageUsage::color_attachment())
            .sharing_mode(&queue)
            .composite_alpha(alpha)
            .transform(SurfaceTransform::Identity)
            .present_mode(present_mode)
            .fullscreen_exclusive(FullscreenExclusive::Default)
            .clipped(true)
            .color_space(ColorSpace::SrgbNonLinear)
            .layers(1)
            .build()?;
        let images = images
            .into_iter()
            .map(|image| ImageView::new(image).unwrap())
            .collect::<Vec<_>>();
        Ok((swap_chain, images))
    }

    fn create_image_texture_id() -> ImageTextureId {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);

        ImageTextureId(id as u32)
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn device_type(&self) -> PhysicalDeviceType {
        self.device_type
    }

    pub fn max_mem_gb(&self) -> f32 {
        self.max_mem_gb
    }

    /// Adds texture to image_textures for later use, returns ImageTextureId
    pub fn add_texture_from_file_bytes(
        &mut self,
        image_file_as_bytes: &[u8],
    ) -> Result<ImageTextureId> {
        let image_view = self.create_image_from_file_bytes(image_file_as_bytes)?;
        let new_id = Self::create_image_texture_id();
        self.add_image_texture(new_id, image_view);
        Ok(new_id)
    }

    /// Adds texture to image_textures for later use, returns ImageTextureId
    pub fn add_texture_from_image_view(
        &mut self,
        image_view: Arc<dyn ImageViewAbstract + 'static>,
    ) -> Result<ImageTextureId> {
        let new_id = Self::create_image_texture_id();
        self.add_image_texture(new_id, image_view);
        Ok(new_id)
    }

    /// Adds texture to image_textures for later use, returns ImageTextureId
    pub fn update_texture_from_image_view(
        &mut self,
        image_view: Arc<dyn ImageViewAbstract + 'static>,
        texture_id: ImageTextureId,
    ) -> Result<()> {
        self.add_image_texture(texture_id, image_view);
        Ok(())
    }

    fn add_image_texture(
        &mut self,
        key: ImageTextureId,
        texture: Arc<dyn ImageViewAbstract + 'static>,
    ) {
        self.image_textures.insert(key, texture);
    }

    /// Get image texture (if exists, else panic)
    pub fn get_image_texture(&self, key: &ImageTextureId) -> Arc<dyn ImageViewAbstract + 'static> {
        self.image_textures.get(key).unwrap().clone()
    }

    /// Creates image view from image file bytes
    fn create_image_from_file_bytes(
        &self,
        file_bytes: &[u8],
    ) -> Result<Arc<dyn ImageViewAbstract + 'static>> {
        let image_view = texture_from_file(self.graphics_queue(), file_bytes, self.image_format())?;
        Ok(image_view)
    }

    /// Return default image format for images (swapchain format may differ)
    pub fn image_format(&self) -> Format {
        Format::R8G8B8A8_UNORM
    }

    /// Return default image format for images (swapchain format may differ)
    pub fn swapchain_format(&self) -> Format {
        self.final_views[self.image_index].format()
    }

    /// Returns the index of last swapchain image that is the next render target
    /// All camera views will render onto their image at the same index
    pub fn image_index(&self) -> usize {
        self.image_index
    }

    /// Access device
    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    /// Access rendering queue
    pub fn graphics_queue(&self) -> Arc<Queue> {
        self.graphics_queue.clone()
    }

    /// Access rendering queue
    pub fn compute_queue(&self) -> Arc<Queue> {
        self.compute_queue.clone()
    }

    /// Render target surface
    pub fn surface(&self) -> Arc<Surface<Window>> {
        self.surface.clone()
    }

    /// Winit window
    pub fn window(&self) -> &Window {
        self.surface.window()
    }

    pub fn window_size(&self) -> [u32; 2] {
        let size = self.window().inner_size();
        [size.width, size.height]
    }

    /// Size of the final swapchain image (surface)
    pub fn final_image_size(&self) -> [u32; 2] {
        self.final_views[0].image().dimensions().width_height()
    }

    /// Return final image which can be used as a render pipeline target
    pub fn final_image(&self) -> FinalImageView {
        self.final_views[self.image_index].clone()
    }

    /*================
    View related functions
    =================*/

    /// Return scale factor accounted window size
    pub fn resolution(&self) -> [u32; 2] {
        let size = self.window().inner_size();
        let scale_factor = self.window().scale_factor();
        [
            (size.width as f64 / scale_factor) as u32,
            (size.height as f64 / scale_factor) as u32,
        ]
    }

    pub fn aspect_ratio(&self) -> f32 {
        let dims = self.window_size();
        dims[0] as f32 / dims[1] as f32
    }

    /// This should be called on resize to e.g. replace main image view (with cuda interoperable view)
    pub fn force_replace_interim_image_view(&mut self, key: usize, image_view: DeviceImageView) {
        self.interim_image_views.insert(key, (image_view, false));
    }

    /// Add interim image view that can be used to render e.g. camera views or other views using
    /// the render pipeline. Not giving a view size ensures the image view follows swapchain (window).
    /// Provide format: It should be renderer.image_format() for full screen images (used in e.g. `compute` examples
    /// And for deferred (default) pipelines use `renderer.swapchain_format()`.
    pub fn add_image_target(
        &mut self,
        key: usize,
        view_size: Option<[u32; 2]>,
        format: Format,
    ) -> Result<()> {
        let size = if view_size.is_some() {
            view_size.unwrap()
        } else {
            self.final_image_size()
        };
        let image = create_device_image(self.graphics_queue.clone(), size, format)?;
        self.interim_image_views
            .insert(key, (image, view_size.is_none()));
        Ok(())
    }

    /// Get interim image view by key (for render calls or for registering as texture for egui)
    pub fn get_image_target(&mut self, key: usize) -> DeviceImageView {
        self.interim_image_views.get(&key).unwrap().clone().0
    }

    /// Get interim image view by key (for render calls or for registering as texture for egui)
    pub fn has_image_target(&mut self, key: usize) -> bool {
        self.interim_image_views.get(&key).is_some()
    }

    pub fn remove_image_target(&mut self, key: usize) -> Result<()> {
        self.interim_image_views.remove(&key);
        Ok(())
    }

    /*================
    Updates
    =================*/

    pub fn toggle_fullscreen(&mut self) {
        self.is_fullscreen = !self.is_fullscreen;
        self.window().set_fullscreen(if self.is_fullscreen {
            Some(Fullscreen::Borderless(self.window().current_monitor()))
        } else {
            None
        });
    }

    /// Resize swapchain and camera view images
    pub fn resize(&mut self) {
        self.recreate_swapchain = true;
    }

    /*================
    RENDERING
    =================*/

    /// Acquires next swapchain image and increments image index
    /// This is the first to call in render orchestration.
    /// Returns a gpu future representing the time after which the swapchain image has been acquired
    /// and previous frame ended.
    /// After this, execute command buffers and return future from them to `finish_frame`.
    pub(crate) fn start_frame(&mut self) -> Result<Box<dyn GpuFuture>> {
        // Recreate swap chain if needed (when resizing of window occurs or swapchain is outdated)
        // Also resize render views if needed
        if self.recreate_swapchain {
            self.recreate_swapchain_and_views()?;
        }

        // Acquire next image in the swapchain
        let (image_num, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swap_chain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Err(anyhow!(AcquireError::OutOfDate));
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };
        if suboptimal {
            self.recreate_swapchain = true;
        }
        // Update our image index
        self.image_index = image_num;

        let future = self.previous_frame_end.take().unwrap().join(acquire_future);

        Ok(future.boxed())
    }

    /// Finishes render by presenting the swapchain
    pub(crate) fn finish_frame(&mut self, after_future: Box<dyn GpuFuture>) {
        let future = after_future
            .then_swapchain_present(
                self.graphics_queue.clone(),
                self.swap_chain.clone(),
                self.image_index,
            )
            .then_signal_fence_and_flush();
        match future {
            Ok(future) => {
                // A hack to prevent OutOfMemory error on Nvidia :(
                // https://github.com/vulkano-rs/vulkano/issues/627
                match future.wait(None) {
                    Ok(x) => x,
                    Err(err) => error!("{:?}", err),
                }
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                error!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }

    /// Swapchain is recreated when resized
    /// Swapchain images also get recreated
    fn recreate_swapchain_and_views(&mut self) -> Result<()> {
        let dimensions: [u32; 2] = self.window().inner_size().into();
        let (new_swapchain, new_images) =
            match self.swap_chain.recreate().dimensions(dimensions).build() {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => {
                    error!(
                        "{}",
                        SwapchainCreationError::UnsupportedDimensions.to_string()
                    );
                    return Ok(());
                }
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

        self.swap_chain = new_swapchain;
        let new_images = new_images
            .into_iter()
            .map(|image| ImageView::new(image).unwrap())
            .collect::<Vec<_>>();
        self.final_views = new_images;
        // Resize images that follow swapchain size
        let resizable_views = self
            .interim_image_views
            .iter()
            .filter(|(_, (_img, follow_swapchain))| *follow_swapchain)
            .map(|c| *c.0)
            .collect::<Vec<usize>>();
        for i in resizable_views {
            let format = self.get_image_target(i).format();
            self.remove_image_target(i)?;
            self.add_image_target(i, None, format)?;
        }
        self.recreate_swapchain = false;
        Ok(())
    }
}

/// Creates a storage image on device
#[allow(unused)]
pub fn create_device_image(
    queue: Arc<Queue>,
    size: [u32; 2],
    format: Format,
) -> Result<DeviceImageView> {
    let dims = ImageDimensions::Dim2d {
        width: size[0],
        height: size[1],
        array_layers: 1,
    };
    let flags = ImageCreateFlags::none();
    Ok(ImageView::new(StorageImage::with_usage(
        queue.device().clone(),
        dims,
        format,
        ImageUsage {
            sampled: true,
            storage: true,
            color_attachment: true,
            transfer_destination: true,
            ..ImageUsage::none()
        },
        flags,
        Some(queue.family()),
    )?)?)
}

#[allow(unused)]
pub fn create_device_image_with_usage(
    queue: Arc<Queue>,
    size: [u32; 2],
    format: Format,
    usage: ImageUsage,
) -> Result<DeviceImageView> {
    let dims = ImageDimensions::Dim2d {
        width: size[0],
        height: size[1],
        array_layers: 1,
    };
    let flags = ImageCreateFlags::none();
    Ok(ImageView::new(StorageImage::with_usage(
        queue.device().clone(),
        dims,
        format,
        usage,
        flags,
        Some(queue.family()),
    )?)?)
}

#[cfg(test)]
mod test {
    use vulkano::{
        format::Format,
        image::{ImageDimensions, ImmutableImage, MipmapsCount},
    };

    use crate::renderer::render_test_helper::test_setup;

    #[test]
    fn test_debug_layer() {
        let (_device, gfx_queue, _dbg) = test_setup();

        // Create an image in order to generate some additional logging:
        let pixel_format = Format::R8G8B8A8_UINT;
        let dimensions = ImageDimensions::Dim2d {
            width: 4096,
            height: 4096,
            array_layers: 1,
        };
        const DATA: [[u8; 4]; 4096 * 4096] = [[0; 4]; 4096 * 4096];
        let _ = ImmutableImage::from_iter(
            DATA.iter().cloned(),
            dimensions,
            MipmapsCount::One,
            pixel_format,
            gfx_queue.clone(),
        )
        .unwrap();
    }
}
