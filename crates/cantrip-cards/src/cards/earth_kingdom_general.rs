use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{earthbend, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("GainLife", "gain that much life?", "One life for each +1/+1 counter put  ·  only once each turn", "Gain life", "Don't gain life");

/// "You may gain that much life. Do this only once each turn." Asked only while its controller has not gained life with this
/// ability this turn, and spent only by a yes: declining leaves the next counters free to trigger it (CR 603.2h).
fn may_gain_that_much_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.history().done(item.object, item.ability, item.controller) || !ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    g.note_done(item.object, item.ability, item.controller);
    g.gain_life(item.controller, item.data[0] as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        earthbend::enters::<2>("When this creature enters, earthbend 2. (Target land you control becomes a 0/0 creature with haste that's still a land. Put two +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped.)"),
        triggered(
            when_asked(EventMask::COUNTERS, triggers::you_put_plus_one_counters_on_a_creature),
            may_gain_that_much_life,
            "Whenever you put one or more +1/+1 counters on a creature, you may gain that much life. Do this only once each turn.",
        ),
    ],
    delayed: &[earthbend::RETURN],
    ..oracle::EARTH_KINGDOM_GENERAL
};
