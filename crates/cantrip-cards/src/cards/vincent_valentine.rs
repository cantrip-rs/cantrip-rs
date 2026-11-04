use cantrip_api::*;
use cantrip_mechanics::triggers::{CREATURE_AN_OPPONENT_CONTROLS_DIES, THIS_ATTACKS, THIS_DIES};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Put a number of +1/+1 counters on Vincent Valentine equal to that creature's power": the power the creature that died had as it
/// last existed on the battlefield (CR 608.2h, the rulings), none for a power of 0 or less (CR 107.1b). They go on Vincent only if it
/// is still on the battlefield (CR 400.7), which it is not when it died at the same time as the creature: putting them together as one
/// placement gives none to an object that has changed zones and none for an amount of none or fewer.
fn counters_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(died) = item.subject else { return Ok(()) };
    let power = targets::power_or_last_known(g, died);
    g.add_counters_together(&[(item.object, CounterKind::PlusOne, power)]);
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

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, effects::return_it_tapped, "When Galian Beast dies, return it to the battlefield tapped (front face up).")],
    ..oracle::GALIAN_BEAST
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            CREATURE_AN_OPPONENT_CONTROLS_DIES,
            counters_equal_to_its_power,
            "Whenever a creature an opponent controls dies, put a number of +1/+1 counters on Vincent Valentine equal to that creature's power.",
        ),
        triggered(THIS_ATTACKS, may_transform, "Whenever Vincent Valentine attacks, you may transform it."),
    ],
    back: Some(&BACK),
    ..oracle::VINCENT_VALENTINE
};
