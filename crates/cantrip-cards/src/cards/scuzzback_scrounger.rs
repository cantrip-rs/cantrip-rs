use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::{blight, triggers};
use cantrip_oracle as oracle;

const BLIGHT: Question = Question::new("Blight", "blight 1 to create a Treasure token?", "", "Blight 1", "Don't");

/// "You may blight 1. If you do, create a Treasure token" (CR 701.68a, 118.12): the cost is paid as the trigger resolves.
fn treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if g.may_pay(you, item.object, item.object, blight::cost(1), &BLIGHT)? {
        tokens::create_treasure(g, you)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_FIRST_MAIN_PHASE,
        treasure,
        "At the beginning of your first main phase, you may blight 1. If you do, create a Treasure token.",
    )],
    ..oracle::SCUZZBACK_SCROUNGER
};
