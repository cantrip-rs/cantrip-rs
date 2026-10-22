//! The game state: plain data in a few contiguous buffers, every write journalled
//! (docs/engine-design.md, "State: one arena, every write journalled"). No behavior lives here.

pub mod decision;
pub mod name;
pub mod object;
pub mod state;
pub mod subtypes;
pub mod vocab;

pub use decision::*;
pub use name::*;
pub use object::*;
pub use state::*;
pub use vocab::*;
