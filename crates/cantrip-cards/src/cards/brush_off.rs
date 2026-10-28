use cantrip_api::*;
use cantrip_mechanics::{counter_target, targets};
use cantrip_oracle as oracle;

/// "This spell costs {1}{U} less to cast if it targets an instant or sorcery spell" (CR 601.2f): the target as the total cost
/// is determined, once it is announced. The reduction is {1}{U}, so the blue part comes off the blue in the cost first
/// (CR 601.2f, 118.7).
fn less_if_it_targets_an_instant_or_sorcery(q: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let one = targets.iter().filter_map(|t| t.object()).any(|spell| {
        let c = q.chars(spell);
        c.is(CardType::Instant) || c.is(CardType::Sorcery)
    });
    if one { CostDelta::less_mana(mana("{1}{U}")) } else { CostDelta::NONE }
}

/// "Counter target spell."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effect),
    own_cost_changes: &[less_if_it_targets_an_instant_or_sorcery],
    ..oracle::BRUSH_OFF
};
