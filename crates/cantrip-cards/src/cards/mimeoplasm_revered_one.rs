use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// "As Mimeoplasm enters, exile up to X creature cards from your graveyard. It enters with three +1/+1 counters on it for each
/// creature card exiled this way" (CR 614.12, 614.1c, 122.6): the cards are linked to the permanent the spell becomes
/// (CR 607.2a, 607.2q), so its own ability finds them.
fn as_enters_exile_x_creature_cards(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let asker = Asker::new(entering.object, entering.controller);
    let cards = g.state().zone(Zone::graveyard(entering.controller)).iter().rev().copied()
        .filter(|&id| targets::card_in_your_graveyard(g, &asker, Target::Object(id), &[]))
        .collect();
    let selected = choose::select(
        g,
        entering.controller,
        entering.object,
        Purpose::Exile,
        cards,
        select::creature_cards(select::up_to(entering.x)),
    )?;
    let exiled = g.exile_with(&selected, g.text_linked(entering.object), false);
    entering.counters[CounterKind::PlusOne as usize] += 3 * exiled.len() as i32;
    Ok(())
}

/// Whether `this` - the object as it is now (CR 400.7) - exiled `card`, the object it is in exile, by any of its abilities:
/// its own "as it enters" exile, or one it had from a copy effect or a grant, an ability's effect, cost or replacement effect
/// alike. The owner's ruling (jschibel, 2026-11-24): "exiled with it" in the copy ability reads every card it exiled, however
/// it was exiled, and not only those the ability linked to it exiled (CR 607.2a), so a copy of Phelia, Exuberant Shepherd can
/// become a copy of what its attack trigger exiled. Read from the card's exile link (`ExileLink`, any text: a link is made
/// by a cost, an "as it enters" exile and an ability's `exile_with`, CR 607.2q from the spell that became it) and from the
/// history's record of the move that put it there: the object whose replacement effect sent it (`sent_by`), or else the
/// ability resolving (`Cause::Ability`).
fn exiled_by(q: &dyn Query, this: ObjectId, card: ObjectId) -> bool {
    let by_this = |by: ObjectId| by == this || (q.obj(by).zone.is(ZoneKind::Stack) && q.obj(by).became == Some(this));
    if q.obj(card).exiled.is_some_and(|link| by_this(link.by)) {
        return true;
    }
    q.history().all().iter().rev().find_map(|r| match r.event {
        Event::ZoneChange { new, to, sent_by, .. } if new == card && to.is(ZoneKind::Exile) => Some(match r.cause {
            _ if sent_by != ObjectId::NONE => sent_by == this,
            Cause::Ability { source, .. } => source == this,
            _ => false,
        }),
        _ => None,
    }) == Some(true)
}

/// "Target creature card exiled with it" (CR 115.1): a creature card in exile that this Mimeoplasm exiled (`exiled_by`).
fn creature_card_exiled_by_this(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Exile) && q.chars(id).is(CardType::Creature) && exiled_by(q, asker.source, id))
}

/// "{2}: Mimeoplasm becomes a copy of target creature card exiled with it, except it's 0/0 and has this ability" (CR 707.9b,
/// 707.2): the card's copiable values with the base size 0/0, and this ability added so a later copy has it. It states no
/// duration (CR 611.2a), and the +1/+1 counters stay. As a copy it still reads every card it exiled (`exiled_by`), the ones
/// it exiled as it entered among them, so it can become a copy again and again.
fn become_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(card).sized(0, 0).with_ability(item.ability);
    g.become_copy(item.object, item.object, values, Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(as_enters_exile_x_creature_cards),
    abilities: &[ability(
        cost("{2}"),
        become_copy,
        "{2}: Mimeoplasm becomes a copy of target creature card exiled with it, except it's 0/0 and has this ability.",
    )
    .targets(&[target(creature_card_exiled_by_this)])],
    ..oracle::MIMEOPLASM_REVERED_ONE
};
