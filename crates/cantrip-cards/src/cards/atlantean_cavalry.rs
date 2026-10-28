use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_DRAW_YOUR_SECOND_CARD,
        plus_one_counter_on_this,
        "Whenever you draw your second card each turn, put a +1/+1 counter on this creature.",
    )],
    ..oracle::ATLANTEAN_CAVALRY
};
