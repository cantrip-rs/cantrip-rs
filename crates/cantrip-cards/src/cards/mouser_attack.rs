use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Create a 1/1 colorless Robot artifact creature token."
fn robot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ROBOT_1_1, 1)?;
    Ok(())
}

/// "Target creature gets +3/+0 and gains first strike until end of turn" (CR 611.2a): one instruction. Nothing if the target
/// is no longer legal as the spell resolves (CR 608.2b).
fn bash(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 3, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    });
    Ok(())
}

/// "Choose one" (CR 700.2).
static MODAL: ModalDef = modes(&[
    mode(robot, "Create a 1/1 colorless Robot artifact creature token."),
    mode(bash, "Target creature gets +3/+0 and gains first strike until end of turn.").targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::MOUSER_ATTACK };

pub static EXTRA: &[&CardDef] = &[&tokens::ROBOT_1_1];
