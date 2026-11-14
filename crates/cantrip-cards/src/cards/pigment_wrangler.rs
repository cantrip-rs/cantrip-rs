use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, prepare};
use cantrip_oracle as oracle;

/// Pigment Wrangler's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static STRIKING_PALETTE: CardDef = CardDef {
    name: "Striking Palette",
    cost: mana("{R}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], delayed::watch_for_the_next),
    delayed: &[copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST],
    text: "When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&STRIKING_PALETTE),
    ..oracle::PIGMENT_WRANGLER
};

pub static EXTRA: &[&CardDef] = &[&STRIKING_PALETTE];
