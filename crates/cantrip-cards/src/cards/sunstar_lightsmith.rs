use cantrip_api::*;
use cantrip_mechanics::{effects, flurry};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger(
        "Whenever you cast your second spell each turn, put a +1/+1 counter on this creature and draw a card.",
        effects::plus_one_counter_on_this_and_draw_a_card,
    )],
    ..oracle::SUNSTAR_LIGHTSMITH
};
