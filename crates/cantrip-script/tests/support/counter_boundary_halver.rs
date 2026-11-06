//! Test-only replacement: observe the semantic proposal before reducing it into the current physical domain.
use cantrip_engine::*;
use std::sync::atomic::{AtomicI32, Ordering};
pub static OBSERVED: AtomicI32 = AtomicI32::new(0);
fn applies(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(c) if c.counts.iter().any(|&n| n >= 32768))
}
fn halve(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        for count in &mut c.counts {
            if *count >= 32768 {
                OBSERVED.store(*count, Ordering::SeqCst);
                *count /= 2;
            }
        }
    }
}
pub static HALVER: CardDef = CardDef {
    name: "Probe Counter Halver", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Artifact]),
    replacements: &[replacement(ReplacementKind::Counters, applies, halve, "Halve the proposed counter amount.")],
    ..CardDef::BLANK
};
pub fn observed() -> i32 { OBSERVED.load(Ordering::SeqCst) }
