use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Tap up to one target creature and put three stun counters on it. If you control that creature, draw two cards" (CR 701.26a,
/// 122.1d, 121.1): the draw is judged on the target it just stunned, which does nothing at all once it is no longer a legal target
/// (CR 608.2b).
fn tap_stun_and_draw_if_yours(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_and_stun::<3>(g, item)?;
    if targets::object(g, item, 0).is_some_and(|creature| g.controller(creature) == item.controller) {
        effects::draw(g, item, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        tap_stun_and_draw_if_yours,
        "When this creature enters, tap up to one target creature and put three stun counters on it. If you control that creature, draw two cards.",
    )
    .targets(&[up_to_one(targets::CREATURE.filter)])],
    ..oracle::DREAMDEW_ENTRANCER
};
