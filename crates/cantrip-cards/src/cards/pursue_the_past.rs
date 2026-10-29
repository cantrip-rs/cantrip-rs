use cantrip_api::*;
use cantrip_mechanics::{effects, flashback};
use cantrip_oracle as oracle;

/// "You gain 2 life. You may discard a card. If you do, draw two cards": the life first, then the discard is the controller's
/// choice as the spell resolves, and the two cards are drawn only if one was discarded (CR 608.2c-d, 701.9).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 2)?;
    effects::may_discard_to_draw(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{2}{R}{W}"), "Flashback {2}{R}{W}")],
    ..oracle::PURSUE_THE_PAST
};
