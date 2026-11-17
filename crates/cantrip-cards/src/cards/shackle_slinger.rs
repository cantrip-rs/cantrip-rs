use cantrip_api::*;
use cantrip_mechanics::{flurry, targets};
use cantrip_oracle as oracle;

/// "Choose target creature an opponent controls. If it's tapped, put a stun counter on it. Otherwise, tap it." (CR 701.26a, 122.1d,
/// 608.2b-c): tapped or not is read as the ability resolves, once; a target that is no longer legal does nothing.
fn stun_if_tapped_otherwise_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(chosen) = targets::object(g, item, 0) else { return Ok(()) };
    if g.obj(chosen).tapped {
        g.add_counters(chosen, CounterKind::Stun, 1);
    } else {
        g.tap(chosen);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger(
        "Whenever you cast your second spell each turn, choose target creature an opponent controls. If it's tapped, put a stun counter on it. Otherwise, tap it. (If a permanent with a stun counter would become untapped, remove one from it instead.)",
        stun_if_tapped_otherwise_tap,
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::SHACKLE_SLINGER
};
