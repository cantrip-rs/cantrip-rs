use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::an_opponent_casts_a_spell),
        effects::may_put_a_plus_one_counter_on_this,
        "Whenever an opponent casts a spell, you may put a +1/+1 counter on this creature.",
    )],
    ..oracle::TAUREAN_MAULER
};
