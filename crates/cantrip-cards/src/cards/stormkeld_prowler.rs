use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_with_mana_value_at_least::<5>),
        effects::two_plus_one_counters_on_this,
        "Whenever you cast a spell with mana value 5 or greater, put two +1/+1 counters on this creature.",
    )],
    ..oracle::STORMKELD_PROWLER
};
