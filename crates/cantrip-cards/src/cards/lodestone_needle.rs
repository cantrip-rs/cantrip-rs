use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{craft, effects, targets};
use cantrip_oracle as oracle;

pub static BACK: CardDef = CardDef {
    abilities: &[ability(cost("{1}, {T}"), effects::target_creature_explores, "{1}, {T}: Target creature you control explores. Activate only as a sorcery.")
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .sorcery_speed()],
    ..oracle::GUIDESTONE_COMPASS
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::tap_and_stun::<2>, "When this artifact enters, tap up to one target artifact or creature and put two stun counters on it.")
            .targets(&[up_to_one(targets::artifact_or_creature_filter)]),
        craft::ability(mana("{2}{U}"), &craft::ARTIFACT, "Craft with artifact {2}{U}"),
    ],
    back: Some(&BACK),
    ..oracle::LODESTONE_NEEDLE
};
