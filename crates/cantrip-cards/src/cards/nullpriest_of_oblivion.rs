use cantrip_api::*;
use cantrip_mechanics::{effects, kicker, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{3}{B}")),
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::return_first_target_to_battlefield,
        "When this creature enters, if it was kicked, return target creature card from your graveyard to the battlefield.",
    )
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])
    .condition(kicker::was_kicked)],
    ..oracle::NULLPRIEST_OF_OBLIVION
};
