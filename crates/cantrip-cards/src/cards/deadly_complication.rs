use cantrip_api::*;
use cantrip_mechanics::effects::{self, ask_may};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Unsuspect", "have the creature become no longer suspected?", "Chosen as Deadly Complication resolves", "No longer suspected", "Stay suspected");

/// "Target suspected creature you control" (CR 115.1, 701.60b): a creature its caster controls that has the suspected designation.
fn suspected_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.is_suspected(id))
}

/// "Put a +1/+1 counter on target suspected creature you control. You may have it become no longer suspected." The counter goes on first;
/// the "may" is asked as the spell resolves (CR 608.2d) and a yes ends the designation (CR 701.60a), with its menace and "can't block".
fn counter_and_may_unsuspect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::plus_one_counter_on_target(g, item)?;
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.unsuspect(creature);
    }
    Ok(())
}

/// "Choose one or both" (CR 700.2): each mode has its own target (CR 700.2c), which may be the same creature, and the modes are carried out in
/// the order printed (CR 608.2c).
static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target creature.").targets(&[targets::CREATURE]),
    mode(counter_and_may_unsuspect, "Put a +1/+1 counter on target suspected creature you control. You may have it become no longer suspected.")
        .targets(&[target(suspected_creature_you_control)]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::DEADLY_COMPLICATION
};
