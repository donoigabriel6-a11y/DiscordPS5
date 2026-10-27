//! PS5 Graphics System
//!
//! Implements graphics initialization. On Linux, uses DRM/KMS/GBM/EGL pipeline
//! with Mesa-based OpenGL rendering. On PS5/FreeBSD, provides stub implementations
//! that will be replaced with native LibGNM APIs.

#[cfg(target_os = "linux")]
use std::ffi::CString;
#[cfg(target_os = "linux")]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(target_os = "linux")]
use std::ptr;
#[cfg(target_os = "linux")]
use std::sync::Arc;

#[cfg(target_os = "linux")]
use anyhow::{Context, Result};
#[cfg(target_os = "linux")]
use drm::control::Device as DrmDevice;
#[cfg(target_os = "linux")]
use drm::control::{connector, crtc, framebuffer, plane, ResourceHandles};
#[cfg(target_os = "linux")]
use drm::ClientCapability;
#[cfg(target_os = "linux")]
use gbm::{BufferObjectFlags, Device as GbmDevice, Format, Surface as GbmSurface};
#[cfg(target_os = "linux")]
use egl::{self, Context as EglContext, Display as EglDisplay, Surface as EglSurface};
#[cfg(target_os = "linux")]
use gl::{self, Gl};
use parking_lot::Mutex;
use tracing::{debug, error, info, warn};

use crate::{DiscordError, Result as PlatformResult};

/// Graphics configuration
#[derive(Debug, Clone, Default)]
pub struct GraphicsConfig {
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
}

impl GraphicsConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Screen resolution (1080p default)
pub const DEFAULT_WIDTH: u32 = 1920;
pub const DEFAULT_HEIGHT: u32 = 1080;

/// Graphics context holding all graphics resources
#[cfg(target_os = "linux")]
pub struct GraphicsContext {
    drm_fd: RawFd,
    drm_device: drm::Device<drm::control::File>,
    gbm_device: GbmDevice<drm::control::File>,
    gbm_surface: GbmSurface<drm::control::File>,
    egl_display: EglDisplay,
    egl_context: EglContext,
    egl_surface: EglSurface,
    gl: Gl,
    width: u32,
    height: u32,
    current_fb: Option<framebuffer::Handle>,
    dumb_buffer: Option<DumbBuffer>,
}

#[cfg(not(target_os = "linux"))]
pub struct GraphicsContext {
    width: u32,
    height: u32,
    initialized: bool,
}

/// Simple dumb buffer for scanout (Linux only)
#[cfg(target_os = "linux")]
struct DumbBuffer {
    handle: u32,
    stride: u32,
    size: u64,
    map: *mut std::ffi::c_void,
    fb_id: u32,
}

impl GraphicsContext {
    /// Initialize the full graphics pipeline
    pub fn new(width: u32, height: u32) -> PlatformResult<Self> {
        #[cfg(target_os = "linux")]
        {
            info!("Initializing Linux graphics pipeline (DRM/KMS/GBM/EGL): {}x{}", width, height);
            Self::new_linux(width, height)
        }
        #[cfg(not(target_os = "linux"))]
        {
            info!("Initializing PS5 graphics stub: {}x{}", width, height);
            // On PS5, this will be replaced with LibGNM initialization
            Ok(Self {
                width,
                height,
                initialized: true,
            })
        }
    }

    #[cfg(target_os = "linux")]
    fn new_linux(width: u32, height: u32) -> PlatformResult<Self> {
        // 1. Open DRM device
        let drm_fd = open_drm_device()?;
        let drm_device = drm::control::File::open(&drm_fd)?;

        // Set DRM client capabilities
        setup_drm_capabilities(&drm_device)?;

        // 2. Find connector and mode
        let (connector_handle, mode) = find_connector_and_mode(&drm_device, width, height)?;

        // 3. Create GBM device
        let gbm_device = GbmDevice::new(drm_fd)?;

        // 4. Create GBM surface
        let gbm_surface = create_gbm_surface(&gbm_device, width, height)?;

        // 5. Initialize EGL
        let (egl_display, egl_context, egl_surface) = init_egl(&gbm_device, &gbm_surface)?;

        // 6. Load OpenGL functions
        let gl = load_gl_functions(&egl_display)?;

        // 7. Create dumb buffer for initial framebuffer
        let dumb_buffer = create_dumb_buffer(&drm_device, width, height)?;

        // 8. Set up initial mode
        setup_mode(&drm_device, connector_handle, &mode, dumb_buffer.fb_id)?;

        info!("Graphics pipeline initialized successfully");

        Ok(Self {
            drm_fd,
            drm_device,
            gbm_device,
            gbm_surface,
            egl_display,
            egl_context,
            egl_surface,
            gl,
            width,
            height,
            current_fb: Some(framebuffer::Handle(dumb_buffer.fb_id)),
            dumb_buffer: Some(dumb_buffer),
        })
    }

    /// Get the OpenGL function table (Linux only)
    #[cfg(target_os = "linux")]
    pub fn gl(&self) -> &Gl {
        &self.gl
    }

    /// Get current framebuffer dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Swap buffers (present)
    pub fn swap_buffers(&mut self) -> PlatformResult<()> {
        #[cfg(target_os = "linux")]
        {
            // Swap EGL buffers
            unsafe {
                egl::swap_buffers(self.egl_display, self.egl_surface)
                    .context("Failed to swap EGL buffers")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Stub for PS5 - will use LibGNM buffer flip
            Ok(())
        }
    }

    /// Make the context current
    pub fn make_current(&self) -> PlatformResult<()> {
        #[cfg(target_os = "linux")]
        {
            unsafe {
                egl::make_current(
                    self.egl_display,
                    Some(self.egl_surface),
                    Some(self.egl_surface),
                    Some(self.egl_context),
                )
                .context("Failed to make EGL context current")?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Stub for PS5
            Ok(())
        }
    }

    /// Resize the graphics context
    pub fn resize(&mut self, width: u32, height: u32) -> PlatformResult<()> {
        if width == self.width && height == self.height {
            return Ok(());
        }

        info!("Resizing graphics context to {}x{}", width, height);

        #[cfg(target_os = "linux")]
        {
            // Destroy old resources
            self.cleanup_gbm_surface()?;

            // Create new GBM surface
            self.gbm_surface = create_gbm_surface(&self.gbm_device, width, height)?;

            // Recreate EGL surface
            self.egl_surface = create_egl_surface(self.egl_display, &self.gbm_surface)?;

            // Recreate dumb buffer
            if let Some(old) = self.dumb_buffer.take() {
                destroy_dumb_buffer(&self.drm_device, &old)?;
            }
            self.dumb_buffer = Some(create_dumb_buffer(&self.drm_device, width, height)?);

            // Update mode
            let (connector_handle, mode) = find_connector_and_mode(&self.drm_device, width, height)?;
            setup_mode(&self.drm_device, connector_handle, &mode, self.dumb_buffer.as_ref().unwrap().fb_id)?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Stub for PS5
        }

        self.width = width;
        self.height = height;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn cleanup_gbm_surface(&mut self) -> PlatformResult<()> {
        Ok(())
    }

    /// Get raw DRM file descriptor for external use (Linux only)
    #[cfg(target_os = "linux")]
    pub fn drm_fd(&self) -> RawFd {
        self.drm_fd
    }
}

impl Drop for GraphicsContext {
    fn drop(&mut self) {
        info!("Shutting down graphics context");

        #[cfg(target_os = "linux")]
        {
            // Cleanup in reverse order
            if let Some(buf) = self.dumb_buffer.take() {
                let _ = destroy_dumb_buffer(&self.drm_device, &buf);
            }

            // EGL cleanup
            unsafe {
                let _ = egl::make_current(
                    self.egl_display,
                    None,
                    None,
                    None,
                );
                let _ = egl::destroy_surface(self.egl_display, self.egl_surface);
                let _ = egl::destroy_context(self.egl_display, self.egl_context);
                let _ = egl::terminate(self.egl_display);
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Stub for PS5
        }
    }
}

// Linux-specific implementations
#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    /// DRM device path
    const DRM_DEVICE_PATH: &str = "/dev/dri/card0";

    /// Open the DRM device
    pub fn open_drm_device() -> PlatformResult<RawFd> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(DRM_DEVICE_PATH)
            .context("Failed to open DRM device")?;

        Ok(file.as_raw_fd())
    }

    /// Set up DRM client capabilities
    pub fn setup_drm_capabilities(device: &drm::control::File) -> PlatformResult<()> {
        device
            .set_client_cap(ClientCapability::UniversalPlanes, 1)
            .context("Failed to set universal planes capability")?;
        device
            .set_client_cap(ClientCapability::Atomic, 1)
            .context("Failed to set atomic capability")?;
        device
            .set_client_cap(ClientCapability::AspectRatio, 1)
            .context("Failed to set aspect ratio capability")?;

        Ok(())
    }

    /// Find a connected connector and its preferred mode
    pub fn find_connector_and_mode(
        device: &drm::control::File,
        width: u32,
        height: u32,
    ) -> PlatformResult<(connector::Handle, drm::control::Mode)> {
        let resources = device.resource_handles().context("Failed to get DRM resources")?;

        for conn_handle in resources.connectors() {
            let connector_info = device
                .get_connector(*conn_handle, true)
                .context("Failed to get connector info")?;

            if connector_info.state() == connector::State::Connected {
                info!("Found connected connector: {:?}", conn_handle);

                // Find matching mode
                for mode in connector_info.modes() {
                    if mode.size().0 == width && mode.size().1 == height {
                        info!("Found matching mode: {}x{}@{}Hz", mode.size().0, mode.size().1, mode.vrefresh());
                        return Ok((*conn_handle, mode.clone()));
                    }
                }

                // Fallback to first mode
                if let Some(mode) = connector_info.modes().first() {
                    warn!("No exact mode match, using: {}x{}@{}Hz", mode.size().0, mode.size().1, mode.vrefresh());
                    return Ok((*conn_handle, mode.clone()));
                }
            }
        }

        Err(DiscordError::Graphics("No connected connector found".to_string()).into())
    }

    /// Create GBM surface
    pub fn create_gbm_surface(
        gbm_device: &GbmDevice<drm::control::File>,
        width: u32,
        height: u32,
    ) -> PlatformResult<GbmSurface<drm::control::File>> {
        let surface = gbm_device
            .create_surface(
                width,
                height,
                Format::Xrgb8888,
                BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING,
            )
            .context("Failed to create GBM surface")?;

        Ok(surface)
    }

    /// Initialize EGL
    pub fn init_egl(
        gbm_device: &GbmDevice<drm::control::File>,
        gbm_surface: &GbmSurface<drm::control::File>,
    ) -> PlatformResult<(EglDisplay, EglContext, EglSurface)> {
        // Get EGL display from GBM device
        let display = unsafe {
            egl::get_display(gbm_device.as_raw_fd() as *mut _)
                .context("Failed to get EGL display")?
        };

        // Initialize EGL
        unsafe {
            egl::initialize(display)
                .context("Failed to initialize EGL")?;
        }

        // Bind OpenGL API
        unsafe {
            egl::bind_api(egl::OPENGL_API)
                .context("Failed to bind OpenGL API")?;
        }

        // Choose config
        let config_attribs = [
            egl::SURFACE_TYPE as i32, egl::WINDOW_BIT as i32,
            egl::RED_SIZE as i32, 8,
            egl::GREEN_SIZE as i32, 8,
            egl::BLUE_SIZE as i32, 8,
            egl::ALPHA_SIZE as i32, 8,
            egl::DEPTH_SIZE as i32, 24,
            egl::STENCIL_SIZE as i32, 8,
            egl::RENDERABLE_TYPE as i32, egl::OPENGL_BIT as i32,
            egl::NONE as i32,
        ];

        let config = unsafe {
            let mut num_configs = 0;
            let mut config = ptr::null_mut();
            egl::choose_config(display, &config_attribs, Some(&mut config), 1, &mut num_configs)
                .context("Failed to choose EGL config")?;
            if num_configs == 0 {
                return Err(DiscordError::Graphics("No suitable EGL config found".to_string()).into());
            }
            config
        };

        // Create context
        let context_attribs = [
            egl::CONTEXT_MAJOR_VERSION as i32, 3,
            egl::CONTEXT_MINOR_VERSION as i32, 3,
            egl::CONTEXT_OPENGL_PROFILE_MASK as i32, egl::CONTEXT_OPENGL_CORE_PROFILE_BIT as i32,
            egl::NONE as i32,
        ];

        let context = unsafe {
            egl::create_context(display, config, egl::NO_CONTEXT, Some(&context_attribs))
                .context("Failed to create EGL context")?
        };

        // Create surface
        let surface = create_egl_surface(display, gbm_surface)?;

        // Make current
        unsafe {
            egl::make_current(display, Some(surface), Some(surface), Some(context))
                .context("Failed to make EGL context current")?;
        }

        Ok((display, context, surface))
    }

    /// Create EGL surface from GBM surface
    pub fn create_egl_surface(
        display: EglDisplay,
        gbm_surface: &GbmSurface<drm::control::File>,
    ) -> PlatformResult<EglSurface> {
        let surface = unsafe {
            egl::create_platform_window_surface(
                display,
                ptr::null(),
                gbm_surface.as_raw_fd() as *mut _,
                ptr::null(),
            )
            .context("Failed to create EGL platform window surface")?
        };
        Ok(surface)
    }

    /// Load OpenGL functions
    pub fn load_gl_functions(display: EglDisplay) -> PlatformResult<Gl> {
        let gl = Gl::load_with(|symbol| {
            let cstr = CString::new(symbol).ok();
            cstr.map(|s| unsafe { egl::get_proc_address(s.as_ptr()) as *const _ })
                .unwrap_or(ptr::null())
        });

        info!("OpenGL version: {}", unsafe {
            let version = gl.get_string(gl::VERSION);
            std::ffi::CStr::from_ptr(version as *const _).to_string_lossy()
        });

        Ok(gl)
    }

    /// Create dumb buffer for scanout
    pub fn create_dumb_buffer(
        device: &drm::control::File,
        width: u32,
        height: u32,
    ) -> PlatformResult<DumbBuffer> {
        use drm::control::dumbbuffer::{CreateDumbBuffer, MapDumbBuffer};
        use drm::control::framebuffer::AddFramebuffer;

        let create = CreateDumbBuffer::new(width, height, 32);
        let dumb = device.create_dumb_buffer(create).context("Failed to create dumb buffer")?;

        let add_fb = AddFramebuffer::new(
            width,
            height,
            24,
            32,
            dumb.pitch(),
            dumb.handle(),
        );
        let fb_id = device.add_framebuffer(add_fb).context("Failed to add framebuffer")?;

        let map = MapDumbBuffer::new(dumb.handle());
        let mapped = device.map_dumb_buffer(map).context("Failed to map dumb buffer")?;

        Ok(DumbBuffer {
            handle: dumb.handle(),
            stride: dumb.pitch(),
            size: dumb.size(),
            map: mapped.as_mut_ptr(),
            fb_id,
        })
    }

    /// Destroy dumb buffer
    pub fn destroy_dumb_buffer(
        device: &drm::control::File,
        buffer: &DumbBuffer,
    ) -> PlatformResult<()> {
        use drm::control::dumbbuffer::DestroyDumbBuffer;
        use drm::control::framebuffer::RemoveFramebuffer;

        device.remove_framebuffer(RemoveFramebuffer::new(buffer.fb_id))
            .context("Failed to remove framebuffer")?;

        device.destroy_dumb_buffer(DestroyDumbBuffer::new(buffer.handle))
            .context("Failed to destroy dumb buffer")?;

        Ok(())
    }

    /// Set up display mode
    pub fn setup_mode(
        device: &drm::control::File,
        connector: connector::Handle,
        mode: &drm::control::Mode,
        fb_id: u32,
    ) -> PlatformResult<()> {
        use drm::control::{crtc, ModeSet};

        let resources = device.resource_handles().context("Failed to get resources")?;
        let crtc_handle = resources.crtcs().first()
            .ok_or_else(|| DiscordError::Graphics("No CRTC available".to_string()))?;

        let mode_set = ModeSet::new(
            *crtc_handle,
            Some(framebuffer::Handle(fb_id)),
            (0, 0),
            Some(connector),
            Some(mode.clone()),
        );

        device.mode_set(mode_set).context("Failed to set mode")?;

        Ok(())
    }
}

// Re-export linux_impl functions for use in GraphicsContext
#[cfg(target_os = "linux")]
use linux_impl::*;

/// RAII wrapper for EGL display (Linux only)
#[cfg(target_os = "linux")]
pub struct EglGuard {
    display: EglDisplay,
}

#[cfg(target_os = "linux")]
impl EglGuard {
    pub fn new(display: EglDisplay) -> Self {
        Self { display }
    }
}

#[cfg(target_os = "linux")]
impl Drop for EglGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = egl::terminate(self.display);
        }
    }
}

// PS5 stub - will be replaced with LibGNM implementation
#[cfg(not(target_os = "linux"))]
pub struct EglGuard;

#[cfg(not(target_os = "linux"))]
impl EglGuard {
    pub fn new() -> Self {
        Self
    }
}

// Dummy types for UI crate compatibility on non-Linux targets
#[cfg(not(target_os = "linux"))]
pub struct GbmSurface {
    width: u32,
    height: u32,
}

#[cfg(not(target_os = "linux"))]
impl GbmSurface {
    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

#[cfg(not(target_os = "linux"))]
pub struct EglContext;

#[cfg(not(target_os = "linux"))]
impl EglContext {
    pub fn get_proc_address(&self, _name: &str) -> *const std::ffi::c_void {
        std::ptr::null()
    }
    pub fn swap_buffers(&self) -> crate::Result<()> {
        Ok(())
    }
}

/// Initialize graphics system
/// On Linux, initializes DRM/KMS/GBM/EGL. On PS5, returns stub implementations.
pub fn init_graphics(config: &GraphicsConfig) -> PlatformResult<(GbmSurface, EglContext)> {
    #[cfg(target_os = "linux")]
    {
        // For Linux, create a real GraphicsContext and extract surface/context
        let ctx = GraphicsContext::new(config.width, config.height)?;
        // Extract surface and context - this is simplified
        // In a real implementation, you'd return the actual GBM surface and EGL context
        Err(DiscordError::Graphics("Not fully implemented for Linux yet".to_string()).into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        // PS5 stub - return dummy surface and context
        info!("Initializing PS5 graphics stub: {}x{}", config.width, config.height);
        let surface = GbmSurface {
            width: config.width,
            height: config.height,
        };
        let egl_context = EglContext;
        Ok((surface, egl_context))
    }
}