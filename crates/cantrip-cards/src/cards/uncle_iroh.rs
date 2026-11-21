use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, firebending, lands};
use cantrip_oracle as oracle;

/// "Lesson spells you cast cost {1} less to cast" (CR 601.2f): a Lesson is an instant or sorcery with the subtype, read from the spell's
/// own characteristics; only the controller's spells. Firebending 1 is the shared trigger.
pub static CARD: CardDef = CardDef {
    abilities: &[firebending::trigger::<1>("Firebending 1 (Whenever this creature attacks, add {R}. This mana lasts until end of combat.)")],
    cost_changes: &[cost_reduction::subtype_spells_you_cast::<1, { lands::sub(Subtype::Lesson) }>],
    ..oracle::UNCLE_IROH
};
