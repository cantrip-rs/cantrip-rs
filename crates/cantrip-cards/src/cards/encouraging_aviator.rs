use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, targets, triggers};
use cantrip_oracle as oracle;

/// Encouraging Aviator's prepare spell (CR 722): not a card here, only ever a copy.
pub static JUMP: CardDef = CardDef {
    name: "Jump",
    cost: mana("{U}"),
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[targets::CREATURE], effects::first_target_gains_flying),
    text: "Target creature gains flying until end of turn.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, prepare::become_prepared, "Whenever this creature attacks, it becomes prepared.")],
    prepare: Some(&JUMP),
    ..oracle::ENCOURAGING_AVIATOR
};

pub static EXTRA: &[&CardDef] = &[&JUMP];
