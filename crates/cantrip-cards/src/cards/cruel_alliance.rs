use cantrip_api::*;
use cantrip_mechanics::{exile_target, targets, teamwork};
use cantrip_oracle as oracle;

/// "Target creature with mana value 3 or less", or, if the spell was cast using teamwork, "target creature" instead. The
/// teamwork cost is announced before the target is chosen (CR 601.2b-c, 702.194b), so the target is judged by it, as the spell
/// is cast and again as it resolves (CR 608.2b).
fn creature_with_mana_value_3_or_less_or_any_with_teamwork(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    if q.obj(asker.source).paid > 0 {
        (targets::CREATURE.filter)(q, asker, t)
    } else {
        targets::creature_with_mana_value_at_most::<3>(q, asker, t)
    }
}

/// "Exile target creature with mana value 3 or less. If this spell was cast using teamwork, instead exile target creature and
/// you gain 3 life." (CR 701.13, 702.194b, 608.2b): one target either way, and the life is gained only if the spell resolves,
/// so a target that became illegal exiles nothing and gains nothing. A copy of a spell cast using teamwork was cast that way
/// too (CR 707.10).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if exile_target(g, item, 0) && teamwork::used(item) {
        g.end_sentence();
        g.gain_life(item.controller, 3);
    }
    Ok(())
}

/// Teamwork 2 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 2 or
/// more): an optional cost announced before the target, which is chosen after it.
pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_with_mana_value_3_or_less_or_any_with_teamwork)], effect),
    late_targets: true,
    optional_cost: Some(teamwork::optional(2)),
    labels: &["Cast without teamwork", "Teamwork 2: Tap any number of creatures you control with total power 2 or more"],
    ..oracle::CRUEL_ALLIANCE
};
