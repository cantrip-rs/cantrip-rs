use cantrip_api::*;
use cantrip_mechanics::{effects, flurry};
use cantrip_oracle as oracle;

/// "Flying" is a core keyword (CR 702.9). Flurry (CR 207.2c): "Whenever you cast your second spell each turn", spells cast
/// before Kraum entered count (ruling 2024-04-12).
pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger(
        "Whenever you cast your second spell each turn, put a +1/+1 counter on Kraum and draw a card.",
        effects::plus_one_counter_on_this_and_draw_a_card,
    )],
    ..oracle::KRAUM_VIOLENT_CACOPHONY
};
