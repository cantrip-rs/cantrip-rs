use cantrip_api::*;
use cantrip_mechanics::{bargain, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::return_first_target_to_hand,
        "When this creature enters, if it was bargained, return target instant or sorcery card from your graveyard to your hand.",
    )
    .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD])
    .condition(bargain::was_bargained)],
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::TENACIOUS_TOMESEEKER
};
