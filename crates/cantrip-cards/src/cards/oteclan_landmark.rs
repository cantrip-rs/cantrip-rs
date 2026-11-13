use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{craft, effects, targets};
use cantrip_oracle as oracle;

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, effects::first_target_gains_flying, "Whenever this creature attacks, target attacking creature without flying gains flying until end of turn.")
        .targets(&[targets::ATTACKING_CREATURE_WITHOUT_FLYING])],
    ..oracle::OTECLAN_LEVITATOR
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::scry_two, "When this artifact enters, scry 2."),
        craft::ability(mana("{2}{W}"), &craft::ARTIFACT, "Craft with artifact {2}{W}"),
    ],
    back: Some(&BACK),
    ..oracle::OTECLAN_LANDMARK
};
