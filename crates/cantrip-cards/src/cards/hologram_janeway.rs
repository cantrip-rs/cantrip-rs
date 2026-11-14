use cantrip_api::*;
use cantrip_mechanics::{your_creatures_have_total_power_at_least, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        effects::plus_one_counter_on_target,
        "Whenever you attack while creatures you control have total power 8 or greater, put a +1/+1 counter on target creature.",
    )
    .only_if(your_creatures_have_total_power_at_least::<8>)
    .targets(&[targets::CREATURE])],
    ..oracle::HOLOGRAM_JANEWAY
};
