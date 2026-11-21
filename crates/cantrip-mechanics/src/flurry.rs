//! Flurry (an ability word, CR 207.2c): "Whenever you cast your second spell each turn, [effect]." Cori-Steel Cutter and Cosmogrand Zenith print it.
//!
//! A card with flurry lists `trigger(text, effect)` among its `abilities`, or `modal_trigger(text, modes)` when the
//! effect is a choice of modes (CR 700.2b).

use cantrip_api::*;

use crate::triggers::YOU_CAST_YOUR_SECOND_SPELL;

pub const fn trigger(text: &'static str, effect: fn(&mut dyn Game, &StackItem) -> Result<(), Suspend>) -> AbilityDef {
    triggered(YOU_CAST_YOUR_SECOND_SPELL, effect, text)
}

/// The same trigger with modes, chosen as it is put on the stack (CR 603.3c, 700.2b).
pub const fn modal_trigger(modes: &'static ModalDef, text: &'static str) -> AbilityDef {
    modal_triggered(YOU_CAST_YOUR_SECOND_SPELL, modes, text)
}
