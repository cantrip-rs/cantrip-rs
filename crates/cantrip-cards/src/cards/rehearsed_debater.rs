use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::REPARTEE};
use cantrip_oracle as oracle;

/// Vigilance is a core keyword. "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, this
/// creature gets +1/+1 until end of turn." Each such spell adds its own, and they stack.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        REPARTEE,
        effects::this_gets::<1, 1>,
        "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, this creature gets +1/+1 until end of turn.",
    )],
    ..oracle::REHEARSED_DEBATER
};
