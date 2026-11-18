use cantrip_api::*;
use cantrip_mechanics::collect_evidence::may_collect_evidence;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, you may collect evidence 3. When you do, put a +1/+1 counter on target creature you control."
/// (CR 701.59a, 603.12): the reflexive ability goes on the stack only if evidence was collected, and its target is chosen then.
fn may_collect_evidence_then_a_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if may_collect_evidence::<3>(g, item)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        may_collect_evidence_then_a_counter,
        "Whenever this creature attacks, you may collect evidence 3. When you do, put a +1/+1 counter on target creature you control.",
    )],
    delayed: &[delayed_trigger(effects::plus_one_counter_on_target, "When you do, put a +1/+1 counter on target creature you control.").targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::SAMPLE_COLLECTOR
};
