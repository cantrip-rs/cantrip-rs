use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        cantrip_mechanics::triggers::YOUR_UPKEEP,
        effects::reveal_top_card_to_hand_you_lose_life_equal_to_its_mana_value,
        "At the beginning of your upkeep, reveal the top card of your library and put that card into your hand. You lose life equal to its mana value.",
    )],
    ..oracle::DARK_CONFIDANT
};
