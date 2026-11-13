use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{R}"),
        effects::two_plus_one_counters_on_this,
        "Exhaust \u{2014} {3}{R}: Put two +1/+1 counters on this creature. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::PROWCATCHER_SPECIALIST
};
