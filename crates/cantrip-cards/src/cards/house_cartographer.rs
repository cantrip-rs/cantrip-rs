use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

/// Survival: "reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the
/// rest on the bottom of your library in a random order."
fn reveal_until_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::reveal_until_to_hand_rest_to_bottom(g, item.controller, &|q, c| q.chars(c).is(CardType::Land));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::survival(
        reveal_until_land,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.",
    )],
    ..oracle::HOUSE_CARTOGRAPHER
};
