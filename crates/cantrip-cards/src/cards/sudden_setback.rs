use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "The owner of target spell or nonland permanent puts it on their choice of the top or bottom of their library." The owner
/// (CR 108.3), not the controller, chooses; a spell leaves the stack without being countered (`library::owner_puts_first_target_on_top_or_bottom`).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL_OR_NONLAND_PERMANENT], library::owner_puts_first_target_on_top_or_bottom),
    ..oracle::SUDDEN_SETBACK
};
