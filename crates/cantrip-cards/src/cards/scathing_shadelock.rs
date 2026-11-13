use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, targets, triggers};
use cantrip_oracle as oracle;

/// Scathing Shadelock's prepare spell (CR 722): not a card here, only ever a copy.
pub static VENOMOUS_WORDS: CardDef = CardDef {
    name: "Venomous Words",
    cost: mana("{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effects::target_creature_gets_and_gains_deathtouch::<2, 0>),
    text: "Target creature you control gets +2/+0 and gains deathtouch until end of turn.",
    ..CardDef::BLANK
};

/// "It becomes prepared" (CR 722.3a): nothing if it left the battlefield, or already is prepared.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_FIRST_MAIN_PHASE,
        prepare::become_prepared,
        "At the beginning of your first main phase, this creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
    )],
    prepare: Some(&VENOMOUS_WORDS),
    ..oracle::SCATHING_SHADELOCK
};

pub static EXTRA: &[&CardDef] = &[&VENOMOUS_WORDS];
