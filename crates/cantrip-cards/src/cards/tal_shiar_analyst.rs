use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, effects::damage_each_opponent_n::<1>, "{T}: This creature deals 1 damage to each opponent."),
        triggered(triggers::NONCREATURE_SPELL_YOU_CAST, effects::untap_self, "Whenever you cast a noncreature spell, untap this creature."),
    ],
    ..oracle::TAL_SHIAR_ANALYST
};
