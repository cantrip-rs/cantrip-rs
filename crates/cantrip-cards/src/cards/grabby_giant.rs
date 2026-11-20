use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): That's Mine, "Create a Treasure token."
pub static THATS_MINE: CardDef = CardDef { spell: spell(&[], tokens::create_a_treasure), ..oracle::THATS_MINE };

// Reach is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{R}").and(sacrifice(1, targets::filters::artifact_or_land)), |g, item| effects::draw(g, item, 1), "{2}{R}, Sacrifice an artifact or land: Draw a card.")],
    adventure: Some(&THATS_MINE),
    ..oracle::GRABBY_GIANT
};

pub static EXTRA: &[&CardDef] = &[&THATS_MINE, &tokens::TREASURE];
