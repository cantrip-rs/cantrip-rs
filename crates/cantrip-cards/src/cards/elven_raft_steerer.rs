use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::LANDFALL};
use cantrip_oracle as oracle;

/// "Tap target creature an opponent controls" (CR 701.26a): nothing if it is no longer a legal target (CR 608.2b).
fn tap_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    Ok(())
}

/// "Choose one" as the trigger goes on the stack (CR 603.3c, 700.2a), with the chosen mode's target (CR 603.3d).
static MODAL: ModalDef = modes(&[
    mode(tap_it, "Tap target creature an opponent controls.").targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
    mode(effects::untap_targets, "Untap target creature you control.").targets(&[targets::CREATURE_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        LANDFALL,
        &MODAL,
        "Landfall \u{2014} Whenever a land you control enters, choose one \u{2014}\n\u{2022} Tap target creature an opponent controls.\n\u{2022} Untap target creature you control.",
    )],
    ..oracle::ELVEN_RAFT_STEERER
};
