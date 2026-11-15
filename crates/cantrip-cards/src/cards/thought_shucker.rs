use cantrip_api::*;
use cantrip_mechanics::{effects, you_have_threshold};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature and draw a card" (CR 122.1, 121.1): the counter goes on only if it is still on the
/// battlefield (CR 400.7), the draw happens regardless.
fn grow_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{U}"),
        grow_and_draw,
        "Threshold \u{2014} {1}{U}: Put a +1/+1 counter on this creature and draw a card. Activate only if there are seven or more cards in your graveyard and only once.",
    )
    .once()
    .condition(you_have_threshold)],
    ..oracle::THOUGHT_SHUCKER
};
