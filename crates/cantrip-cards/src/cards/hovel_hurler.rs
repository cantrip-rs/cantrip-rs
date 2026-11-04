use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<2>),
    abilities: &[ability(
        cost("{R/W}{R/W}").and(REMOVE_A_COUNTER),
        effects::target_creature_gets_and_gains_flying::<1, 0>,
        "{R/W}{R/W}, Remove a counter from this creature: Another target creature you control gets +1/+0 and gains flying until end of turn. Activate only as a sorcery.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])
    .sorcery_speed()],
    ..oracle::HOVEL_HURLER
};
