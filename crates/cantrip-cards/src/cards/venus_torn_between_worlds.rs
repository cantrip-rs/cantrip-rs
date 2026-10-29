use cantrip_api::*;
use cantrip_mechanics::triggers::{self, creature_you_control_deals_combat_damage_to_player};
use cantrip_oracle as oracle;

const DRAW: Question = Question::new("DrawCard", "pay {cost} to draw a card?", "", "Pay {cost}", "Don't pay");

/// "Put that many +1/+1 counters on her" (CR 120.2, 122.1): the damage dealt at once, the trigger's `data[0]`. Venus must have
/// survived it (the reminder text, CR 400.7): the ability's source that has left the battlefield is another object and gets none.
fn counters_for_the_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, item.data[0] as i16);
    Ok(())
}

/// "Whenever a creature you control with a counter on it deals combat damage to a player" (CR 510.2, 603.2, 122.1): a counter
/// of any kind, on the creature as it dealt the damage.
fn creature_with_a_counter_deals_combat_damage_to_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, .. } if q.state().obj(*from).has_counters() => {
            creature_you_control_deals_combat_damage_to_player(q, me, event)
        }
        _ => None,
    }
}

/// "You may pay {U}. If you do, draw a card." (CR 118.12, 608.2d): the controller of the ability pays and draws.
fn pay_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{U}"), &DRAW)? {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_DEALT_DAMAGE,
            counters_for_the_damage,
            "Whenever Venus is dealt damage, put that many +1/+1 counters on her. (She must survive the damage to get the counters.)",
        )
        .sums_per_subject(),
        triggered(
            when_asked(EventMask::DAMAGE, creature_with_a_counter_deals_combat_damage_to_player),
            pay_to_draw,
            "Whenever a creature you control with a counter on it deals combat damage to a player, you may pay {U}. If you do, draw a card.",
        ),
    ],
    ..oracle::VENUS_TORN_BETWEEN_WORLDS
};
