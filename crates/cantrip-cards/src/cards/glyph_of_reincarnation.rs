use cantrip_api::*;
use cantrip_mechanics::{choose, combat, effects, history, select, targets};
use cantrip_oracle as oracle;

/// "The player who controlled that creature the last time it became blocked by that Wall" (CR 509.1h, 110.2, 608.2h): read
/// back from the creature as it died through the history. Walking back from its death, each change of control since that last
/// block by `wall` (CR 506.4: the change removed it from combat, it is still the same object, CR 110.2) hands it back to the
/// player it came from; the block itself ends the walk.
fn controller_when_last_blocked(q: &dyn Query, wall: ObjectId, creature: ObjectId) -> PlayerId {
    let mut controller = q.obj(creature).controller;
    let about = EventQuery::kind(EventMask(EventMask::BLOCKS.0 | EventMask::CONTROL_CHANGED.0)).object(creature).this_turn();
    for record in q.history().select(about).rev() {
        match record.event {
            Event::ControlChanged { from, .. } => controller = from,
            Event::Blocks { blocker, .. } if blocker == wall => break,
            _ => {}
        }
    }
    controller
}

/// "Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died
/// this way, put a creature card from the graveyard of the player who controlled that creature the last time it became
/// blocked by that Wall onto the battlefield under its owner's control." (CR 509.1h, 701.8a, 701.19c, 700.4, 608.2c).
/// The creatures go to the graveyard first, then the cards are chosen, so a creature just destroyed may come back (the
/// 2004-10-04 ruling). One that was indestructible, or went elsewhere instead, did not die. The cards are chosen by the
/// spell's controller as it resolves, not targeted, as many as can be from each graveyard up to the number owed to it, and
/// all enter at once; a token that died is in its graveyard until the state-based actions (CR 704.5d), but is no card
/// (CR 111.1).
fn destroy_blocked_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(wall) = targets::object(g, item, 0) else { return Ok(()) };
    let blocked = history::creatures_blocked_by_this_turn(g, wall);
    let destroyed = g.destroy_all_no_regen(&blocked);
    g.end_sentence();
    let owed: Vec<PlayerId> = effects::put_into_a_graveyard(g, &destroyed)
        .into_iter()
        .map(|creature| controller_when_last_blocked(g, wall, creature))
        .collect();
    let mut entering: Vec<(ObjectId, PlayerId)> = Vec::new();
    for player in g.state().apnap() {
        let count = owed.iter().filter(|&&p| p == player).count() as u32;
        if count == 0 {
            continue;
        }
        let cards: Vec<ObjectId> = g.state().zone(Zone::graveyard(player)).iter().copied().filter(|&c| !g.obj(c).token).collect();
        let rule = select::creature_cards(select::as_many_as_can(count));
        for card in choose::select(g, item.controller, item.object, Purpose::PutOntoBattlefield, cards, rule)? {
            entering.push((card, g.obj(card).owner));
        }
    }
    g.put_all_onto_battlefield(&entering)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::WALL], destroy_blocked_and_return),
    // "Cast this spell only after combat." (CR 601.3, 506.8)
    cast_only_if: Some(combat::cast_only_after_combat),
    ..oracle::GLYPH_OF_REINCARNATION
};
