use cantrip_api::*;
use cantrip_mechanics::{effects, prepare};
use cantrip_oracle as oracle;

/// Spellbook Seeker's prepare spell (CR 722): not a card, only ever a copy, cast from exile (CR 722.3c).
pub static CAREFUL_STUDY: CardDef = CardDef {
    name: "Careful Study",
    cost: mana("{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], effects::draw_two_then_discard_two),
    text: "Draw two cards, then discard two cards.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&CAREFUL_STUDY),
    ..oracle::SPELLBOOK_SEEKER
};

pub static EXTRA: &[&CardDef] = &[&CAREFUL_STUDY];
