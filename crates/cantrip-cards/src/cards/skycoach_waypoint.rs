use cantrip_api::*;
use cantrip_mechanics::prepare::target_becomes_prepared;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{3}, {T}"), target_becomes_prepared, "{3}, {T}: Target creature becomes prepared.").targets(&[targets::CREATURE]),
    ],
    ..oracle::SKYCOACH_WAYPOINT
};
