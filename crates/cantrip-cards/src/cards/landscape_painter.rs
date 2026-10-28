use cantrip_api::*;
use cantrip_mechanics::{effects, prepare};
use cantrip_oracle as oracle;

/// Landscape Painter's prepare spell (CR 722): not a card, only ever a copy, cast from exile (CR 722.3c).
pub static VIBRANT_IDEA: CardDef = CardDef {
    name: "Vibrant Idea",
    cost: mana("{4}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], |g, item| effects::draw(g, item, 2)),
    text: "Draw two cards.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&VIBRANT_IDEA),
    ..oracle::LANDSCAPE_PAINTER
};

pub static EXTRA: &[&CardDef] = &[&VIBRANT_IDEA];
