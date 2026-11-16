use cantrip_api::*;
use cantrip_mechanics::{effects, history::you_havent_cast_a_spell_from_your_hand, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}{U}"), effects::this_gets::<2, -2>, "{1}{U}: This creature gets +2/-2 until end of turn."),
        triggered(
            YOUR_END_STEP,
            effects::loot,
            "At the beginning of your end step, if you haven't cast a spell from your hand this turn, draw a card, then discard a card.",
        )
        .condition(you_havent_cast_a_spell_from_your_hand),
    ],
    ..oracle::CANYON_CRAB
};
