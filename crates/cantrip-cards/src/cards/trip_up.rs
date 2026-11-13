use cantrip_api::*;
use cantrip_mechanics::{library, targets, triggers};
use cantrip_oracle as oracle;

/// "Target nonland permanent's owner puts it on their choice of the top or bottom of their library." (CR 108.3, 401.1): its
/// owner, not its controller, chooses; with the target gone the spell does nothing (CR 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT], library::owner_puts_first_target_on_top_or_bottom),
    abilities: &[triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::TRIP_UP
};
