use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 2/2 white and blue Detective creature token, then investigate." (CR 701.16a): the token first, then the Clue.
fn detective_then_investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_a_white_blue_detective(g, item)?;
    tokens::you_investigate(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::creature_card_leaves_your_graveyard),
        detective_then_investigate,
        "Whenever one or more creature cards leave your graveyard, create a 2/2 white and blue Detective creature token, then investigate.",
    )
    .once_per_batch()],
    ..oracle::CHALK_OUTLINE
};

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLUE_DETECTIVE, &tokens::CLUE];
