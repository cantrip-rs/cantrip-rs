use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Reach is a core keyword.
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_with_mana_value_at_least::<5>),
        tokens::create_a_food_token,
        "Whenever you cast a spell with mana value 5 or greater, create a Food token. (It's an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")",
    )],
    ..oracle::SKYBEAST_TRACKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
