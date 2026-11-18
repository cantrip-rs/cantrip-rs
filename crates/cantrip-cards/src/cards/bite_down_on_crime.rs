use cantrip_api::*;
use cantrip_mechanics::{collect_evidence, fight, targets};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, you may collect evidence 6" (CR 701.59a, 601.2b): announced like any optional
/// additional cost; the spell has `paid` set when it was paid.
const LABELS: &[&str] = &["Cast without collecting evidence", "Collect evidence 6"];

/// "This spell costs {2} less to cast if evidence was collected" (CR 601.2f, 118.7a): judged as the total cost is determined,
/// after the optional cost is announced, and generic mana only; the spell reads whether the evidence was collected as it
/// will on the stack.
fn less_if_evidence_was_collected(q: &dyn Query, source: ObjectId, _: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if q.obj(source).paid > 0 { CostDelta::less(2) } else { CostDelta::NONE }
}

/// "Target creature you control gets +2/+0 until end of turn. It deals damage equal to its power to target creature you don't
/// control." (CR 608.2b-c): the pump is done first, so the damage is the power it has now. If the creature you control is still
/// a legal target but the other is not, it still gets the pump and no damage is dealt; if the creature you control is illegal,
/// nothing happens.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::pump_first_target_then_it_bites_second(g, item, 2, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL], effect),
    optional_cost: Some(collect_evidence::optional(6)),
    labels: LABELS,
    own_cost_changes: &[less_if_evidence_was_collected],
    ..oracle::BITE_DOWN_ON_CRIME
};
