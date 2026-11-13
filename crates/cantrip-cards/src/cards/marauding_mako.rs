use cantrip_api::*;
use cantrip_mechanics::effects::that_many_plus_one_counters_on_this;
use cantrip_mechanics::triggers::cycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            cantrip_mechanics::triggers::YOU_DISCARD,
            that_many_plus_one_counters_on_this,
            "Whenever you discard one or more cards, put that many +1/+1 counters on this creature.",
        )
        .once_per_batch(),
        cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)"),
    ],
    ..oracle::MARAUDING_MAKO
};
