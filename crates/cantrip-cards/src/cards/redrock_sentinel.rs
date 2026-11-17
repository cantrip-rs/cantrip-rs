use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// Defender is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}, {T}").and(sacrifice(1, targets::filters::land)),
        effects::draw_cards_and_create_a_treasure::<1>,
        "{2}, {T}, Sacrifice a land: Draw a card and create a Treasure token.",
    )],
    ..oracle::REDROCK_SENTINEL
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
