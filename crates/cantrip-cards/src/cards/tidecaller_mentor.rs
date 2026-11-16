use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS, you_have_threshold};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::return_first_target_to_hand,
        "Threshold \u{2014} When this creature enters, if there are seven or more cards in your graveyard, return up to one target nonland permanent to its owner's hand.",
    )
    .targets(&[up_to_one(targets::NONLAND_PERMANENT.filter)])
    .condition(you_have_threshold)],
    ..oracle::TIDECALLER_MENTOR
};
