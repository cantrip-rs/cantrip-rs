use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{delayed, effects, haste_until_end_of_turn, targets, tokens};
use cantrip_oracle as oracle;

/// "Create a 2/2 red Mutant creature token. It gains haste until end of turn. Destroy it at the beginning of the next end
/// step" (CR 603.7, 611.2): the delayed ability is about exactly the tokens made (CR 603.7c, 400.7).
fn create_mutant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let created = tokens::create(g, item.controller, &tokens::MUTANT, 1)?;
    instruction(g, |g| {
        for &token in &created {
            haste_until_end_of_turn(g, item.object, token);
        }
    });
    delayed::at_next_end_step_about(g, item, 0, &created);
    Ok(())
}

/// "Destroy it": the tokens the delayed ability carries that are still those objects, so still on the battlefield (CR 603.7c, 400.7).
/// An indestructible one stays, and the ability does not trigger again (CR 603.7).
fn destroy_created(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let created: Vec<ObjectId> = g.event_objects(item).iter().copied().filter(|&id| is_live(g, id)).collect();
    g.destroy_all(&created);
    Ok(())
}

fn attacking_creature_token(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::ATTACKING_CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).token)
}

/// "Target attacking creature token gains indestructible until end of turn" (CR 611.2, 702.12).
fn protect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(token) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_indestructible_until_end_of_turn(g, item.object, token);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            create_mutant,
            "At the beginning of combat on your turn, create a 2/2 red Mutant creature token. It gains haste until end of turn. Destroy it at the beginning of the next end step.",
        ),
        ability(cost("{1}{W}"), protect, "{1}{W}: Target attacking creature token gains indestructible until end of turn.").targets(&[target(attacking_creature_token)]),
    ],
    delayed: &[delayed_trigger(destroy_created, "Destroy it.")],
    ..oracle::OLD_HOB_ALLEYCAT_BLUES
};

pub static EXTRA: &[&CardDef] = &[&tokens::MUTANT];
