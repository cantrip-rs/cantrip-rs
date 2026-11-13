use cantrip_api::*;
use cantrip_mechanics::triggers::INSTANT_OR_SORCERY_YOU_CAST;
use cantrip_mechanics::{cost_reduction, effects};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        INSTANT_OR_SORCERY_YOU_CAST,
        |g, item| effects::draw(g, item, 1),
        "Whenever you cast an instant or sorcery spell, draw a card.",
    )],
    cost_changes: &[cost_reduction::instants_and_sorceries_you_cast_cost_less::<1>],
    ..oracle::ARCHMAGE_OF_RUNES
};
