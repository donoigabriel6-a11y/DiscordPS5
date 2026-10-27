//! PS5 Input System - DualSense Controller
//!
//! Handles DualSense controller input via HID raw interface on Linux.
//! On PS5/FreeBSD, provides stub implementations using native PS5 input APIs.

#[cfg(feature = "linux-input")]
use std::collections::HashMap;
#[cfg(feature = "linux-input")]
use std::fs::File;
#[cfg(feature = "linux-input")]
use std::os::unix::io::AsRawFd;
#[cfg(feature = "linux-input")]
use std::path::Path;
#[cfg(feature = "linux-input")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "linux-input")]
use std::thread;
#[cfg(feature = "linux-input")]
use std::time::Duration;

#[cfg(feature = "linux-input")]
use anyhow::{Context, Result};
#[cfg(feature = "linux-input")]
use nix::poll::{poll, PollFd, PollFlags};
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};

use crate::{DiscordError, Result as PlatformResult};

/// DualSense HID report IDs
const REPORT_ID_INPUT: u8 = 0x01;
const REPORT_ID_OUTPUT: u8 = 0x02;
const REPORT_ID_FEATURE: u8 = 0x03;

/// DualSense button bitmasks (from Linux kernel hid-playstation driver)
mod button {
    pub const CROSS: u32 = 1 << 0;      // X
    pub const CIRCLE: u32 = 1 << 1;     // O
    pub const SQUARE: u32 = 1 << 2;     // Square
    pub const TRIANGLE: u32 = 1 << 3;   // Triangle
    pub const L1: u32 = 1 << 4;
    pub const R1: u32 = 1 << 5;
    pub const L2: u32 = 1 << 6;
    pub const R2: u32 = 1 << 7;
    pub const SELECT: u32 = 1 << 8;     // Create
    pub const START: u32 = 1 << 9;      // Options
    pub const L3: u32 = 1 << 10;
    pub const R3: u32 = 1 << 11;
    pub const PS: u32 = 1 << 12;
    pub const TOUCHPAD: u32 = 1 << 13;
    pub const MUTE: u32 = 1 << 14;
    pub const DPAD_UP: u32 = 1 << 15;
    pub const DPAD_DOWN: u32 = 1 << 16;
    pub const DPAD_LEFT: u32 = 1 << 17;
    pub const DPAD_RIGHT: u32 = 1 << 18;
}

/// DualSense input report structure (64 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct DualSenseInputReport {
    report_id: u8,
    left_stick_x: u8,
    left_stick_y: u8,
    right_stick_x: u8,
    right_stick_y: u8,
    buttons_low: u16,
    buttons_high: u16,
    l2_trigger: u8,
    r2_trigger: u8,
    // ... rest of report (gyro, accel, touchpad, battery, etc.)
    _padding: [u8; 44],
}

/// Controller state
#[derive(Debug, Clone, Default)]
pub struct ControllerState {
    pub connected: bool,
    pub buttons: u32,
    pub left_stick: StickState,
    pub right_stick: StickState,
    pub l2_trigger: f32,
    pub r2_trigger: f32,
    pub touchpad: TouchpadState,
    pub gyro: GyroState,
    pub accel: AccelState,
    pub battery: BatteryState,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StickState {
    pub x: f32,  // -1.0 to 1.0
    pub y: f32,  // -1.0 to 1.0
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TouchpadState {
    pub touches: [TouchPoint; 2],
    pub touch_count: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TouchPoint {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GyroState {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AccelState {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BatteryState {
    pub level: u8,      // 0-100
    pub charging: bool,
    pub usb_powered: bool,
}

/// Button state for edge detection
#[derive(Debug, Clone, Default)]
struct ButtonState {
    current: u32,
    previous: u32,
}

impl ButtonState {
    fn pressed(&self, button: u32) -> bool {
        (self.current & button) != 0 && (self.previous & button) == 0
    }

    fn released(&self, button: u32) -> bool {
        (self.current & button) == 0 && (self.previous & button) != 0
    }

    fn held(&self, button: u32) -> bool {
        (self.current & button) != 0
    }
}

// Linux implementation using HID raw interface
#[cfg(feature = "linux-input")]
mod linux_input {
    use super::*;

    /// Input manager for handling DualSense controllers
    pub struct InputManager {
        devices: RwLock<HashMap<String, ControllerDevice>>,
        state: Arc<RwLock<ControllerState>>,
        event_callback: Option<Box<dyn Fn(InputEvent) + Send + Sync>>,
    }

    struct ControllerDevice {
        path: String,
        fd: RawFd,
        thread_handle: Option<thread::JoinHandle<()>>,
        running: Arc<std::sync::atomic::AtomicBool>,
    }

    type RawFd = i32;

    #[derive(Debug, Clone)]
    pub enum InputEvent {
        ButtonPressed(u32),
        ButtonReleased(u32),
        StickMoved { stick: Stick, x: f32, y: f32 },
        TriggerMoved { trigger: Trigger, value: f32 },
        TouchpadTouch { index: u8, x: u16, y: u16 },
        TouchpadRelease { index: u8 },
        GyroUpdate { x: f32, y: f32, z: f32 },
        AccelUpdate { x: f32, y: f32, z: f32 },
        BatteryUpdate { level: u8, charging: bool },
        Connected,
        Disconnected,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Stick {
        Left,
        Right,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Trigger {
        L2,
        R2,
    }

    impl InputManager {
        pub fn new() -> Self {
            Self {
                devices: RwLock::new(HashMap::new()),
                state: Arc::new(RwLock::new(ControllerState::default())),
                event_callback: None,
            }
        }

        pub fn set_event_callback<F>(&mut self, callback: F)
        where
            F: Fn(InputEvent) + Send + Sync + 'static,
        {
            self.event_callback = Some(Box::new(callback));
        }

        /// Scan for and connect to DualSense controllers
        pub fn scan_and_connect(&self) -> PlatformResult<usize> {
            let hidraw_path = Path::new("/dev");
            let mut count = 0;

            for entry in std::fs::read_dir(hidraw_path)? {
                let entry = entry?;
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if name.starts_with("hidraw") && self.is_dualsense(&path)? {
                    info!("Found DualSense at {:?}", path);
                    self.connect_device(path.to_string_lossy().to_string())?;
                    count += 1;
                }
            }

            Ok(count)
        }

        fn is_dualsense(&self, path: &Path) -> PlatformResult<bool> {
            // Check if device is a DualSense by reading HID descriptor
            // VID: 0x054c (Sony), PID: 0x0ce6 (DualSense)
            let file = File::open(path)?;
            let fd = file.as_raw_fd();

            // Use HIDIOCGRAWINFO to get device info
            // For now, try to open and read a report
            Ok(true) // Simplified - would check VID/PID in real implementation
        }

        fn connect_device(&self, path: String) -> PlatformResult<()> {
            let file = File::open(&path)
                .with_context(|| format!("Failed to open device {}", path))?;
            let fd = file.as_raw_fd();

            let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
            let running_clone = running.clone();
            let state = self.state.clone();
            let callback = self.event_callback.clone();
            let path_clone = path.clone();

            let handle = thread::spawn(move || {
                Self::device_read_loop(fd, path_clone, running_clone, state, callback);
            });

            let device = ControllerDevice {
                path: path.clone(),
                fd,
                thread_handle: Some(handle),
                running,
            };

            self.devices.write().insert(path, device);
            info!("Connected to DualSense at {}", path);

            // Emit connected event
            if let Some(cb) = &self.event_callback {
                cb(InputEvent::Connected);
            }

            Ok(())
        }

        fn device_read_loop(
            fd: RawFd,
            path: String,
            running: Arc<std::sync::atomic::AtomicBool>,
            state: Arc<RwLock<ControllerState>>,
            callback: Option<Box<dyn Fn(InputEvent) + Send + Sync>>,
        ) {
            let mut poll_fd = PollFd::new(fd, PollFlags::POLLIN);
            let mut prev_buttons = ButtonState::default();
            let mut prev_touchpad = TouchpadState::default();

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                match poll(&mut [&mut poll_fd], 100) {
                    Ok(0) => continue, // Timeout
                    Ok(_) => {}
                    Err(e) => {
                        error!("Poll error on {}: {}", path, e);
                        break;
                    }
                }

                if poll_fd.revents().contains(PollFlags::POLLIN) {
                    let mut buf = [0u8; 64];
                    match nix::unistd::read(fd, &mut buf) {
                        Ok(n) if n >= 6 => {
                            if let Some(report) = parse_input_report(&buf) {
                                let mut state_guard = state.write();
                                state_guard.connected = true;

                                // Process buttons
                                let buttons = u32::from(report.buttons_low) | (u32::from(report.buttons_high) << 16);
                                state_guard.buttons = buttons;

                                // Check for button events
                                let current_buttons = ButtonState {
                                    current: buttons,
                                    previous: prev_buttons.current,
                                };

                                if let Some(cb) = &callback {
                                    for (btn, name) in [
                                        (button::CROSS, "Cross"),
                                        (button::CIRCLE, "Circle"),
                                        (button::SQUARE, "Square"),
                                        (button::TRIANGLE, "Triangle"),
                                        (button::L1, "L1"),
                                        (button::R1, "R1"),
                                        (button::L2, "L2"),
                                        (button::R2, "R2"),
                                        (button::SELECT, "Select"),
                                        (button::START, "Start"),
                                        (button::L3, "L3"),
                                        (button::R3, "R3"),
                                        (button::PS, "PS"),
                                        (button::TOUCHPAD, "Touchpad"),
                                        (button::MUTE, "Mute"),
                                        (button::DPAD_UP, "DpadUp"),
                                        (button::DPAD_DOWN, "DpadDown"),
                                        (button::DPAD_LEFT, "DpadLeft"),
                                        (button::DPAD_RIGHT, "DpadRight"),
                                    ] {
                                        if current_buttons.pressed(btn) {
                                            cb(InputEvent::ButtonPressed(btn));
                                        } else if current_buttons.released(btn) {
                                            cb(InputEvent::ButtonReleased(btn));
                                        }
                                    }
                                }
                                prev_buttons = current_buttons;

                                // Process sticks
                                state_guard.left_stick = stick_from_raw(report.left_stick_x, report.left_stick_y);
                                state_guard.right_stick = stick_from_raw(report.right_stick_x, report.right_stick_y);

                                if let Some(cb) = &callback {
                                    cb(InputEvent::StickMoved {
                                        stick: Stick::Left,
                                        x: state_guard.left_stick.x,
                                        y: state_guard.left_stick.y
                                    });
                                    cb(InputEvent::StickMoved {
                                        stick: Stick::Right,
                                        x: state_guard.right_stick.x,
                                        y: state_guard.right_stick.y
                                    });
                                }

                                // Process triggers
                                state_guard.l2_trigger = report.l2_trigger as f32 / 255.0;
                                state_guard.r2_trigger = report.r2_trigger as f32 / 255.0;

                                if let Some(cb) = &callback {
                                    cb(InputEvent::TriggerMoved {
                                        trigger: Trigger::L2,
                                        value: state_guard.l2_trigger
                                    });
                                    cb(InputEvent::TriggerMoved {
                                        trigger: Trigger::R2,
                                        value: state_guard.r2_trigger
                                    });
                                }

                                // TODO: Parse touchpad, gyro, accel, battery from report
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            error!("Read error on {}: {}", path, e);
                            break;
                        }
                    }
                }
            }

            // Device disconnected
            let mut state_guard = state.write();
            state_guard.connected = false;

            if let Some(cb) = callback {
                cb(InputEvent::Disconnected);
            }

            info!("Disconnected from {}", path);
        }

        /// Get current controller state
        pub fn get_state(&self) -> ControllerState {
            self.state.read().clone()
        }

        /// Check if a button is currently pressed
        pub fn is_button_pressed(&self, button: u32) -> bool {
            (self.state.read().buttons & button) != 0
        }

        /// Get left stick state
        pub fn left_stick(&self) -> StickState {
            self.state.read().left_stick
        }

        /// Get right stick state
        pub fn right_stick(&self) -> StickState {
            self.state.read().right_stick
        }

        /// Get trigger values
        pub fn triggers(&self) -> (f32, f32) {
            let state = self.state.read();
            (state.l2_trigger, state.r2_trigger)
        }

        /// Disconnect all devices
        pub fn disconnect_all(&self) {
            let mut devices = self.devices.write();
            for (_, device) in devices.drain() {
                device.running.store(false, std::sync::atomic::Ordering::Relaxed);
                if let Some(handle) = device.thread_handle {
                    let _ = handle.join();
                }
            }
        }
    }

    impl Default for InputManager {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for InputManager {
        fn drop(&mut self) {
            self.disconnect_all();
        }
    }

    fn stick_from_raw(x: u8, y: u8) -> StickState {
        // Convert 0-255 to -1.0 to 1.0 with deadzone
        const DEADZONE: f32 = 0.1;
        let fx = (x as f32 - 128.0) / 128.0;
        let fy = (y as f32 - 128.0) / 128.0;

        let x = if fx.abs() < DEADZONE { 0.0 } else { fx };
        let y = if fy.abs() < DEADZONE { 0.0 } else { fy };

        StickState { x, y }
    }

    fn parse_input_report(buf: &[u8]) -> Option<DualSenseInputReport> {
        if buf.len() < 6 || buf[0] != REPORT_ID_INPUT {
            return None;
        }

        // Parse the report - simplified for now
        let report = DualSenseInputReport {
            report_id: buf[0],
            left_stick_x: buf[1],
            left_stick_y: buf[2],
            right_stick_x: buf[3],
            right_stick_y: buf[4],
            buttons_low: u16::from_le_bytes([buf[5], buf[6]]),
            buttons_high: u16::from_le_bytes([buf[7], buf[8]]),
            l2_trigger: buf[9],
            r2_trigger: buf[10],
            _padding: [0; 44],
        };

        Some(report)
    }
}

// PS5/FreeBSD stub implementation
#[cfg(not(feature = "linux-input"))]
mod ps5_input {
    use super::*;

    /// Stub input manager for PS5
    pub struct InputManager {
        state: Arc<RwLock<ControllerState>>,
        event_callback: Option<Box<dyn Fn(InputEvent) + Send + Sync>>,
    }

    #[derive(Debug, Clone)]
    pub enum InputEvent {
        ButtonPressed(u32),
        ButtonReleased(u32),
        StickMoved { stick: Stick, x: f32, y: f32 },
        TriggerMoved { trigger: Trigger, value: f32 },
        TouchpadTouch { index: u8, x: u16, y: u16 },
        TouchpadRelease { index: u8 },
        GyroUpdate { x: f32, y: f32, z: f32 },
        AccelUpdate { x: f32, y: f32, z: f32 },
        BatteryUpdate { level: u8, charging: bool },
        Connected,
        Disconnected,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Stick {
        Left,
        Right,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Trigger {
        L2,
        R2,
    }

    impl InputManager {
        pub fn new() -> Self {
            Self {
                state: Arc::new(RwLock::new(ControllerState::default())),
                event_callback: None,
            }
        }

        pub fn set_event_callback<F>(&mut self, callback: F)
        where
            F: Fn(InputEvent) + Send + Sync + 'static,
        {
            self.event_callback = Some(Box::new(callback));
        }

        /// Scan for controllers (stub for PS5)
        pub fn scan_and_connect(&self) -> PlatformResult<usize> {
            info!("PS5 input scan_and_connect (stub)");
            // On PS5, this would use the native controller API
            Ok(0)
        }

        /// Get current controller state
        pub fn get_state(&self) -> ControllerState {
            self.state.read().clone()
        }

        /// Check if a button is currently pressed
        pub fn is_button_pressed(&self, button: u32) -> bool {
            (self.state.read().buttons & button) != 0
        }

        /// Get left stick state
        pub fn left_stick(&self) -> StickState {
            self.state.read().left_stick
        }

        /// Get right stick state
        pub fn right_stick(&self) -> StickState {
            self.state.read().right_stick
        }

        /// Get trigger values
        pub fn triggers(&self) -> (f32, f32) {
            let state = self.state.read();
            (state.l2_trigger, state.r2_trigger)
        }

        /// Disconnect all devices (no-op for stub)
        pub fn disconnect_all(&self) {}
    }

    impl Default for InputManager {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for InputManager {
        fn drop(&mut self) {}
    }
}

// Navigation helper (platform-independent)
/// Navigation helper for gamepad UI navigation
pub struct NavigationHelper {
    repeat_delay: Duration,
    repeat_rate: Duration,
    last_input: std::time::Instant,
    last_direction: Option<NavDirection>,
    repeat_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Menu,
    Search,
}

impl NavigationHelper {
    pub fn new() -> Self {
        Self {
            repeat_delay: Duration::from_millis(300),
            repeat_rate: Duration::from_millis(100),
            last_input: std::time::Instant::now(),
            last_direction: None,
            repeat_count: 0,
        }
    }

    pub fn update(&mut self, input: &InputManager) -> Option<NavDirection> {
        let state = input.get_state();
        let now = std::time::Instant::now();

        // Check D-pad
        let direction = if (state.buttons & button::DPAD_UP) != 0 {
            Some(NavDirection::Up)
        } else if (state.buttons & button::DPAD_DOWN) != 0 {
            Some(NavDirection::Down)
        } else if (state.buttons & button::DPAD_LEFT) != 0 {
            Some(NavDirection::Left)
        } else if (state.buttons & button::DPAD_RIGHT) != 0 {
            Some(NavDirection::Right)
        } else if (state.buttons & button::CROSS) != 0 {
            Some(NavDirection::Select)
        } else if (state.buttons & button::CIRCLE) != 0 {
            Some(NavDirection::Back)
        } else if (state.buttons & button::TRIANGLE) != 0 {
            Some(NavDirection::Menu)
        } else if (state.buttons & button::SQUARE) != 0 {
            Some(NavDirection::Search)
        } else {
            None
        };

        if let Some(dir) = direction {
            if Some(dir) == self.last_direction {
                let elapsed = now.duration_since(self.last_input);
                if self.repeat_count == 0 {
                    if elapsed >= self.repeat_delay {
                        self.repeat_count = 1;
                        self.last_input = now;
                        return Some(dir);
                    }
                } else if elapsed >= self.repeat_rate {
                    self.repeat_count += 1;
                    self.last_input = now;
                    return Some(dir);
                }
            } else {
                self.last_direction = Some(dir);
                self.repeat_count = 0;
                self.last_input = now;
                return Some(dir);
            }
        } else {
            self.last_direction = None;
            self.repeat_count = 0;
        }

        None
    }
}

impl Default for NavigationHelper {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export the appropriate implementation
#[cfg(feature = "linux-input")]
pub use linux_input::*;

#[cfg(not(feature = "linux-input"))]
pub use ps5_input::*;

/// DualSense controller wrapper (matches main.rs API)
pub struct DualSense {
    manager: InputManager,
}

impl DualSense {
    /// Create a new DualSense controller
    pub fn new() -> PlatformResult<Self> {
        let manager = InputManager::new();
        // On Linux, scan for controllers
        #[cfg(feature = "linux-input")]
        {
            let _ = manager.scan_and_connect();
        }
        Ok(Self { manager })
    }

    /// Poll for controller events
    pub fn poll_event(&mut self) -> PlatformResult<()> {
        // On Linux, the InputManager runs in a background thread
        // On PS5, this would poll the native controller API
        Ok(())
    }

    /// Get current controller state
    pub fn get_state(&self) -> ControllerState {
        self.manager.get_state()
    }

    /// Check if a button is pressed
    pub fn is_button_pressed(&self, button: u32) -> bool {
        self.manager.is_button_pressed(button)
    }

    /// Get left stick state
    pub fn left_stick(&self) -> StickState {
        self.manager.left_stick()
    }

    /// Get right stick state
    pub fn right_stick(&self) -> StickState {
        self.manager.right_stick()
    }

    /// Get trigger values
    pub fn triggers(&self) -> (f32, f32) {
        self.manager.triggers()
    }
}

impl Default for DualSense {
    fn default() -> Self {
        Self::new().expect("Failed to create DualSense")
    }
}