use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{effects, targets, triggers::BEGINNING_OF_YOUR_COMBAT};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("GetPlusTwoPower", "have the targeted creature get +2/+0 until end of turn?", "Chosen as the ability resolves", "Get +2/+0", "Don't");

/// "you may have target creature get +2/+0 until end of turn" (CR 611.2a, 608.2b): the target is mandatory and chosen as the trigger
/// goes on the stack, and the "may" is asked as it resolves. If the creature is no longer a legal target the ability does not
/// resolve at all, so nothing is asked.
fn may_have_target_creature_get_plus_two_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        effects::target_creature_gets::<2, 0>(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        may_have_target_creature_get_plus_two_power,
        "At the beginning of combat on your turn, you may have target creature get +2/+0 until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::BATTLE_RATTLE_SHAMAN
};
