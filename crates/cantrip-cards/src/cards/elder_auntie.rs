use cantrip_api::*;
use cantrip_mechanics::tokens::{self, BLACK_RED_GOBLIN};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Create a 1/1 black and red Goblin creature token."
fn goblin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &BLACK_RED_GOBLIN, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&BLACK_RED_GOBLIN];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, goblin, "When this creature enters, create a 1/1 black and red Goblin creature token.")],
    ..oracle::ELDER_AUNTIE
};
