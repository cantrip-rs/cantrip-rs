use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, YOU_ATTACK};
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Surveil 1", by the delayed ability's controller.
fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        delayed::target_creature_gets_plus_power_and_a_delayed_ability_for_when_it_dies::<1>,
        "Whenever you attack, target attacking creature gets +1/+0 until end of turn. When that creature dies this turn, surveil 1.",
    )
    .targets(&[targets::ATTACKING_CREATURE])],
    delayed: &[triggered(THIS_DIES, surveil_one, "When that creature dies this turn, surveil 1.")],
    ..oracle::GRIM_JAVELINEER
};
