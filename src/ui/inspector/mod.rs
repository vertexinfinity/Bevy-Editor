//! Streamlined Entity Inspector subsystem with modular docking, transform, scripts, and picking.

pub mod types;
pub mod picking;
pub mod components_card;
pub mod script_card;
pub mod transform_card;
pub mod dock;

// Re-export all public symbols for backwards compatibility
pub use types::*;
pub use picking::*;
pub use components_card::*;
pub use script_card::*;
pub use transform_card::*;
pub use dock::*;
