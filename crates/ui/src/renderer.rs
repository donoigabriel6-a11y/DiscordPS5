//! OpenGL/ImGui Renderer
//!
//! ImGui renderer using OpenGL for the PS5 platform.

use std::sync::Arc;
use anyhow::{Context, Result};
use glow::HasContext;
use imgui::{Context as ImGuiContext, FontSource, FontConfig, Ui};
use imgui_opengl_renderer::Renderer as ImGuiOpenGLRenderer;
use parking_lot::RwLock;
use ps5_platform::graphics::{GbmSurface, EglContext};
use tracing::{debug, info, warn};

use crate::theme::apply_discord_theme;

/// OpenGL/ImGui renderer for PS5
pub struct Renderer {
    imgui: Arc<RwLock<ImGuiContext>>,
    opengl_renderer: Option<ImGuiOpenGLRenderer>,
    gl: Option<Arc<glow::Context>>,
    surface: Option<GbmSurface>,
    egl_context: Option<EglContext>,
    last_frame_time: std::time::Instant,
    frame_count: u64,
    viewport_size: [u32; 2],
    dpi_scale: f32,
}

impl Renderer {
    /// Create a new renderer with the given GBM surface and EGL context
    pub fn new(
        surface: GbmSurface,
        egl_context: EglContext,
    ) -> Result<Self> {
        info!("Creating ImGui renderer...");

        // Create OpenGL context from EGL
        let gl = create_gl_context(&egl_context)?;

        // Create ImGui context
        let mut imgui = ImGuiContext::create();
        apply_discord_theme(&mut imgui.style_mut());

        // Load fonts
        load_fonts(&mut imgui)?;

        // Create OpenGL renderer for ImGui
        let opengl_renderer = ImGuiOpenGLRenderer::new(&mut imgui, |s| {
            gl.get_proc_address(s) as *const _
        });

        let viewport_size = [surface.width(), surface.height()];

        Ok(Self {
            imgui: Arc::new(RwLock::new(imgui)),
            opengl_renderer: Some(opengl_renderer),
            gl: Some(gl),
            surface: Some(surface),
            egl_context: Some(egl_context),
            last_frame_time: std::time::Instant::now(),
            frame_count: 0,
            viewport_size,
            dpi_scale: 1.0,
        })
    }

    /// Get the ImGui context
    pub fn imgui(&self) -> Arc<RwLock<ImGuiContext>> {
        self.imgui.clone()
    }

    /// Start a new ImGui frame
    pub fn new_frame(&mut self) -> Result<Ui> {
        let imgui = self.imgui.read();
        let io = imgui.io_mut();

        // Update delta time
        let now = std::time::Instant::now();
        let delta = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        io.delta_time = delta.max(1.0 / 120.0); // Cap at 120 FPS minimum

        // Update display size
        io.display_size = [self.viewport_size[0] as f32, self.viewport_size[1] as f32];
        io.display_framebuffer_scale = [self.dpi_scale, self.dpi_scale];

        // Drop the read lock before creating the frame
        drop(imgui);

        Ok(self.imgui.write().new_frame())
    }

    /// Render ImGui draw data to OpenGL
    pub fn render(&mut self, ui: Ui) -> Result<()> {
        let draw_data = ui.render();

        // Get OpenGL context
        let gl = self.gl.as_ref()
            .ok_or_else(|| anyhow::anyhow!("OpenGL context not available"))?;

        // Clear screen with Discord background color
        unsafe {
            gl.clear_color(0.137, 0.137, 0.145, 1.0); // Discord dark bg
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }

        // Render ImGui
        if let Some(renderer) = &mut self.opengl_renderer {
            renderer.render(draw_data)
                .context("Failed to render ImGui")?;
        }

        // Swap buffers
        if let Some(egl_context) = &self.egl_context {
            egl_context.swap_buffers()
                .context("Failed to swap buffers")?;
        }

        self.frame_count += 1;
        Ok(())
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport_size = [width, height];

        if let Some(gl) = &self.gl {
            unsafe {
                gl.viewport(0, 0, width as i32, height as i32);
            }
        }

        if let Some(surface) = &mut self.surface {
            surface.resize(width, height);
        }

        debug!("Renderer resized to {}x{}", width, height);
    }

    /// Set DPI scale factor
    pub fn set_dpi_scale(&mut self, scale: f32) {
        self.dpi_scale = scale.max(0.5).min(4.0);
    }

    /// Get current frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get viewport size
    pub fn viewport_size(&self) -> [u32; 2] {
        self.viewport_size
    }

    /// Get average FPS
    pub fn average_fps(&self) -> f32 {
        let elapsed = self.last_frame_time.elapsed().as_secs_f32();
        if elapsed > 0.0 {
            self.frame_count as f32 / elapsed
        } else {
            0.0
        }
    }

    /// Shutdown the renderer
    pub fn shutdown(&mut self) {
        info!("Shutting down renderer (frames rendered: {})", self.frame_count);

        self.opengl_renderer = None;
        self.gl = None;
        self.surface = None;
        self.egl_context = None;
    }
}

/// Create glow OpenGL context from EGL
fn create_gl_context(egl_context: &EglContext) -> Result<Arc<glow::Context>> {
    // Get function pointers from EGL
    let get_proc_address = |name: &str| {
        egl_context.get_proc_address(name) as *const _
    };

    let gl = unsafe {
        glow::Context::from_loader_function_cstr(|s| get_proc_address(s.to_str().unwrap()))
    };

    // Check OpenGL version
    let version = unsafe {
        let mut major = 0;
        let mut minor = 0;
        gl.get_parameter_i32(glow::MAJOR_VERSION, &mut major);
        gl.get_parameter_i32(glow::MINOR_VERSION, &mut minor);
        (major, minor)
    };

    info!("OpenGL version: {}.{}", version.0, version.1);

    if version.0 < 3 || (version.0 == 3 && version.1 < 2) {
        warn!("OpenGL version < 3.2, some features may not work");
    }

    Ok(Arc::new(gl))
}

/// Load fonts for ImGui
fn load_fonts(imgui: &mut ImGuiContext) -> Result<()> {
    let fonts = imgui.fonts();

    // Default font (will be used if no custom font loaded)
    fonts.add_font(&[FontSource::DefaultFontData {
        config: Some(FontConfig {
            size_pixels: 16.0,
            ..Default::default()
        }),
    }]);

    // Try to load Noto Sans (common on PS5 Linux)
    let font_paths = [
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/TTF/NotoSans-Regular.ttf",
        "/usr/local/share/fonts/NotoSans-Regular.ttf",
        "C:/Windows/Fonts/segoeui.ttf", // Windows fallback
    ];

    let mut loaded_custom = false;
    for path in &font_paths {
        if std::path::Path::new(path).exists() {
            fonts.add_font(&[FontSource::TtfData {
                data: std::fs::read(path)?,
                size_pixels: 16.0,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.5,
                    oversample_h: 2,
                    oversample_v: 2,
                    ..Default::default()
                }),
            }]);
            info!("Loaded font from: {}", path);
            loaded_custom = true;
            break;
        }
    }

    if !loaded_custom {
        warn!("No custom font found, using default");
    }

    // Build font atlas
    fonts.build_rgba32_texture();

    Ok(())
}

/// RAII guard for ImGui frame
pub struct FrameGuard<'a> {
    ui: Option<Ui>,
    renderer: &'a mut Renderer,
}

impl<'a> FrameGuard<'a> {
    pub fn new(renderer: &'a mut Renderer) -> Result<Self> {
        let ui = renderer.new_frame()?;
        Ok(Self { ui: Some(ui), renderer })
    }

    pub fn ui(&mut self) -> &mut Ui {
        self.ui.as_mut().expect("Frame already ended")
    }

    pub fn end(mut self) -> Result<()> {
        if let Some(ui) = self.ui.take() {
            self.renderer.render(ui)?;
        }
        Ok(())
    }
}

impl<'a> Drop for FrameGuard<'a> {
    fn drop(&mut self) {
        if self.ui.is_some() {
            // Frame was not properly ended, try to render anyway
            if let Some(ui) = self.ui.take() {
                let _ = self.renderer.render(ui);
            }
        }
    }
}