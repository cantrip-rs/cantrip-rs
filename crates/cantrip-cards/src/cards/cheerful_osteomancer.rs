use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, targets};
use cantrip_oracle as oracle;

/// Cheerful Osteomancer's prepare spell (CR 722): not a card here, only ever a copy.
pub static RAISE_DEAD: CardDef = CardDef {
    name: "Raise Dead",
    cost: mana("{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], effects::return_first_target_to_hand),
    text: "Return target creature card from your graveyard to your hand.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&RAISE_DEAD),
    ..oracle::CHEERFUL_OSTEOMANCER
};

pub static EXTRA: &[&CardDef] = &[&RAISE_DEAD];
