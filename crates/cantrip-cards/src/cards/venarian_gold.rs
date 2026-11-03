use cantrip_api::*;
use cantrip_mechanics::{attach, targets, triggers::{THIS_ENTERS, ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP}};
use cantrip_oracle as oracle;

/// The enchanted creature is determined on resolution, using the Aura's last known information if it left (CR 608.2h).
fn tap_and_add_sleep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = attach::tap_attached(g, item) else { return Ok(()) };
    let amount = g.obj(item.object).x as i32;
    g.add_counters_together(&[(creature, CounterKind::Sleep, amount)]);
    Ok(())
}

/// Sleep counters on the enchanted creature prevent its untap in its controller's untap step (CR 502.3).
fn sleep_prevents_untap(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    attach::attached_to_it(q, asker, id) && q.obj(id).counters(CounterKind::Sleep) > 0
}

/// Remove a counter from the creature enchanted as the ability resolves (CR 303.4m, 608.2h).
fn remove_sleep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        g.remove_counters(creature, CounterKind::Sleep, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tap_and_add_sleep,
            "When this Aura enters, tap enchanted creature and put X sleep counters on it."),
        triggered(ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP, remove_sleep,
            "At the beginning of the upkeep of enchanted creature's controller, remove a sleep counter from that creature."),
    ],
    untap_bans: &[sleep_prevents_untap],
    enchant: Some(targets::CREATURE),
    ..oracle::VENARIAN_GOLD
};
