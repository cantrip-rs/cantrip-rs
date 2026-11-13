use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER,
            effects::damage_each_opponent_n::<1>,
            "Whenever you cast a spell with mana value 4 or greater, this creature deals 1 damage to each opponent.",
        ),
        ability(cost("{5}{R}"), effects::two_plus_one_counters_on_this, "{5}{R}: Put two +1/+1 counters on this creature. Activate only as a sorcery.").sorcery_speed(),
    ],
    ..oracle::ANGRY_RABBLE
};
