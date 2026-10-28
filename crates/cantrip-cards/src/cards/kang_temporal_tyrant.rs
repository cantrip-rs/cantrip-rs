use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, YOU_DRAW_YOUR_SECOND_CARD};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ATTACKS, effects::this_connives, "Whenever Kang attacks, he connives. (Draw a card, then discard a card. If you discarded a nonland card, put a +1/+1 counter on this creature.)"),
        triggered(
            YOU_DRAW_YOUR_SECOND_CARD,
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever you draw your second card each turn, each opponent loses 1 life and you gain 1 life.",
        ),
    ],
    ..oracle::KANG_TEMPORAL_TYRANT
};
