use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// The enchanted creature's controller at resolution, or its last-known controller, may pay {3} (CR 608.2h, 118.12).
fn damage_unless_paid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    let player = g.controller(creature);
    if !g.may_pay(player, item.object, item.object, cost("{3}"), &PAY_QUESTION)? {
        g.damage(item.object, Target::Player(player), 3);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ATTACHED_PERMANENT_BECOMES_TAPPED,
        damage_unless_paid,
        "Whenever enchanted creature becomes tapped, this Aura deals 3 damage to that creature's controller unless that player pays {3}.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::SEIZURES
};
