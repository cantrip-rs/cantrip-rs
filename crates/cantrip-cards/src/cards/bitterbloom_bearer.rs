use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::YOUR_UPKEEP};
use cantrip_oracle as oracle;

/// "You lose 1 life and create a 1/1 blue and black Faerie creature token with flying" (CR 119.3, 111.10f): the life is lost
/// first, and the token comes whether or not the life loss could be paid in full (losing life is not a cost).
fn lose_life_and_create_a_faerie(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 1);
    tokens::create(g, item.controller, &tokens::FAERIE_BLUE_BLACK, 1)?;
    Ok(())
}

/// Flash and flying are core keywords read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_UPKEEP,
        lose_life_and_create_a_faerie,
        "At the beginning of your upkeep, you lose 1 life and create a 1/1 blue and black Faerie creature token with flying.",
    )],
    ..oracle::BITTERBLOOM_BEARER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FAERIE_BLUE_BLACK];
