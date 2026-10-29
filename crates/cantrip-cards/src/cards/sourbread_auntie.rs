use cantrip_api::*;
use cantrip_mechanics::tokens::{self, BLACK_RED_GOBLIN};
use cantrip_mechanics::{blight, triggers};
use cantrip_oracle as oracle;

const BLIGHT: Question = Question::new("Blight", "blight 2 to create two 1/1 black and red Goblin creature tokens?", "", "Blight 2", "Don't");

/// "When this creature enters, you may blight 2. If you do, create two 1/1 black and red Goblin creature tokens"
/// (CR 701.68a, 118.12): the cost is paid as the trigger resolves, and the tokens come only if it was.
fn goblins(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if g.may_pay(you, item.object, item.object, blight::cost(2), &BLIGHT)? {
        tokens::create(g, you, &BLACK_RED_GOBLIN, 2)?;
    }
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&BLACK_RED_GOBLIN];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, goblins, "When this creature enters, you may blight 2. If you do, create two 1/1 black and red Goblin creature tokens.")],
    ..oracle::SOURBREAD_AUNTIE
};
