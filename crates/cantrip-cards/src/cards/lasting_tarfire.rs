use cantrip_api::*;
use cantrip_mechanics::effects::damage_each_opponent;
use cantrip_mechanics::triggers::EACH_END_STEP;
use cantrip_oracle as oracle;

/// "If you put a counter on a creature this turn" (CR 603.4, 122.6): a counter this ability's controller put this turn on something
/// that was a creature as the counter was put (the event's card types, so a creature that has since stopped being one still counts
/// and a permanent that has since become one does not), checked as the end step begins and again as the ability resolves.
fn you_put_a_counter_on_a_creature_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history().select(EventQuery::kind(EventMask::COUNTERS).this_turn()).any(|r| {
        matches!(r.event, Event::CountersPut { by, types, .. } if by == asker.controller && types.has(CardType::Creature))
    })
}

/// "This enchantment deals 2 damage to each opponent."
fn two_damage_to_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_each_opponent(g, item, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EACH_END_STEP,
        two_damage_to_each_opponent,
        "At the beginning of each end step, if you put a counter on a creature this turn, this enchantment deals 2 damage to each opponent.",
    )
    .condition(you_put_a_counter_on_a_creature_this_turn)],
    ..oracle::LASTING_TARFIRE
};
