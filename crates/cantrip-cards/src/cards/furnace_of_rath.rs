use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If a source would deal damage to a permanent or player" (CR 614.1a, 120.3): any source's damage, combat or not, to anything,
/// once it is divided among its recipients (the rulings: divided damage and trample's assignment are doubled after). Damage
/// already prevented to nothing is no damage (CR 615.6), and nothing is left to double.
fn source_would_deal_damage(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.amount > 0)
}

/// "... it deals double that damage to that permanent or player instead": still dealt by the source, not by Furnace of Rath (the
/// ruling). Two Furnaces make it four times as much (the ruling, CR 614.5), and the player or controller of the permanent it is
/// dealt to orders it with prevention (CR 616.1, the Mending Hands ruling).
fn double_that_damage(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount = d.amount.checked_mul(2).expect("damage expression exceeds i64 semantic domain");
    }
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(ReplacementKind::Damage, source_would_deal_damage, double_that_damage, "deals double that damage instead")],
    ..oracle::FURNACE_OF_RATH
};
