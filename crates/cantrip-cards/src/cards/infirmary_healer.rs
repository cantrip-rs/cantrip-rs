use cantrip_api::*;
use cantrip_mechanics::{effects::target_player_gains_x_life, targets};
use cantrip_oracle as oracle;

/// Infirmary Healer's prepare spell (CR 722): not a card, only ever a copy.
pub static STREAM_OF_LIFE: CardDef = CardDef {
    name: "Stream of Life",
    cost: mana("{X}{G}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::PLAYER], target_player_gains_x_life),
    text: "Target player gains X life.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::prepare::enters_prepared),
    prepare: Some(&STREAM_OF_LIFE),
    ..oracle::INFIRMARY_HEALER
};

pub static EXTRA: &[&CardDef] = &[&STREAM_OF_LIFE];
