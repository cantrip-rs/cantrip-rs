use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{destroy_target, targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target permanent an opponent controls. They create a Treasure token." (CR 701.8, 111.10a) "They" is the
/// target's controller as it resolves; they create the Treasure whether or not the destroy happened (indestructible,
/// regeneration). With the target illegal the ability does not resolve (CR 608.2b), so no Treasure is made.
fn destroy_and_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(target);
    destroy_target(g, item, 0);
    tokens::create_treasure(g, controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        destroy_and_treasure,
        "When Vraska enters, if you control six or more lands, destroy target permanent an opponent controls. They create a Treasure token.",
    )
    .condition(cantrip_mechanics::you_control_at_least_n_lands::<6>)
    .targets(&[targets::PERMANENT_AN_OPPONENT_CONTROLS])],
    ..oracle::VRASKA_THE_CUTTING_GLARE
};
