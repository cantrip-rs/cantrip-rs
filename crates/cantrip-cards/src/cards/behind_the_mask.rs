use cantrip_api::*;
use cantrip_mechanics::{collect_evidence, effects, targets};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, you may collect evidence 6" (CR 701.59a, 601.2b): announced like any optional
/// additional cost; the spell has `paid` set when it was paid.
const LABELS: &[&str] = &["Cast without collecting evidence", "Collect evidence 6"];

/// "Until end of turn, target artifact or creature becomes an artifact creature with base power and toughness 4/3. If evidence was
/// collected, it has base power and toughness 1/1 until end of turn instead." (CR 608.2c, 613.4b): one setting effect, with the
/// numbers the evidence decides, so counters and pumps count on top either way.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (power, toughness) = if item.paid > 0 { (1, 1) } else { (4, 3) };
    effects::first_target_becomes_artifact_creature_with_base_pt(g, item, power, toughness);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE], effect),
    optional_cost: Some(collect_evidence::optional(6)),
    labels: LABELS,
    ..oracle::BEHIND_THE_MASK
};
