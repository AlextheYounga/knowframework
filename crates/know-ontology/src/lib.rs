#![expect(clippy::module_name_repetitions)]

pub mod compile;
pub mod ir;
pub mod source;

pub use ir::*;
pub use source::*;
