use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{destroy_target, targets};
use cantrip_oracle as oracle;

/// "When Red Guardian enters, destroy target creature an opponent controls that dealt damage this turn." (CR 701.8, 120.2): the
/// target is judged as it is chosen and again as the ability resolves (CR 608.2b), so a creature that has left the battlefield
/// since is a new object that dealt nothing.
fn destroy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        destroy_it,
        "When Red Guardian enters, destroy target creature an opponent controls that dealt damage this turn.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS_THAT_DEALT_DAMAGE_THIS_TURN])],
    ..oracle::RED_GUARDIAN_SUPER_SOLDIER
};
