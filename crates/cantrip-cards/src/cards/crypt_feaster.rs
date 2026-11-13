use cantrip_api::*;
use cantrip_mechanics::{effects, triggers, you_have_threshold};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        effects::this_gets::<2, 0>,
        "Threshold \u{2014} Whenever this creature attacks, if there are seven or more cards in your graveyard, this creature gets +2/+0 until end of turn.",
    )
    .condition(you_have_threshold)],
    ..oracle::CRYPT_FEASTER
};
