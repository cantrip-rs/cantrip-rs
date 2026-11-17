use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Draw a card and create a Treasure token": in that order (CR 608.2c), the draw finished before the token is made.
fn draw_and_create_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.end_sentence();
    tokens::create_a_treasure(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{R}").and(SACRIFICE_THIS),
        draw_and_create_a_treasure,
        "{2}{R}, Sacrifice this creature: Draw a card and create a Treasure token.",
    )],
    ..oracle::RECKLESS_LACKEY
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
