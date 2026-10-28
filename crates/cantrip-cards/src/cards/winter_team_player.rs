use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::NONCREATURE_SPELL_YOU_CAST,
        effects::creatures_you_control_get::<1, 0>,
        "Whenever you cast a noncreature spell, creatures you control get +1/+0 until end of turn.",
    )],
    ..oracle::WINTER_TEAM_PLAYER
};
