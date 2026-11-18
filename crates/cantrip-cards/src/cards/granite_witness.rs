use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_TURNED_FACE_UP;
use cantrip_mechanics::{choose, disguise, targets};
use cantrip_oracle as oracle;

/// "When this creature is turned face up, you may tap or untap target creature" (CR 701.26, 702.168): the target is chosen as the
/// ability goes on the stack, the choice made as it resolves. Disguise {W/U}{W/U} is the shared mechanic.
fn tap_or_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&CARD);
    choose::may_tap_or_untap_target(g, item, card)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_TURNED_FACE_UP,
        tap_or_untap,
        "When this creature is turned face up, you may tap or untap target creature.",
    )
    .targets(&[targets::CREATURE])],
    ways: &[disguise::way(Cost::mana(mana("{W/U}{W/U}")), "Disguise {W/U}{W/U}")],
    labels: choose::TAP_OR_UNTAP,
    ..oracle::GRANITE_WITNESS
};
