use cantrip_api::*;
use cantrip_mechanics::crew::{becomes_a_creature, crew};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::noncreature_spell_you_cast_with_at_least_mana_spent::<4>),
            becomes_a_creature,
            "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, The Prima Vista becomes an artifact creature until end of turn.",
        ),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::THE_PRIMA_VISTA
};
