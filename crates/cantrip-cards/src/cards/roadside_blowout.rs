use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if it targets a permanent with mana value 1" (CR 601.2c, 601.2f, 202.3): judged once the
/// target is announced, as the total cost is determined, by the mana value the target has then; generic mana only (CR 118.7a).
fn less_for_a_mana_value_one_target(q: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let one = targets.iter().filter_map(|t| t.object()).any(|permanent| q.mana_value(permanent) == 1);
    if one { CostDelta::less(2) } else { CostDelta::NONE }
}

/// "Return target creature or Vehicle an opponent controls to its owner's hand. Draw a card." (CR 400.3, 121.1): a spell whose
/// only target has become illegal does nothing at all (CR 608.2b), so there is no card then; otherwise it is drawn whether or not
/// the permanent moved.
fn return_it_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.return_to_hand(&[permanent]);
    g.end_sentence();
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_VEHICLE_AN_OPPONENT_CONTROLS], return_it_then_draw),
    own_cost_changes: &[less_for_a_mana_value_one_target],
    ..oracle::ROADSIDE_BLOWOUT
};
