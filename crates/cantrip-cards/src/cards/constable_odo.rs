use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Investigate twice." (CR 701.16a)
fn investigate_twice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for _ in 0..2 {
        tokens::investigate(g, item.controller)?;
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::tap_and_stun::<2>, "Tap target creature an opponent controls. Put two stun counters on it.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
    mode(investigate_twice, "Investigate twice."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When Constable Odo enters, choose one —\n• Tap target creature an opponent controls. Put two stun counters on it.\n• Investigate twice.",
    )],
    ..oracle::CONSTABLE_ODO
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
