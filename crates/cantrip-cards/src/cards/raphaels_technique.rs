use cantrip_api::*;
use cantrip_mechanics::{effects::each_player_may_discard_hand_and_draw_seven, sneak};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], each_player_may_discard_hand_and_draw_seven),
    ways: &[sneak::way(mana("{2}{R}"), "Sneak {2}{R}")],
    ..oracle::RAPHAELS_TECHNIQUE
};
