use cantrip_api::*;
use cantrip_mechanics::effects::first_target_gains_vigilance_and_gets_plus_x_creatures_you_control;
use cantrip_mechanics::statics::{plus_pt, your_creature_tokens};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Hare Raising.
pub static HARE_RAISING: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], first_target_gains_vigilance_and_gets_plus_x_creatures_you_control),
    ..oracle::HARE_RAISING
};

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, your_creature_tokens, plus_pt::<1, 1>, "Creature tokens you control get +1/+1.")],
    adventure: Some(&HARE_RAISING),
    ..oracle::POLLEN_SHIELD_HARE
};

pub static EXTRA: &[&CardDef] = &[&HARE_RAISING];
