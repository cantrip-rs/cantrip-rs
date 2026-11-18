use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{delayed, haste_until_end_of_turn, tokens};
use cantrip_oracle as oracle;

pub use cantrip_mechanics::tokens::THOPTER;

/// "Create a 1/1 colorless Thopter artifact creature token with flying. It gains haste until end of turn. Sacrifice it at the
/// beginning of your next end step." (CR 111.1, 611.2a, 603.7c): the Thopter enters, gains haste for the turn (CR 702.10), and a
/// delayed ability about exactly that token (CR 603.7c, 400.7) sacrifices it at the beginning of its controller's next end step,
/// this turn's as the trigger resolves in the beginning of combat step.
fn thopter_that_is_sacrificed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &THOPTER, 1)?;
    g.end_sentence();
    instruction(g, |g| {
        for &token in &made {
            haste_until_end_of_turn(g, item.object, token);
        }
    });
    g.end_sentence();
    delayed::at_your_next_end_step_about(g, item, 0, &made);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        thopter_that_is_sacrificed,
        "At the beginning of combat on your turn, create a 1/1 colorless Thopter artifact creature token with flying. It gains haste until end of turn. Sacrifice it at the beginning of your next end step.",
    )],
    delayed: &[delayed_trigger(delayed::sacrifice_created, "Sacrifice it at the beginning of your next end step.")],
    ..oracle::HARRIED_DRONESMITH
};

pub static EXTRA: &[&CardDef] = &[&THOPTER];
