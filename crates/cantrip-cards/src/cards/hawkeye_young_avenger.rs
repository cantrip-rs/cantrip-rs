use cantrip_api::*;
use cantrip_mechanics::replacements::noncombat_damage_you_deal_to_an_opponent_or_their_permanent;
use cantrip_oracle as oracle;

/// "... instead it deals that much damage plus X, where X is Hawkeye's power" (CR 614.1, 120.3): X is his power as the damage is
/// about to be dealt, 0 for a negative power; the damage stays that of its source (CR 614.5: two Hawkeyes add both powers).
fn plus_hawkeyes_power(q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount += q.chars(asker.source).power.unwrap_or(0).max(0) as u16;
    }
}

/// Reach is a core keyword.
pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::Damage,
        noncombat_damage_you_deal_to_an_opponent_or_their_permanent,
        plus_hawkeyes_power,
        "it deals that much damage plus X instead, where X is Hawkeye's power",
    )],
    ..oracle::HAWKEYE_YOUNG_AVENGER
};
