use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, effects::damage_first_target::<1>, "{T}: This creature deals 1 damage to target opponent.").targets(&[targets::OPPONENT]),
        triggered(triggers::NONCREATURE_SPELL_YOU_CAST, effects::untap_self, "Whenever you cast a noncreature spell, untap this creature."),
    ],
    ..oracle::KINDLESPARK_DUO
};
