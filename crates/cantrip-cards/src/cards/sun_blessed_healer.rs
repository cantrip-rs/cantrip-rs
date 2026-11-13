use cantrip_api::*;
use cantrip_mechanics::kicker::was_kicked;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{1}{W}")),
    abilities: &[triggered(
        THIS_ENTERS,
        effects::return_first_target_to_battlefield,
        "When this creature enters, if it was kicked, return target nonland permanent card with mana value 2 or less from your graveyard to the battlefield.",
    )
    .targets(&[targets::NONLAND_PERMANENT_CARD_WITH_MANA_VALUE_2_OR_LESS_IN_YOUR_GRAVEYARD])
    .condition(was_kicked)],
    ..oracle::SUN_BLESSED_HEALER
};
