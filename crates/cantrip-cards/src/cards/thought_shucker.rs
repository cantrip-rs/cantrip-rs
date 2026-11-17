use cantrip_api::*;
use cantrip_mechanics::{effects, you_have_threshold};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{U}"),
        effects::plus_one_counter_on_this_and_draw_a_card,
        "Threshold \u{2014} {1}{U}: Put a +1/+1 counter on this creature and draw a card. Activate only if there are seven or more cards in your graveyard and only once.",
    )
    .once()
    .condition(you_have_threshold)],
    ..oracle::THOUGHT_SHUCKER
};
