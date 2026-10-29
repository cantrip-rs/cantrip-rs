use cantrip_api::*;
use cantrip_mechanics::triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("PayLife", "pay {cost} to put that many +1/+1 counters on {subject}?", "", "Pay {cost}", "Don't pay");

/// "You may pay life equal to its power. If you do, put that many +1/+1 counters on it" (CR 119.4, 608.2h): the entering
/// creature's power is read as the ability resolves, the power it last had on the battlefield if it has left (the
/// rulings), and a negative power is 0 life (CR 107.1b). The life is paid even if the creature is gone, by a player with
/// the life to pay; the counters go on it only if it is still on the battlefield.
fn pay_life_for_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(entering) = item.subject else { return Ok(()) };
    let power = targets::power_or_last_known(g, entering).max(0);
    let cost = Cost { life: power as u32, ..Cost::NONE };
    if g.may_pay(item.controller, item.object, entering, cost, &ASK)? && g.is_on_battlefield(entering) {
        g.add_counters(entering, CounterKind::PlusOne, power);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        pay_life_for_counters,
        "Whenever another creature you control enters, you may pay life equal to its power. If you do, put that many +1/+1 counters on it.",
    )],
    ..oracle::MADAME_NULL_POWER_BROKER
};
