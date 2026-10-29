use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Draw two cards and create a Treasure token": in that order (CR 608.2c), the draw finished before the token is made.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 2)?;
    g.end_sentence();
    tokens::create_treasure(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    // "As an additional cost to cast this spell, discard a card" (CR 601.2b, 118.8): exactly one, paid with the other costs.
    additional_cost: Some(discard_cards(1)),
    ..oracle::SEIZE_THE_SPOILS
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
