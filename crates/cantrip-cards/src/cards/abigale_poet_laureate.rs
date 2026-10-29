use cantrip_api::*;
use cantrip_mechanics::triggers::CREATURE_SPELL_YOU_CAST;
use cantrip_mechanics::{effects, prepare, targets};
use cantrip_oracle as oracle;

/// Abigale's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static HEROIC_STANZA: CardDef = CardDef {
    name: "Heroic Stanza",
    cost: mana("{1}{W/B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE], effects::plus_one_counter_on_target),
    text: "Put a +1/+1 counter on target creature.",
    ..CardDef::BLANK
};

/// Flying is a core keyword. "Whenever you cast a creature spell, Abigale becomes prepared" (CR 722.3a): the ability resolves
/// before the spell, even if it is countered. It does nothing if Abigale is gone or already prepared.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        CREATURE_SPELL_YOU_CAST,
        prepare::become_prepared,
        "Whenever you cast a creature spell, Abigale becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
    )],
    prepare: Some(&HEROIC_STANZA),
    ..oracle::ABIGALE_POET_LAUREATE
};

pub static EXTRA: &[&CardDef] = &[&HEROIC_STANZA];
