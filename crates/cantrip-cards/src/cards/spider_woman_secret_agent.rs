use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Tap target creature an opponent controls. That creature can't become untapped for as long as you control Spider-Woman"
/// (CR 701.26a, 101.2, 611.2b). A resolved effect on that creature, so it holds whatever abilities the creature loses. Nothing
/// if the target is no longer legal (CR 608.2b), and no rule against untapping if she has left before it resolves.
fn tap_and_hold(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.add_effect(item.object, creature, EffectKind::CantBecomeUntapped, Duration::WhileSourceControlled);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        tap_and_hold,
        "When Spider-Woman enters, tap target creature an opponent controls. That creature can't become untapped for as long as you control Spider-Woman.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::SPIDER_WOMAN_SECRET_AGENT
};
