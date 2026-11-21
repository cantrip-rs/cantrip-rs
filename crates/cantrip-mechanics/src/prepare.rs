//! Prepare (CR 722): a permanent with a prepare spell that is prepared lets its controller cast a copy of that spell
//! (`CardDef::prepare`); `Game::become_prepared` makes it prepared later, and this is "enters prepared".
//! Carnivorous Cultivator, Emeritus of Abundance, Emeritus of Ideation, Skycoach Conductor, Pompous Battlemage and Heartwood Crafter print it.

use cantrip_api::*;

/// "This creature enters prepared" (CR 614.1d, 722.3a): a replacement effect of what it is as it enters, for `CardDef::enters`.
pub fn enters_prepared(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.prepared = true;
    Ok(())
}

/// "This creature becomes prepared" during resolution (CR 722.3a). Bloodline Recollector, Emeritus of Abundance,
/// Paradox Shaper and Stingerquill Voxmancer use it.
pub fn become_prepared(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.become_prepared(item.object);
    Ok(())
}

/// "Target creature becomes prepared" (CR 722.3a): only a permanent with a prepare spell can, which the verb checks. Hexhaven
/// Dueling Arena, Skycoach Waypoint and Biblioplex Tomekeeper print it.
pub fn target_becomes_prepared(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(id) = crate::targets::object(g, item, 0) {
        g.become_prepared(id);
    }
    Ok(())
}

/// Whether this object is a prepare spell (CR 722.3c-d), including copies. The
/// exile-only `prepared_from` link belongs to the old object after casting, so
/// follow the history's zone-change and copy provenance rather than a card name.
/// Codie, Ravenous Codex asks this when a spell becomes cast.
pub fn is_prepare_spell(q: &dyn Query, mut object: ObjectId) -> bool {
    if q.obj(object).prepared_from.is_some() {
        return true;
    }
    for record in q.history().all().iter().rev() {
        let predecessor = match record.event {
            Event::ZoneChange { old, new, .. } if new == object => Some(old),
            Event::Copied { copy, original, .. } if copy == object => Some(original),
            _ => None,
        };
        if let Some(previous) = predecessor {
            object = previous;
            if q.obj(object).prepared_from.is_some() {
                return true;
            }
        }
    }
    false
}

/// The intervening "if this creature isn't prepared" (CR 603.4), checked as the ability triggers and again as it resolves.
fn this_is_not_prepared(q: &dyn Query, asker: &Asker) -> bool {
    !q.obj(asker.source).prepared
}

/// "At the beginning of your upkeep, if this creature isn't prepared, it becomes prepared." (CR 603.4, 722.3a). Paradox Shaper,
/// Stingerquill Voxmancer and Woodwork Prodigy print it.
pub const AT_YOUR_UPKEEP_IF_NOT_PREPARED_BECOMES_PREPARED: AbilityDef = triggered(
    crate::triggers::YOUR_UPKEEP,
    become_prepared,
    "At the beginning of your upkeep, if this creature isn't prepared, it becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
)
.condition(this_is_not_prepared);
