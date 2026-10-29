use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::REPARTEE};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        REPARTEE,
        effects::plus_one_counter_on_this,
        "Repartee \u{2014} Whenever you cast an instant or sorcery spell that targets a creature, put a +1/+1 counter on this creature.",
    )],
    ..oracle::LECTURING_SCORNMAGE
};
