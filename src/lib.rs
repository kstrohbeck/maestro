// TODO: Move the macros to a "macros" module.
#[macro_use]
pub mod macros;

pub mod album;
pub mod disc;
pub mod image;
pub mod raw;
pub mod text;
pub mod track;
pub mod utils;

pub use text::Text;
