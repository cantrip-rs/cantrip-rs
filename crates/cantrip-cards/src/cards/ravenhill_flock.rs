use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOU_DRAW_A_CARD;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_DRAW_A_CARD, effects::plus_one_counter_on_this, "Whenever you draw a card, put a +1/+1 counter on this creature.")],
    ..oracle::RAVENHILL_FLOCK
};
