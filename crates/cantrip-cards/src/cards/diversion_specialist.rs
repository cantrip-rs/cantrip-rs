use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(sacrifice(1, targets::filters::another_creature_or_enchantment)),
        library::exile_top_card_play_this_turn,
        "{1}, Sacrifice another creature or enchantment: Exile the top card of your library. You may play it this turn.",
    )],
    ..oracle::DIVERSION_SPECIALIST
};
