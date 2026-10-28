use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::REPARTEE};
use cantrip_oracle as oracle;

/// "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, put a +1/+1 counter on target
/// creature you control." The target is chosen when the ability goes on the stack, so it can be the creature the spell targets
/// or another; with none to choose, the ability is removed from the stack (CR 603.3d).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        REPARTEE,
        effects::plus_one_counter_on_target,
        "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, put a +1/+1 counter on target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::GRADUATION_DAY
};
