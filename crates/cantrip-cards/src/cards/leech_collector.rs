use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::prepare::become_prepared;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Leech Collector's prepare spell (CR 722): not a card here, only ever a copy. "Each opponent loses 2 life."
pub static BLOODLETTING: CardDef = CardDef {
    name: "Bloodletting",
    cost: mana("{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], effects::each_opponent_loses_2_life),
    text: "Each opponent loses 2 life.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::LIFE_CHANGE, triggers::you_gain_life_for_the_first_time_each_turn),
        become_prepared,
        "Whenever you gain life for the first time each turn, this creature becomes prepared.",
    )],
    prepare: Some(&BLOODLETTING),
    ..oracle::LEECH_COLLECTOR
};

pub static EXTRA: &[&CardDef] = &[&BLOODLETTING];
