use cantrip_api::*;
use cantrip_mechanics::{effects, offspring, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{B}"))),
    abilities: &[
        offspring::trigger("Offspring {B} (You may pay an additional {B} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(
            triggers::YOUR_UPKEEP,
            effects::reveal_top_card_to_hand_you_lose_life_equal_to_its_mana_value,
            "At the beginning of your upkeep, reveal the top card of your library and put that card into your hand. You lose life equal to its mana value.",
        ),
    ],
    ..oracle::DARKSTAR_AUGUR
};
