use cantrip_api::*;
use cantrip_mechanics::{earthbend, fight, targets};
use cantrip_oracle as oracle;

/// "Earthbend 2. When you do, up to one target creature you control fights target creature an opponent controls." (CR 603.12,
/// 701.66a, 608.2b): the spell does not resolve if the land is no longer a legal target, so the reflexive ability is made only
/// after the land was earthbent; its own targets are chosen as it is put on the stack, the creature you control among them
/// the land itself when it has become one.
fn earthbend_then_fight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    earthbend::effect::<2>(g, item)?;
    g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(item.ability.card, 1), Target::None);
    Ok(())
}

/// "Up to one target creature you control fights target creature an opponent controls" (CR 701.14a-b): neither fights without both targets still legal as it resolves.
fn fights(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::targets_fight(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[earthbend::TARGET], earthbend_then_fight),
    delayed: &[
        earthbend::RETURN,
        delayed_trigger(
            fights,
            "When you do, up to one target creature you control fights target creature an opponent controls.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.optional(), targets::CREATURE_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::EARTH_RUMBLE
};
