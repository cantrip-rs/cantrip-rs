use cantrip_api::*;
use cantrip_mechanics::{choose, effects};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Whenever one or more other creatures you control enter, if they entered or were cast from a graveyard"
/// (CR 603.2c, 603.4): asked of each creature of the batch, and it keeps the ones that qualify as the batch's members,
/// the creatures to choose among. A creature qualifies if it entered from a graveyard, or came from the stack and was
/// cast from one, which this turn's history says.
fn from_a_graveyard(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, new, from, to, reason, .. } = event else { return None };
    if !to.is(ZoneKind::Battlefield) || *new == source || q.controller(*new) != q.controller(source) {
        return None;
    }
    if !q.chars(*new).is(CardType::Creature) {
        return None;
    }
    let cast_from_graveyard = *reason == ZoneReason::Resolve
        && q.history().cast_of(*old).is_some_and(|c| c.from == ZoneKind::Graveyard);
    if from.is(ZoneKind::Graveyard) || cast_from_graveyard { Fired::about(Target::Object(*new)) } else { None }
}

/// "Create a token that's a copy of one of them" (CR 707.2, 608.2h): the controller chooses among the members the
/// trigger kept, by name, since one that has left since is still one of them; the token has its copiable values.
fn copy_one_of_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let them = g.event_objects(item).to_vec();
    let Some(&first) = them.first() else { return Ok(()) };
    let chosen = if them.len() > 1 {
        let modes: Vec<Label> = them.iter().map(|&m| Label::Name(g.chars(m).card)).collect();
        them[choose::one_of(g, item.controller, item.object, modes)? as usize]
    } else {
        first
    };
    g.create_token_copies(chosen, item.controller, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::surveil(g, item, 2), "When this creature enters, surveil 2."),
        triggered(when(EventMask::ZONE_CHANGE, from_a_graveyard), copy_one_of_them, "Whenever one or more other creatures you control enter, if they entered or were cast from a graveyard, create a token that's a copy of one of them. This ability triggers only once each turn.")
            .triggers_each_turn(1)
            .once_per_batch(),
    ],
    ..oracle::TWILIGHT_DIVINER
};
