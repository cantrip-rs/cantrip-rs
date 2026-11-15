use std::ops::ControlFlow;

use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::grant_delayed_ability, targets};
use cantrip_oracle as oracle;

/// The resolution destroyed a land `you` controlled (CR 701.8a, 701.19c): a land moved from the battlefield by destruction,
/// one that can't be regenerated included - by the spell itself, since a speculative resolution ends before state-based
/// actions (an animated land dealt lethal damage is destroyed by a rule, not the spell: the rulings). A land sacrificed, or
/// one not destroyed (indestructible, regenerated), is not. Whose it was and that it was a land are read as it last existed
/// on the battlefield (CR 608.2h).
fn destroyed_a_land_of(outcome: &Speculated<'_>, you: PlayerId) -> bool {
    outcome.records.iter().any(|r| matches!(r.event,
        Event::ZoneChange { old, reason: ZoneReason::Destroy | ZoneReason::DestroyNoRegen, .. }
            if outcome.game.obj(old).controller == you
                && outcome.game.state().last_known(old).is_some_and(|l| l.types.has(CardType::Land))))
}

/// "{T}: Counter target spell if it would destroy a land you control." (CR 701.6, 608.2): as this resolves, the spell is
/// resolved as it is now on a hidden copy of the game (`speculate_settled`), every way it could go - each answer to each
/// question it asks, each random outcome. It would destroy a land you control only if every way does: one that depends on a
/// choice (Cleansing, Lava Blister) or on chance is not countered (the rulings). A land destroyed stays destroyed, so a way
/// that has destroyed one already settles the rest of it.
fn counter_if_it_would_destroy_a_land_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let you = item.controller;
    let would = speculate_settled(g, Target::Object(spell), |o| destroyed_a_land_of(o, you),
        |o| if destroyed_a_land_of(o, you) { ControlFlow::Continue(()) } else { ControlFlow::Break(()) });
    if would == Speculation::Undecided {
        g.counter(spell);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::LAND),
    delayed: &[ability(TAP, counter_if_it_would_destroy_a_land_you_control, "{T}: Counter target spell if it would destroy a land you control.")
        .targets(&[targets::SPELL])],
    statics: &[static_ability(Layer::Ability, attached_object, grant_delayed_ability,
        "Enchanted land has \"{T}: Counter target spell if it would destroy a land you control.\"")],
    ..oracle::EQUINOX
};
