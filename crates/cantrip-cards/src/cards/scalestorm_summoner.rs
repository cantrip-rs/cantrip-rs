use cantrip_api::*;
use cantrip_mechanics::{ferocious, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 3/1 red Dinosaur creature token if you control a creature with power 4 or greater." The condition is part of the
/// instruction, not an intervening "if" (CR 603.4): the trigger always goes on the stack, the power is checked as it resolves,
/// and nothing is created when it fails.
fn dinosaur_if_power_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ferocious::you_control(g, item.controller) {
        tokens::create(g, item.controller, &tokens::DINOSAUR, 1)?;
    }
    Ok(())
}

/// "Whenever this creature attacks, create a 3/1 red Dinosaur creature token if you control a creature with power 4 or
/// greater."
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        dinosaur_if_power_four,
        "Whenever this creature attacks, create a 3/1 red Dinosaur creature token if you control a creature with power 4 or greater.",
    )],
    ..oracle::SCALESTORM_SUMMONER
};

pub static EXTRA: &[&CardDef] = &[&tokens::DINOSAUR];
