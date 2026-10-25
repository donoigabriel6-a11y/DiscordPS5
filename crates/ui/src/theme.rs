//! Discord Dark Theme
//!
//! ImGui theme matching Discord's dark mode colors.

use imgui::{StyleVar, StyleColor, ColorEditFlags};

/// Discord dark theme colors
pub const DISCORD_BG: [f32; 4] = [0.137, 0.137, 0.145, 1.0];        // #23272A
pub const DISCORD_CHAT_BG: [f32; 4] = [0.110, 0.110, 0.118, 1.0];   // #1C1D1E
pub const DISCORD_SIDEBAR_BG: [f32; 4] = [0.114, 0.118, 0.125, 1.0]; // #1D1E20
pub const DISCORD_TEXT: [f32; 4] = [0.863, 0.871, 0.878, 1.0];      // #DCDDDE
pub const DISCORD_TEXT_MUTED: [f32; 4] = [0.447, 0.471, 0.506, 1.0]; // #72767D
pub const DISCORD_TEXT_LINK: [f32; 4] = [0.008, 0.604, 0.918, 1.0];  // #009BF5
pub const DISCORD_ACCENT: [f32; 4] = [0.349, 0.220, 0.933, 1.0];    // #5936F5
pub const DISCORD_ACCENT_HOVER: [f32; 4] = [0.427, 0.298, 0.953, 1.0]; // #6D4CF3
pub const DISCORD_ACCENT_ACTIVE: [f32; 4] = [0.271, 0.141, 0.914, 1.0]; // #4524E9
pub const DISCORD_RED: [f32; 4] = [0.937, 0.298, 0.341, 1.0];       // #ED4245
pub const DISCORD_GREEN: [f32; 4] = [0.180, 0.729, 0.333, 1.0];     // #2EB855
pub const DISCORD_YELLOW: [f32; 4] = [0.988, 0.757, 0.161, 1.0];    // #FCC129
pub const DISCORD_BORDER: [f32; 4] = [0.180, 0.184, 0.192, 1.0];    // #2E2F31
pub const DISCORD_HOVER: [f32; 4] = [0.176, 0.180, 0.188, 1.0];     // #2D2E30
pub const DISCORD_ACTIVE: [f32; 4] = [0.220, 0.224, 0.231, 1.0];    // #38393B
pub const DISCORD_POPOUT_BG: [f32; 4] = [0.114, 0.118, 0.125, 1.0]; // #1D1E20
pub const DISCORD_SCROLLBAR: [f32; 4] = [0.114, 0.118, 0.125, 1.0]; // #1D1E20
pub const DISCORD_SCROLLBAR_HOVER: [f32; 4] = [0.220, 0.224, 0.231, 1.0]; // #38393B

/// Apply Discord dark theme to ImGui style
pub fn apply_discord_theme(style: &mut imgui::Style) {
    // Window
    style.colors[StyleColor::WindowBg as usize] = DISCORD_CHAT_BG.into();
    style.colors[StyleColor::ChildBg as usize] = DISCORD_CHAT_BG.into();
    style.colors[StyleColor::PopupBg as usize] = DISCORD_POPOUT_BG.into();

    // Borders
    style.colors[StyleColor::Border as usize] = DISCORD_BORDER.into();
    style.colors[StyleColor::BorderShadow as usize] = [0.0, 0.0, 0.0, 0.0].into();

    // Text
    style.colors[StyleColor::Text as usize] = DISCORD_TEXT.into();
    style.colors[StyleColor::TextDisabled as usize] = DISCORD_TEXT_MUTED.into();
    style.colors[StyleColor::TextSelectedBg as usize] = [DISCORD_ACCENT[0], DISCORD_ACCENT[1], DISCORD_ACCENT[2], 0.35].into();

    // Headers
    style.colors[StyleColor::Header as usize] = DISCORD_HOVER.into();
    style.colors[StyleColor::HeaderHovered as usize] = DISCORD_HOVER.into();
    style.colors[StyleColor::HeaderActive as usize] = DISCORD_ACTIVE.into();

    // Buttons
    style.colors[StyleColor::Button as usize] = DISCORD_ACCENT.into();
    style.colors[StyleColor::ButtonHovered as usize] = DISCORD_ACCENT_HOVER.into();
    style.colors[StyleColor::ButtonActive as usize] = DISCORD_ACCENT_ACTIVE.into();

    // Frame (input fields, etc.)
    style.colors[StyleColor::FrameBg as usize] = DISCORD_SIDEBAR_BG.into();
    style.colors[StyleColor::FrameBgHovered as usize] = DISCORD_HOVER.into();
    style.colors[StyleColor::FrameBgActive as usize] = DISCORD_ACTIVE.into();

    // Tabs
    style.colors[StyleColor::Tab as usize] = DISCORD_SIDEBAR_BG.into();
    style.colors[StyleColor::TabHovered as usize] = DISCORD_HOVER.into();
    style.colors[StyleColor::TabActive as usize] = DISCORD_ACTIVE.into();
    style.colors[StyleColor::TabUnfocused as usize] = DISCORD_SIDEBAR_BG.into();
    style.colors[StyleColor::TabUnfocusedActive as usize] = DISCORD_HOVER.into();

    // Title bar
    style.colors[StyleColor::TitleBg as usize] = DISCORD_SIDEBAR_BG.into();
    style.colors[StyleColor::TitleBgActive as usize] = DISCORD_SIDEBAR_BG.into();
    style.colors[StyleColor::TitleBgCollapsed as usize] = DISCORD_SIDEBAR_BG.into();

    // Menu bar
    style.colors[StyleColor::MenuBarBg as usize] = DISCORD_SIDEBAR_BG.into();

    // Scrollbar
    style.colors[StyleColor::ScrollbarBg as usize] = DISCORD_SCROLLBAR.into();
    style.colors[StyleColor::ScrollbarGrab as usize] = DISCORD_BORDER.into();
    style.colors[StyleColor::ScrollbarGrabHovered as usize] = DISCORD_SCROLLBAR_HOVER.into();
    style.colors[StyleColor::ScrollbarGrabActive as usize] = DISCORD_SCROLLBAR_HOVER.into();

    // Separator
    style.colors[StyleColor::Separator as usize] = DISCORD_BORDER.into();
    style.colors[StyleColor::SeparatorHovered as usize] = DISCORD_BORDER.into();
    style.colors[StyleColor::SeparatorActive as usize] = DISCORD_BORDER.into();

    // Resize grip
    style.colors[StyleColor::ResizeGrip as usize] = DISCORD_ACCENT.into();
    style.colors[StyleColor::ResizeGripHovered as usize] = DISCORD_ACCENT_HOVER.into();
    style.colors[StyleColor::ResizeGripActive as usize] = DISCORD_ACCENT_ACTIVE.into();

    // Plot/Graph
    style.colors[StyleColor::PlotLines as usize] = DISCORD_ACCENT.into();
    style.colors[StyleColor::PlotLinesHovered as usize] = DISCORD_ACCENT_HOVER.into();
    style.colors[StyleColor::PlotHistogram as usize] = DISCORD_ACCENT.into();
    style.colors[StyleColor::PlotHistogramHovered as usize] = DISCORD_ACCENT_HOVER.into();

    // Table
    style.colors[StyleColor::TableHeaderBg as usize] = DISCORD_SIDEBAR_BG.into();
    style.colors[StyleColor::TableBorderStrong as usize] = DISCORD_BORDER.into();
    style.colors[StyleColor::TableBorderLight as usize] = DISCORD_BORDER.into();
    style.colors[StyleColor::TableRowBg as usize] = DISCORD_CHAT_BG.into();
    style.colors[StyleColor::TableRowBgAlt as usize] = DISCORD_HOVER.into();

    // Slider
    style.colors[StyleColor::SliderGrab as usize] = DISCORD_ACCENT.into();
    style.colors[StyleColor::SliderGrabActive as usize] = DISCORD_ACCENT_ACTIVE.into();

    // Check mark
    style.colors[StyleColor::CheckMark as usize] = DISCORD_ACCENT.into();

    // Drag drop target
    style.colors[StyleColor::DragDropTarget as usize] = DISCORD_ACCENT.into();

    // Nav highlight
    style.colors[StyleColor::NavHighlight as usize] = DISCORD_ACCENT.into();
    style.colors[StyleColor::NavWindowingHighlight as usize] = DISCORD_ACCENT.into();
    style.colors[StyleColor::NavWindowingDimBg as usize] = [0.0, 0.0, 0.0, 0.2].into();

    // Modal window dim
    style.colors[StyleColor::ModalWindowDimBg as usize] = [0.0, 0.0, 0.0, 0.5].into();

    // Style variables
    style.window_rounding = 4.0;
    style.child_rounding = 4.0;
    style.frame_rounding = 3.0;
    style.popup_rounding = 4.0;
    style.scrollbar_rounding = 9.0;
    style.grab_rounding = 3.0;
    style.tab_rounding = 4.0;

    style.window_padding = [8.0, 8.0].into();
    style.frame_padding = [6.0, 4.0].into();
    style.item_spacing = [8.0, 4.0].into();
    style.item_inner_spacing = [4.0, 4.0].into();
    style.cell_padding = [4.0, 2.0].into();
    style.touch_extra_padding = [0.0, 0.0].into();

    style.indent_spacing = 16.0;
    style.scrollbar_size = 8.0;
    style.grab_min_size = 8.0;

    style.window_border_size = 0.0;
    style.child_border_size = 1.0;
    style.popup_border_size = 1.0;
    style.frame_border_size = 0.0;
    style.tab_border_size = 0.0;

    style.alpha = 1.0;
    style.disabled_alpha = 0.5;
}

/// Apply compact mode theme (tighter spacing)
pub fn apply_compact_theme(style: &mut imgui::Style) {
    apply_discord_theme(style);
    style.item_spacing = [4.0, 2.0].into();
    style.frame_padding = [4.0, 2.0].into();
    style.window_padding = [4.0, 4.0].into();
    style.cell_padding = [4.0, 1.0].into();
}

/// Apply comfortable mode theme (looser spacing)
pub fn apply_comfortable_theme(style: &mut imgui::Style) {
    apply_discord_theme(style);
    style.item_spacing = [12.0, 8.0].into();
    style.frame_padding = [8.0, 6.0].into();
    style.window_padding = [12.0, 12.0].into();
    style.cell_padding = [6.0, 4.0].into();
}