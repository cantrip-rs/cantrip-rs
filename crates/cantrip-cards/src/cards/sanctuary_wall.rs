use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Tap target creature. You may put a stun counter on it. If you do, put a stun counter on this creature." (CR 701.26a, 122.1d,
/// 608.2c): the creature is tapped first, tapped already or not. The controller then chooses whether to put the counter; "if you
/// do" is judged on the counter actually being put on it (a replacement may keep it off, CR 614.1a), and only then does this
/// creature get its own. A target that is no longer legal as the ability resolves makes none of it happen (CR 608.2b).
fn tap_then_may_stun_both(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    if !effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        return Ok(());
    }
    let before = g.state().obj(creature).counters(CounterKind::Stun);
    g.add_counters(creature, CounterKind::Stun, 1);
    if g.state().obj(creature).counters(CounterKind::Stun) > before {
        g.add_counters(item.object, CounterKind::Stun, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{W}").and(TAP),
        tap_then_may_stun_both,
        "{2}{W}, {T}: Tap target creature. You may put a stun counter on it. If you do, put a stun counter on this creature.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::SANCTUARY_WALL
};
