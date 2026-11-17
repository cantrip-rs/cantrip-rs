use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Return target nonland permanent to its owner's hand; if you controlled that permanent, draw a card. Control is
/// read as the spell resolves, before the permanent leaves (`effects::return_first_target_to_hand_was_yours`).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::return_first_target_to_hand_was_yours(g, item)? {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT], effect),
    ..oracle::BOOMERANG_BASICS
};
