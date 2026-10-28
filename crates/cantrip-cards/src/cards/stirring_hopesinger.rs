use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::REPARTEE};
use cantrip_oracle as oracle;

/// Flying and lifelink are core keywords. "Repartee — Whenever you cast an instant or sorcery spell that targets a
/// creature, put a +1/+1 counter on each creature you control": on the creatures its controller controls as it resolves,
/// all at once (CR 608.2, 101.4).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        REPARTEE,
        effects::plus_one_counter_on_each_creature_you_control,
        "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, put a +1/+1 counter on each creature you control.",
    )],
    ..oracle::STIRRING_HOPESINGER
};
