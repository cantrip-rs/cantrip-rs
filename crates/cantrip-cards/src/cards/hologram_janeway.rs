use cantrip_api::*;
use cantrip_mechanics::{creatures_have_total_power_at_least, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "While creatures you control have total power 8 or greater" checks only on attack (CR 508.3d, 603.4).
fn eight_power(q: &dyn Query, asker: &Asker) -> bool {
    creatures_have_total_power_at_least(q, asker.controller, 8)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        effects::plus_one_counter_on_target,
        "Whenever you attack while creatures you control have total power 8 or greater, put a +1/+1 counter on target creature.",
    )
    .only_if(eight_power)
    .targets(&[targets::CREATURE])],
    ..oracle::HOLOGRAM_JANEWAY
};
