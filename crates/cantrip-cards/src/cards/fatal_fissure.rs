use cantrip_api::*;
use cantrip_mechanics::{earthbend, targets, triggers};
use cantrip_oracle as oracle;

/// "Choose target creature. When that creature dies this turn, you earthbend 4." (CR 603.7c): the delayed triggered ability
/// (ability 1, after earthbending's own `RETURN`) is fixed on that creature, so it triggers only when that very object dies,
/// whatever kills it; its land is chosen as it triggers (CR 603.3d), not when the spell is cast. The spell does nothing to
/// the creature itself.
fn choose_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.delayed_trigger(DelayedWhen::ThisTurn, creature, item.controller, AbilityRef::delayed(item.ability.card, 1), Target::None);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], choose_creature),
    delayed: &[
        earthbend::RETURN,
        triggered(triggers::THIS_DIES, earthbend::effect::<4>, "When that creature dies this turn, you earthbend 4.").targets(&[earthbend::TARGET]),
    ],
    ..oracle::FATAL_FISSURE
};
