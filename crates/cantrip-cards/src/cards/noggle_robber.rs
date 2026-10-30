use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Create a Treasure token." (CR 111.10a)
fn create_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_treasure(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // "Enters or dies" is one event or the other, never both at once, so it is written as the two triggers it is (CR 603.6a, 603.10a).
        triggered(THIS_ENTERS, create_treasure, "When this creature enters, create a Treasure token."),
        triggered(THIS_DIES, create_treasure, "When this creature dies, create a Treasure token."),
    ],
    ..oracle::NOGGLE_ROBBER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
