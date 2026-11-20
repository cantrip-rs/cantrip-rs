use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Draw a card, then discard a card. Create a Treasure token." (CR 121.1, 701.9, 111.10a): the controller loots, then the
/// Treasure is made for them, in that order (CR 608.2c).
fn loot_and_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::loot(g, item)?;
    tokens::create_a_treasure(g, item)
}

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}").and(TAP),
        loot_and_treasure,
        "{2}, {T}: Draw a card, then discard a card. Create a Treasure token.",
    )],
    ..oracle::COLLECTORS_VAULT
};
