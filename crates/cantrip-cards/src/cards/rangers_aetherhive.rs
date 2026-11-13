use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_ACTIVATE_AN_EXHAUST_ABILITY,
            tokens::create_a_thopter,
            "Whenever you activate an exhaust ability, create a 1/1 colorless Thopter artifact creature token with flying.",
        ),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::RANGERS_AETHERHIVE
};

pub static EXTRA: &[&CardDef] = &[&tokens::THOPTER];
