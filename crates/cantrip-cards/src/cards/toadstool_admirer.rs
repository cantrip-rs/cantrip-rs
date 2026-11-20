use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::triggers::ward;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2}"),
        ability(cost("{3}{G}"), plus_one_counter_on_this, "{3}{G}: Put a +1/+1 counter on this creature."),
    ],
    ..oracle::TOADSTOOL_ADMIRER
};
