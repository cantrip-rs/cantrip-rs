use cantrip_api::*;
use cantrip_mechanics::manifest_dread::manifest_dread_object;
use cantrip_oracle as oracle;

/// "Manifest dread X times, then put X +1/+1 counters on each of those creatures" (CR 701.62a, 608.2c): the counters go on every
/// face-down permanent the X manifests made, together in one action, after all of them (the second instruction). One that has left
/// the battlefield by then (CR 400.7) gets none.
fn manifest_dread_x_times_then_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut manifested = Vec::new();
    for _ in 0..item.x {
        manifested.extend(manifest_dread_object(g, item.controller, item.object)?);
    }
    g.end_sentence();
    let puts: Vec<(ObjectId, CounterKind, i32)> = manifested.into_iter().map(|creature| (creature, CounterKind::PlusOne, item.x as i32)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], manifest_dread_x_times_then_counters), ..oracle::VALGAVOTHS_ONSLAUGHT };
