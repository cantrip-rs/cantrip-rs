use cantrip_api::*;
use cantrip_mechanics::power_up;
use cantrip_oracle as oracle;

/// "Power-up abilities of other creatures you control cost {3} less to activate" (CR 601.2f, 118.7, 702.193): asked of an
/// activated ability that is a power-up ability (not of a special action, CR 116.1) of another creature its controller controls
/// on the battlefield, activated by that controller.
fn others_power_up_costs_three_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Ability(permanent, ability, _) = costed else { return CostDelta::NONE };
    let yours = q.controller(source);
    if permanent != source
        && player == yours
        && q.controller(permanent) == yours
        && q.is_on_battlefield(permanent)
        && q.chars(permanent).is(CardType::Creature)
        && q.activated_ability(ability).is_some_and(|d| d.power_up)
    {
        CostDelta::less(3)
    } else {
        CostDelta::NONE
    }
}

/// "Put five +1/+1 counters on Hulk." (CR 122.1)
fn five_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 5);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{6}{R}{G}"),
        five_counters,
        "Power-up \u{2014} {6}{R}{G}: Put five +1/+1 counters on Hulk. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
    )],
    cost_changes: &[others_power_up_costs_three_less],
    ..oracle::HULK_GAMMA_GOLIATH
};
