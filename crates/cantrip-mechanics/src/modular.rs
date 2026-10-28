//! Modular (CR 702.43a), a static ability and a triggered ability: "Modular N" means "This permanent enters with N +1/+1
//! counters on it" and "When this permanent is put into a graveyard from the battlefield, you may put a +1/+1 counter on
//! target artifact creature for each +1/+1 counter on this permanent." Each instance works separately (CR 702.43b). Arcbound
//! Wanderer prints "Modular-Sunburst" (CR 702.44c): the number of counters is `sunburst::enters`'s, the trigger the same.
//!
//! A card with modular lists `TRIGGER` among its `abilities`, and enters with its counters through `enters`: `sunburst::enters` for
//! modular-sunburst; a plain "Modular N" enters with N +1/+1 counters, which no card in the engine prints yet. The counters moved are
//! the ones the permanent last had (CR 603.10a, 608.2h).

use crate::effects::ask_may;
use crate::targets;
use crate::triggers::THIS_DIES;
use cantrip_api::*;

/// "Target artifact creature" (CR 115.1): on the battlefield, an artifact and a creature, whoever controls it.
fn artifact_creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::on_battlefield(q, id) && q.chars(id).is(CardType::Artifact) && q.chars(id).is(CardType::Creature))
}

const ASK: Question = Question::new(
    "Modular",
    "put its +1/+1 counters on the artifact creature?",
    "Modular: one +1/+1 counter for each it had",
    "Put the counters",
    "Don't",
);

/// A +1/+1 counter on the target for each one the permanent had as it last was on the battlefield (CR 603.10a).
fn move_the_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let counters = g.obj(item.object).counters(CounterKind::PlusOne);
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.add_counters(creature, CounterKind::PlusOne, counters);
    }
    Ok(())
}

pub const TRIGGER: AbilityDef = triggered(
    THIS_DIES,
    move_the_counters,
    "Modular (When this creature dies, you may put its +1/+1 counters on target artifact creature.)",
)
.targets(&[target(artifact_creature)]);
