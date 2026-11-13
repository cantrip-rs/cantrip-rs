use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Exhaust \u{2014} {4}{U}: Put two +1/+1 counters on this creature." (CR 702.177a, 122.1a): activate only once, ever, for this permanent.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}{U}"),
        effects::two_plus_one_counters_on_this,
        "Exhaust \u{2014} {4}{U}: Put two +1/+1 counters on this creature. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::SKYSTREAK_ENGINEER
};
