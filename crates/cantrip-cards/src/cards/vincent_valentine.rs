use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_DIES};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever a creature an opponent controls dies" (CR 700.4, 603.10a), looked back on: the creature as it last was on the battlefield,
/// under the controller it last had, an opponent of this ability's controller. The subject is the creature that died, so the ability
/// reads its power as it last existed (CR 608.2h); `triggers::CREATURE_AN_OPPONENT_CONTROLS_DIES` is about its controller instead.
fn creature_an_opponent_controls_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, from, to, .. } = event else { return None };
    if !from.is(ZoneKind::Battlefield) || !to.is(ZoneKind::Graveyard) || !q.chars(*old).is(CardType::Creature) { return None }
    if q.controller(*old) == me.controller { return None }
    Fired::about(Target::Object(*old))
}

/// "Put a number of +1/+1 counters on Vincent Valentine equal to that creature's power": the power the creature that died had as it
/// last existed on the battlefield (CR 608.2h, the rulings), none for a power of 0 or less (CR 107.1b). They go on Vincent only if it
/// is still on the battlefield (CR 400.7), which it is not when it died at the same time as the creature: putting them together as one
/// placement gives none to an object that has changed zones and none for an amount of none or fewer.
fn counters_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(died) = item.subject else { return Ok(()) };
    let power = targets::power_or_last_known(g, died);
    g.add_counters_together(&[(item.object, CounterKind::PlusOne, i64::from(power))]);
    Ok(())
}

const MAY_TRANSFORM: Question = Question::new("Transform", "transform {subject}?", "", "Transform", "Don't transform");

/// "You may transform it" (CR 701.27, 608.2d): asked as the ability resolves, and it transforms only if it is still the permanent the
/// ability is of and has not transformed since the ability was put onto the stack (CR 701.27f).
fn may_transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&MAY_TRANSFORM))? {
        g.transform_by(item.object, item)?;
    }
    Ok(())
}

/// Galian Beast: "When Galian Beast dies, return it to the battlefield tapped (front face up)." (CR 110.2a, 400.7, 712.14a): the card as
/// it is in the graveyard (each card of a merged permanent, together, CR 730.3c), front face up, as a new object under the control
/// of the ability's controller: the text does not say "under its owner's control" (CR 603.3a), so a Beast that died under a thief's
/// control comes back under the thief's. A token has ceased to exist, so nothing returns. The trigger looks back (CR 603.10a).
fn return_it_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<(ObjectId, PlayerId, bool)> = g.found_all_after_move(item.object).into_iter().map(|c| (c, item.controller, true)).collect();
    if !cards.is_empty() {
        g.put_all_onto_battlefield_with_counters(&cards, &[])?;
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, return_it_tapped, "When Galian Beast dies, return it to the battlefield tapped (front face up).")],
    ..oracle::GALIAN_BEAST
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, creature_an_opponent_controls_dies),
            counters_equal_to_its_power,
            "Whenever a creature an opponent controls dies, put a number of +1/+1 counters on Vincent Valentine equal to that creature's power.",
        ),
        triggered(THIS_ATTACKS, may_transform, "Whenever Vincent Valentine attacks, you may transform it."),
    ],
    back: Some(&BACK),
    ..oracle::VINCENT_VALENTINE
};
