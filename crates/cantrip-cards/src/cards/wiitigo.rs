use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "This creature enters with six +1/+1 counters on it." (CR 614.1c).
fn enters_with_six(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] = entering.counters[CounterKind::PlusOne as usize].saturating_add(6);
    Ok(())
}

/// Check the controller's previous upkeep and the source's combat history at resolution (CR 608.2h, 509.3a, 509.3c).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let history = g.history();
    let in_upkeep = g.state().turn.step() == Step::Upkeep;
    let start = history.select(EventQuery::kind(EventMask::STEP_BEGAN).step(Step::Upkeep))
        .rev().filter(|r| r.active == item.controller).nth(usize::from(in_upkeep))
        .map_or(0, |r| r.serial as usize + 1);
    let blocked = history.all()[start..].iter().any(|r| match r.event {
        Event::Blocks { blocker, declared: true, .. } => blocker == item.object,
        Event::BecameBlocked { attacker } => attacker == item.object,
        _ => false,
    });
    if blocked {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    } else {
        g.remove_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_UPKEEP, upkeep,
        "At the beginning of your upkeep, put a +1/+1 counter on this creature if it has blocked or been blocked since your last upkeep. Otherwise, remove a +1/+1 counter from it.")],
    enters: Some(enters_with_six),
    ..oracle::WIITIGO
};
