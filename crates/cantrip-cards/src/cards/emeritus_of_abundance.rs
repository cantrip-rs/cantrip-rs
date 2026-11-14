use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// Emeritus of Abundance's prepare spell (CR 722): not a card here, only ever a copy.
pub static REGROWTH: CardDef = CardDef {
    name: "Regrowth",
    cost: mana("{1}{G}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CARD_IN_YOUR_GRAVEYARD], cantrip_mechanics::effects::return_first_target_to_hand),
    text: "Return target card from your graveyard to your hand.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, cantrip_mechanics::prepare::become_prepared, "Whenever this creature attacks, if you control eight or more lands, this creature becomes prepared.")
        .condition(cantrip_mechanics::you_control_at_least_n_lands::<8>)],
    enters: Some(cantrip_mechanics::prepare::enters_prepared),
    prepare: Some(&REGROWTH),
    ..oracle::EMERITUS_OF_ABUNDANCE
};

pub static EXTRA: &[&CardDef] = &[&REGROWTH];
