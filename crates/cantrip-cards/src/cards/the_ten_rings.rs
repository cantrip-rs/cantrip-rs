use cantrip_api::*;
use cantrip_mechanics::{cards_in_hand, effects, triggers};
use cantrip_oracle as oracle;

/// The intervening "if you have fewer than ten cards in hand" (CR 603.4): asked as it triggers and as it resolves.
fn fewer_than_ten(q: &dyn Query, asker: &Asker) -> bool {
    cards_in_hand(q, asker.controller) < 10
}

/// Draw cards equal to the difference between ten and the cards in hand, counted as the ability resolves (CR 608.2h).
fn draw_the_difference(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let held = cards_in_hand(g, item.controller) as u32;
    effects::draw(g, item, 10_u32.saturating_sub(held))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        draw_the_difference,
        "At the beginning of your end step, if you have fewer than ten cards in hand, draw cards equal to the difference.",
    )
    .condition(fewer_than_ten)],
    play_rules: &[PlayRule::MaximumHandSize(Some(10))],
    ..oracle::THE_TEN_RINGS
};
