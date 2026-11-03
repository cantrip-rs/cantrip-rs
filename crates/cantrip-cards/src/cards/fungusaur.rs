use cantrip_api::*;
use cantrip_mechanics::{effects::plus_one_counter_on_this, triggers::THIS_TAKES_DAMAGE};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_TAKES_DAMAGE,
        plus_one_counter_on_this,
        "Whenever this creature is dealt damage, put a +1/+1 counter on it.",
    ).damage_amount().sums_per_subject()],
    ..oracle::FUNGUSAUR
};
