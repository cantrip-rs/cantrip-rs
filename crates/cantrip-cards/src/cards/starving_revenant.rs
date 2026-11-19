use cantrip_api::*;
use cantrip_mechanics::{effects, surveil, targets, top_of_library, triggers};
use cantrip_oracle as oracle;

/// "Surveil 2. Then for each card you put on top of your library, you draw a card and you lose 3 life" (CR 701.25a, 608.2c): the
/// surveiled cards that are still in the library afterwards are the ones put on top, and the count scales the instruction: draw that
/// many cards, then lose 3 life for each as one life loss (CR 119.3), even if the library has run out (CR 121.4).
fn surveil_2_then_draw_and_lose_3_life_for_each_card_put_on_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let looked = top_of_library(g, item.controller, 2);
    surveil(g, item.controller, item.object, 2)?;
    let library = g.state().zone(Zone::library(item.controller));
    let kept = looked.iter().filter(|card| library.contains(card)).count();
    g.draw(item.controller, kept as u32);
    g.lose_life(item.controller, 3 * kept as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            surveil_2_then_draw_and_lose_3_life_for_each_card_put_on_top,
            "When this creature enters, surveil 2. Then for each card you put on top of your library, you draw a card and you lose 3 life.",
        ),
        triggered(
            triggers::YOU_DRAW_A_CARD,
            effects::target_player_loses_1_life_you_gain_1,
            "Descend 8 \u{2014} Whenever you draw a card, if there are eight or more permanent cards in your graveyard, target opponent loses 1 life and you gain 1 life.",
        )
        .condition(targets::you_have_permanent_cards_in_graveyard::<8>)
        .targets(&[targets::OPPONENT]),
    ],
    ..oracle::STARVING_REVENANT
};
