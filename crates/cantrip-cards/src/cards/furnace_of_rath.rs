use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "If a source would deal damage to a permanent or player" (CR 614.1a, 120.3): any source's damage, combat or not, to anything,
/// once it is divided among its recipients (the rulings: divided damage and trample's assignment are doubled after). Damage
/// already prevented to nothing is no damage (CR 615.6), and nothing is left to double.
fn source_would_deal_damage(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.amount > 0)
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(ReplacementKind::Damage, source_would_deal_damage, replacements::double_that_damage, "deals double that damage instead")],
    ..oracle::FURNACE_OF_RATH
};
