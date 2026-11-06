use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(remove_counters(CounterKind::Javelin, 1)),
        effects::damage_first_target::<1>,
        "{T}, Remove a javelin counter from this creature: It deals 1 damage to any target.",
    ).targets(&[targets::ANY])],
    enters: Some(effects::enters_with_a_counter::<{ CounterKind::Javelin as usize }>),
    ..oracle::ICATIAN_JAVELINEERS
};
