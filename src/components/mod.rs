// Component-based architecture for dotstate TUI

pub mod component;

pub mod file_preview;
pub mod footer;
pub mod header;
pub mod help_overlay;
pub mod message;
pub mod message_box;

pub use component::{Component, ComponentAction};
// Footer and InputField are used directly via their module paths
// pub use footer::Footer;
// pub use input_field::InputField;

pub use message::MessageComponent;
