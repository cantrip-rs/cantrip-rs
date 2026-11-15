use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODES: ModalDef = modes(&[
    mode(effects::target_creature_gets::<-1, -1>, "Target creature an opponent controls gets -1/-1 until end of turn.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
    mode(effects::destroy_first_target, "Destroy target creature an opponent controls that was dealt damage this turn.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS_THAT_WAS_DEALT_DAMAGE_THIS_TURN]),
]);

/// "When this creature enters, choose one —" (CR 700.2, 603.3c): the mode, and only that mode's target, is chosen as the ability goes on the stack.
pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODES,
        "When this creature enters, choose one — Target creature an opponent controls gets -1/-1 until end of turn; or destroy target creature an opponent controls that was dealt damage this turn.",
    )],
    ..oracle::DOWNWIND_AMBUSHER
};
