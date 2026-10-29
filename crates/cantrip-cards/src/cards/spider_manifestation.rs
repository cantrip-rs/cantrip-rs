use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Red, Color::Green]), "{T}: Add {R} or {G}."),
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_with_mana_value_at_least::<4>),
            effects::untap_self,
            "Whenever you cast a spell with mana value 4 or greater, untap this creature.",
        ),
    ],
    ..oracle::SPIDER_MANIFESTATION
};
