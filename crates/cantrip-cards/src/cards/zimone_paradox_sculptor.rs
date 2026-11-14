use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{double, effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            effects::plus_one_counter_on_each_target,
            "At the beginning of combat on your turn, put a +1/+1 counter on each of up to two target creatures you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.between(0, 2)]),
        ability(
            cost("{G}{U}").and(TAP),
            double::each_kind_of_counter_on_each_target,
            "{G}{U}, {T}: Double the number of each kind of counter on up to two target creatures and/or artifacts you control.",
        )
        .targets(&[targets::ARTIFACT_OR_CREATURE_YOU_CONTROL.between(0, 2)]),
    ],
    ..oracle::ZIMONE_PARADOX_SCULPTOR
};
