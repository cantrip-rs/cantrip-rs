use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Creatures and Vehicles you control gain trample until end of turn. Put two +1/+1 counters on this creature." (CR 611.2c):
/// the creatures and Vehicles (an uncrewed one included, CR 301.7) its controller has as the ability resolves, and none that
/// enters later; the Vehicle type is read through the words of this card's text (CR 612.2).
fn creatures_and_vehicles_gain_trample_and_this_gets_two_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let vehicle = g.words(item.object).subtype(Subtype::Vehicle);
    effects::permanents_you_control_where_gain(g, item, &[Keyword::Trample], |g, id| g.chars(id).is(CardType::Creature) || g.chars(id).has_subtype(vehicle));
    g.end_sentence();
    effects::two_plus_one_counters_on_this(g, item)
}

/// "Exhaust abilities of other permanents you control cost {2} less to activate" (CR 601.2f, 602.2b, 702.177a): the exhaust
/// abilities (`ActivatedDef::exhaust`) of the permanents its controller controls but this one, generic mana only (CR 118.7a).
fn other_exhaust_abilities_cost_two_less(q: &dyn Query, source: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Ability(permanent, ability, _) = costed else { return CostDelta::NONE };
    let exhaust = q.activated_ability(ability).is_some_and(|d| d.exhaust);
    if exhaust && permanent != source && q.is_on_battlefield(permanent) && q.controller(permanent) == q.controller(source) {
        CostDelta::less(2)
    } else {
        CostDelta::NONE
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}{R}{G}"),
        creatures_and_vehicles_gain_trample_and_this_gets_two_counters,
        "Exhaust \u{2014} {4}{R}{G}: Creatures and Vehicles you control gain trample until end of turn. Put two +1/+1 counters on this creature. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    cost_changes: &[other_exhaust_abilities_cost_two_less],
    ..oracle::BOOM_SCHOLAR
};
