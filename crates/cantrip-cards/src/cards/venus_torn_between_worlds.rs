use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{self, creature_you_control_with_a_counter_deals_combat_damage_to_player};
use cantrip_oracle as oracle;

/// "Put that many +1/+1 counters on her" (CR 120.2, 122.1): the damage dealt at once, the trigger's `event_amount`. Venus must have
/// survived it (the reminder text, CR 400.7): the ability's source that has left the battlefield is another object and gets none.
fn counters_for_the_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, item.matched_damage_amount() as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_TAKES_DAMAGE,
            counters_for_the_damage,
            "Whenever Venus is dealt damage, put that many +1/+1 counters on her. (She must survive the damage to get the counters.)",
        ).damage_amount()
        .sums_per_subject(),
        triggered(
            when_asked(EventMask::DAMAGE, creature_you_control_with_a_counter_deals_combat_damage_to_player),
            |g, item| effects::may_pay_to_draw(g, item, cost("{U}")),
            "Whenever a creature you control with a counter on it deals combat damage to a player, you may pay {U}. If you do, draw a card.",
        ),
    ],
    ..oracle::VENUS_TORN_BETWEEN_WORLDS
};
