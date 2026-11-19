use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::put_first_target_on_top_of_library,
        "When this creature enters, put up to one target noncreature, nonland card from your graveyard on top of your library.",
    )
    .targets(&[up_to_one(targets::noncreature_nonland_card_in_your_graveyard)])],
    ..oracle::MONASTERY_MESSENGER
};
