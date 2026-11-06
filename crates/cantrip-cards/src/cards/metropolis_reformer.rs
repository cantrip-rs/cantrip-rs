use cantrip_api::*;
use cantrip_mechanics::{effects::gain_matched_damage_life, statics, triggers::this_takes_damage};
use cantrip_oracle as oracle;

/// Flying and vigilance are core keywords. "You have hexproof" (CR 702.11c) is a player keyword ability of the permanent
/// (`statics::YOU_HAVE_HEXPROOF`). "Whenever this creature is dealt damage, you gain that much life" (CR 120.4b, 603.2c): the
/// damage dealt at once, even lethal, summed per subject (`sums_per_subject`), gained by the ability's controller.
pub static CARD: CardDef = CardDef {
    player_keywords: &[statics::YOU_HAVE_HEXPROOF],
    abilities: &[triggered(
        when(EventMask::DAMAGE, this_takes_damage),
        gain_matched_damage_life,
        "Whenever this creature is dealt damage, you gain that much life.",
    ).damage_amount()
    .sums_per_subject()],
    ..oracle::METROPOLIS_REFORMER
};
