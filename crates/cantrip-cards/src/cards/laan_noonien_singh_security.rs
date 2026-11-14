use cantrip_api::*;
use cantrip_mechanics::{your_creatures_have_total_power_at_least, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        effects::first_target_cant_block_this_turn,
        "Whenever you attack while creatures you control have total power 8 or greater, target creature can't block this turn.",
    )
    .only_if(your_creatures_have_total_power_at_least::<8>)
    .targets(&[targets::CREATURE])],
    ..oracle::LAAN_NOONIEN_SINGH_SECURITY
};
