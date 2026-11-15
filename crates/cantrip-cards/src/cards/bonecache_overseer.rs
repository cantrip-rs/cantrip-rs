use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "Activate only if three or more cards left your graveyard this turn or if you've sacrificed a Food this turn" (CR 602.5):
/// asked of the event history as the ability is activated.
fn three_cards_left_your_graveyard_or_you_sacrificed_a_food(q: &dyn Query, asker: &Asker) -> bool {
    history::cards_left_graveyard_this_turn(q, asker.controller) >= 3 || history::sacrificed_a_food_this_turn(q, asker.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{T}").and(life(1)),
        |g, item| effects::draw(g, item, 1),
        "{T}, Pay 1 life: Draw a card. Activate only if three or more cards left your graveyard this turn or if you've sacrificed a Food this turn.",
    )
    .condition(three_cards_left_your_graveyard_or_you_sacrificed_a_food)],
    ..oracle::BONECACHE_OVERSEER
};
