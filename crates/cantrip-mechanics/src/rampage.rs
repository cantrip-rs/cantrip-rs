//! Rampage (CR 702.23): one trigger when this creature becomes blocked, with its bonus calculated at resolution.
//! Chromium, Craw Giant, Marhault Elsdragon and Wolverine Pack print it. Multiple instances trigger separately.

use cantrip_api::*;

fn this_becomes_blocked(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::BecameBlocked { attacker } if *attacker == source => Fired::PLAIN,
        _ => None,
    }
}

/// Count the creatures still blocking the source when the ability resolves (CR 702.23b), not as it triggers.
fn grow<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let blockers = g.state().blocks.iter().filter(|b| b.attacker == item.object).count();
    let bonus = if blockers > 1 { (blockers - 1) as i32 * i32::from(N) } else { 0 };
    if bonus != 0 { crate::pump(g, item.object, item.object, bonus, bonus); }
    Ok(())
}

/// "Rampage N" (CR 702.23a-c): list each printed instance separately among the creature's abilities.
pub const fn trigger<const N: u8>(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::BECAME_BLOCKED, this_becomes_blocked), grow::<N>, text).keyword_family(KeywordFamily::Rampage)
}
