use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a Treasure token" (CR 111.10a).
fn create_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_treasure(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, create_treasure, "When this creature enters, create a Treasure token.")],
    ..oracle::FLAMEKIN_GILDWEAVER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
