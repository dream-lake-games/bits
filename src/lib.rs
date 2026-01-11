// Allow the crate to reference itself as `bits::` for macro compatibility
extern crate self as bits;

pub mod ai;
pub mod bg;
pub mod bits_ui;
pub mod consts;
pub mod exa;
pub mod protocol;
pub mod window;

pub mod prelude {
    pub use super::ai::*;
    pub use super::bits_ui::*;
    pub use super::consts::*;
    pub use super::exa::*;
    pub use super::protocol::*;
    pub use macros::*;
}
