use cantrip_api::*;
use cantrip_mechanics::triggers::{self, ward};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2}"),
        triggered(triggers::YOU_ATTACK, effects::creatures_you_control_get::<1, 1>, "Whenever you attack, creatures you control get +1/+1 until end of turn."),
    ],
    ..oracle::LIONHEART_GLIMMER
};
