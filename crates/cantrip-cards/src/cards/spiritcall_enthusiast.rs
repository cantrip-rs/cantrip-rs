use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, targets, triggers};
use cantrip_oracle as oracle;

/// Spiritcall Enthusiast's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static SCROLLBOOST: CardDef = CardDef {
    name: "Scrollboost",
    cost: mana("{1}{W}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE.between(1, 2)], effects::targets_get::<2, 2>),
    text: "One or two target creatures each get +2/+2 until end of turn.",
    ..CardDef::BLANK
};

/// "Whenever one or more tokens you control enter, this creature becomes prepared": one trigger for a batch (CR 603.2c).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::token_you_control_enters),
        prepare::become_prepared,
        "Whenever one or more tokens you control enter, this creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
    )
    .once_per_batch()],
    prepare: Some(&SCROLLBOOST),
    ..oracle::SPIRITCALL_ENTHUSIAST
};

pub static EXTRA: &[&CardDef] = &[&SCROLLBOOST];
