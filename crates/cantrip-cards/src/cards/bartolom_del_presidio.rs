use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(1, filters::another_creature_or_artifact),
        plus_one_counter_on_this,
        "Sacrifice another creature or artifact: Put a +1/+1 counter on Bartolom\u{e9} del Presidio.",
    )],
    ..oracle::BARTOLOM_DEL_PRESIDIO
};
