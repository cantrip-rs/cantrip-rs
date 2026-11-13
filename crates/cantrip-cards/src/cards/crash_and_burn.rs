use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target Vehicle" (CR 115.1, 205.3g): a permanent on the battlefield with the subtype Vehicle, whether or not it is a
/// creature right now.
fn vehicle(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Vehicle])
}

/// "Choose one — • Destroy target Vehicle. • Crash and Burn deals 6 damage to target creature or planeswalker." (CR 700.2).
static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target Vehicle.").targets(&[target(vehicle)]),
    mode(effects::damage_first_target::<6>, "Crash and Burn deals 6 damage to target creature or planeswalker.")
        .targets(&[targets::CREATURE_OR_PLANESWALKER]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CRASH_AND_BURN };
