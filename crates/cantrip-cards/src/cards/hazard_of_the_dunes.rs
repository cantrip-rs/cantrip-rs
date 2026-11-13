use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Reach and trample are the card's keywords. "Exhaust \u{2014} {6}{G}: Put three +1/+1 counters on this creature" (CR 702.177a).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{6}{G}"),
        effects::three_plus_one_counters_on_this,
        "Exhaust \u{2014} {6}{G}: Put three +1/+1 counters on this creature. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::HAZARD_OF_THE_DUNES
};
