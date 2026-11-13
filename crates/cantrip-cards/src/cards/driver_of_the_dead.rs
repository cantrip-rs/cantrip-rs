use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        effects::return_first_target_to_battlefield,
        "When this creature dies, return target creature card with mana value 2 or less from your graveyard to the battlefield.",
    )
    .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_2_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::DRIVER_OF_THE_DEAD
};
