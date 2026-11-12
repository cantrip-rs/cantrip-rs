//! The game state: plain data in a few contiguous buffers, every write journalled
//! (docs/engine-design.md, "State: one arena, every write journalled"). No behavior lives here.

pub mod attacks;
pub mod blocks;
pub mod combat_groups;
pub mod cost_receipt;
pub mod decision;
pub mod object;
pub mod state;
pub mod subtypes;
pub mod vocab;

pub use attacks::*;
pub use blocks::*;
pub use combat_groups::*;
pub use cost_receipt::*;
pub use decision::*;
pub use object::*;
pub use state::*;
pub use vocab::*;

#[cfg(test)]
mod combat_group_tests;
