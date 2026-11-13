use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::return_first_target_to_hand,
        "When this creature enters, if you control three or more creatures, return target permanent card from your graveyard to your hand.",
    )
    .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD])
    .condition(triggers::you_control_three_or_more_creatures)],
    ..oracle::DUNDOOLIN_WEAVER
};
