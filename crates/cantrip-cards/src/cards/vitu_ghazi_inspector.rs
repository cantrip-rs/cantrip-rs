use cantrip_api::*;
use cantrip_mechanics::{collect_evidence, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, you may collect evidence 6" (CR 701.59a, 601.2b): announced like any optional
/// additional cost, so it cannot be chosen without cards in the graveyard with total mana value 6 or more.
const LABELS: &[&str] = &["Cast without collecting evidence", "Collect evidence 6"];

/// The intervening "if evidence was collected" (CR 603.4): the permanent keeps how it was cast, so this reads the same as the ability
/// triggers and as it resolves.
fn evidence_was_collected(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).paid > 0
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(collect_evidence::optional(6)),
    labels: LABELS,
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::plus_one_counter_on_target_and_you_gain_life::<2>,
        "When this creature enters, if evidence was collected, put a +1/+1 counter on target creature and you gain 2 life.",
    )
    .targets(&[targets::CREATURE])
    .condition(evidence_was_collected)],
    ..oracle::VITU_GHAZI_INSPECTOR
};
