use cantrip_api::*;
use cantrip_mechanics::history::convoking_creatures_on_the_battlefield;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, put a +1/+1 counter on each creature that convoked it." The creatures that convoked the
/// spell this permanent was (CR 702.51c, 608.3) that are still the same objects on the battlefield (CR 400.7) and still
/// creatures (CR 109.2) get one each, put at once; a convoker that left or is no longer a creature gets none, and a creature
/// that did not convoke gets none.
fn counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let puts: Vec<(ObjectId, CounterKind, i64)> = convoking_creatures_on_the_battlefield(g, item.object)
        .into_iter()
        .map(|creature| (creature, CounterKind::PlusOne, 1))
        .collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, counters, "When this creature enters, put a +1/+1 counter on each creature that convoked it.")],
    ..oracle::VENERATED_LOXODON
};
