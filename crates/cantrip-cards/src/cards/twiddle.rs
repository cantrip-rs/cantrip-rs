use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_oracle as oracle;

/// Choose whether to tap, untap or do nothing as the spell resolves (CR 608.2d, 701.26).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&CARD);
    choose::may_tap_or_untap_target(g, item, card)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_CREATURE_OR_LAND], effect),
    labels: choose::TAP_OR_UNTAP,
    ..oracle::TWIDDLE
};
