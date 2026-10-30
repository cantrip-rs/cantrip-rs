use cantrip_api::*;
use cantrip_mechanics::{creatures_have_total_power_at_least, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever you attack while creatures you control have total power 8 or greater" (CR 508.3d): a condition of the trigger
/// event, checked only as attackers are declared and not again as the ability resolves (`only_if`, not an intervening "if",
/// CR 603.4), counting every creature the attacking player controls then, attacking or not.
fn eight_power(q: &dyn Query, asker: &Asker) -> bool {
    creatures_have_total_power_at_least(q, asker.controller, 8)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        effects::first_target_cant_block_this_turn,
        "Whenever you attack while creatures you control have total power 8 or greater, target creature can't block this turn.",
    )
    .only_if(eight_power)
    .targets(&[targets::CREATURE])],
    ..oracle::LAAN_NOONIEN_SINGH_SECURITY
};
