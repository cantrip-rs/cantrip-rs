use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(sacrifice(1, targets::filters::another_creature)),
        effects::plus_one_counter_on_this,
        "{1}, Sacrifice another creature: Put a +1/+1 counter on this creature.",
    )],
    ..oracle::HUNGRY_GHOUL
};
