use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Red, Color::Green]), "{T}: Add {R} or {G}."),
        triggered(
            YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER,
            effects::untap_self,
            "Whenever you cast a spell with mana value 4 or greater, untap this creature.",
        ),
    ],
    ..oracle::SPIDER_MANIFESTATION
};
