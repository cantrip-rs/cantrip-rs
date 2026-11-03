use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Choose target creature an opponent controls. Tap it, then you may sacrifice an artifact or creature. If you do, put a stun
/// counter on the chosen creature" (CR 701.26a, 701.21a, 608.2c-d, 122.1d): the creature is tapped first (an ability with a single
/// target that is no longer legal does nothing, CR 608.2b); the sacrifice is chosen as the ability resolves, from any artifact or
/// creature its controller has, this Villagers included; the stun counter follows only if one was sacrificed.
fn tap_then_sacrifice_to_stun(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(chosen) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(chosen);
    g.end_sentence();
    let sacrifice = select::cards(select::up_to(1)).with(select::ARTIFACT_OR_CREATURE);
    if !effects::may_sacrifice(g, item.controller, item.object, sacrifice)?.is_empty() {
        g.add_counters(chosen, CounterKind::Stun, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        tap_then_sacrifice_to_stun,
        "Whenever this creature attacks, choose target creature an opponent controls. Tap it, then you may sacrifice an artifact or creature. If you do, put a stun counter on the chosen creature.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::VENGEFUL_VILLAGERS
};
