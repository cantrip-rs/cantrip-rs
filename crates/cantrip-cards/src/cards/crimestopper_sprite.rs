use cantrip_api::*;
use cantrip_mechanics::{collect_evidence, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, you may collect evidence 6" (CR 701.59a, 601.2b): announced like any optional
/// additional cost; the permanent keeps `paid` set when it was paid, for the enters trigger to read.
const LABELS: &[&str] = &["Cast without collecting evidence", "Collect evidence 6"];

/// "Tap target creature. If evidence was collected, put a stun counter on it." (CR 701.26a, 122.1d, 608.2h): the counter goes on
/// whether or not the tap did anything; nothing if the target is no longer legal (CR 608.2b). Whether evidence was collected is how the
/// permanent was cast, as it last existed if it has left the battlefield.
fn tap_and_stun_if_evidence(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.obj(item.object).paid > 0 {
        effects::tap_and_stun::<1>(g, item)
    } else {
        effects::tap_targets(g, item)
    }
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(collect_evidence::optional(6)),
    labels: LABELS,
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        tap_and_stun_if_evidence,
        "When this creature enters, tap target creature. If evidence was collected, put a stun counter on it.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::CRIMESTOPPER_SPRITE
};
