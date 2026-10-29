use cantrip_api::*;
use cantrip_mechanics::{effects::each_player_may_discard_hand_and_draw_seven, triggers::PROWESS};
use cantrip_oracle as oracle;

/// Variable Chaser's prepare spell (CR 722.3c), a copy cast from exile.
pub static ARC_OF_FORTUNE: CardDef = CardDef {
    name: "Arc of Fortune",
    cost: mana("{2}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], each_player_may_discard_hand_and_draw_seven),
    text: "Each player may discard their hand and draw seven cards.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[PROWESS],
    enters: Some(cantrip_mechanics::prepare::enters_prepared),
    prepare: Some(&ARC_OF_FORTUNE),
    ..oracle::VARIABLE_CHASER
};

pub static EXTRA: &[&CardDef] = &[&ARC_OF_FORTUNE];
