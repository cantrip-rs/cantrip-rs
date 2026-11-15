use cantrip_api::*;
use cantrip_mechanics::{attach, gift, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this Aura enters, tap enchanted creature. If the gift wasn't promised, put three stun counters on it." (CR 701.26a,
/// 122.1d, 702.174): the creature it is attached to as the ability resolves is tapped, then, when the Aura was cast without the
/// gift, gets the counters (whether or not it was untapped). An Aura that has left the battlefield by then still names the
/// creature it was attached to as it left (CR 608.2h), which is tapped and stunned all the same.
fn tap_enchanted_creature_and_stun_it_unless_the_gift_was_promised(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = attach::tap_attached(g, item) else { return Ok(()) };
    let promised = { let q: &dyn Query = g; gift::paid(q, &Asker::new(item.object, item.controller)) };
    if !promised {
        g.add_counters(creature, CounterKind::Stun, 3);
    }
    Ok(())
}

/// "You control enchanted creature." (CR 613.1b, layer 2): the Aura's controller controls the creature it is attached to.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        gift::trigger("When this permanent enters, if its gift cost was paid, the chosen opponent draws a card."),
        triggered(THIS_ENTERS, tap_enchanted_creature_and_stun_it_unless_the_gift_was_promised, "When this Aura enters, tap enchanted creature. If the gift wasn't promised, put three stun counters on it."),
    ],
    statics: &[control_changing(attach::attached_object, "You control enchanted creature.")],
    gift: Some(gift::CARD),
    ..oracle::KITNAP
};
