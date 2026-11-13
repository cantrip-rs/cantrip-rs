use cantrip_api::*;
use cantrip_mechanics::{effects, fight::targets_fight, targets};
use cantrip_oracle as oracle;

/// "Target Vehicle" (CR 115.1, 301.7): a permanent with the Vehicle subtype, whoever controls it and whether or not it is
/// crewed.
fn vehicle(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Vehicle])
}

/// "Target creature you control fights target creature an opponent controls" (CR 701.14a): if either target is illegal as the
/// spell resolves, neither creature fights (CR 701.14b, 608.2b).
fn fight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    targets_fight(g, item, 0, 1);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(fight, "Target creature you control fights target creature an opponent controls. (Each deals damage equal to its power to the other.)")
        .targets(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS]),
    mode(effects::destroy_first_target, "Destroy target Vehicle.").targets(&[target(vehicle)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::PLOW_THROUGH };
