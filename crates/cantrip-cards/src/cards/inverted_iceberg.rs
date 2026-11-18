use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{choose, craft, effects, targets};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, you may tap or untap target artifact or creature."
fn tap_or_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&BACK);
    choose::may_tap_or_untap_target(g, item, card)
}

pub static BACK: CardDef = CardDef {
    labels: choose::TAP_OR_UNTAP,
    abilities: &[triggered(THIS_ATTACKS, tap_or_untap, "Whenever this creature attacks, you may tap or untap target artifact or creature.")
        .targets(&[targets::ARTIFACT_OR_CREATURE])],
    ..oracle::ICEBERG_TITAN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::you_mill_then_draw::<1, 1>, "When this artifact enters, mill a card, then draw a card."),
        craft::ability(mana("{4}{U}{U}"), &craft::ARTIFACT, "Craft with artifact {4}{U}{U}"),
    ],
    back: Some(&BACK),
    ..oracle::INVERTED_ICEBERG
};
