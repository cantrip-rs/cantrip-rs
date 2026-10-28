use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("TapSpiderMan", "tap Spider-Man?", "If you do, another target nonattacking creature you control gains indestructible until end of turn", "Tap him", "Don't tap him");

/// "When Spider-Man enters, you may tap him. When you do (CR 603.12), ...": tapping him is what makes the reflexive ability
/// trigger, so an already tapped Spider-Man, tapped by something in response, does not satisfy it.
fn may_tap_him(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    if !g.is_on_battlefield(item.object) {
        return Ok(());
    }
    if g.obj(item.object).tapped {
        return Ok(());
    }
    g.tap(item.object);
    let this = g.card_id(&CARD);
    g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

/// "Another target nonattacking creature you control" (CR 115.1): on the battlefield under your control, other than Spider-Man,
/// and not attacking.
fn other_nonattacking_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::other_creature_you_control_filter(q, asker, t) && t.object().is_some_and(|id| !q.is_attacking(id))
}

/// "It gains indestructible until end of turn." (CR 611.2a.)
fn gains_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Indestructible);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, may_tap_him, "No One Dies! \u{2014} When Spider-Man enters, you may tap him. When you do, another target nonattacking creature you control gains indestructible until end of turn.")],
    delayed: &[delayed_trigger(gains_indestructible, "When you do, another target nonattacking creature you control gains indestructible until end of turn.")
        .targets(&[target(other_nonattacking_creature_you_control)])],
    ..oracle::SPIDER_MAN_TO_THE_RESCUE
};
