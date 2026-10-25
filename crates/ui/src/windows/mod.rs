//! UI Windows
//!
//! Individual window implementations for the Discord UI.

pub mod servers;
pub mod channels;
pub mod messages;
pub mod voice;
pub mod settings;
pub mod user_menu;

pub use servers::ServersWindow;
pub use channels::ChannelsWindow;
pub use messages::MessagesWindow;
pub use voice::VoiceWindow;
pub use settings::SettingsWindow;
pub use user_menu::UserMenuWindow;