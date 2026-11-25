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

/// Whether an ability its source had this way is one of its own (`exiled_by`): printed on it, or in its copiable values (CR
/// 707.2, 707.9b) - its own text or a copy effect's - and not one another effect granted or lent it (CR 613.1f: Agatha's
/// Soul Cauldron's, Quicksilver Elemental's).
fn own_ability(acquired: Acquired) -> bool {
    acquired.by != AcquiredBy::Grant
}

/// Whether `this` - the object as it is now (CR 400.7) - exiled `card`, the object it is in exile, by an ability printed on
/// it or copied onto it: its own "as it enters" exile (CR 607.2b), or an ability of the copiable values it had then (CR
/// 707.2, 707.9b), its effect, cost or replacement effect alike. The owner's ruling (jschibel, 2026-11-24): "The concept of a
/// card 'exiled with it' means it was exiled with an ability 'printed' on that card (or copied like the Phelia case) and that
/// is an attribute of the exiled card that persists" - so a copy of Phelia, Exuberant Shepherd can become a copy of what its
/// attack trigger exiled, but not of a card an ability Agatha's Soul Cauldron lent it exiled. Read from the card's exile link
/// (`ExileLink`, any text, with how `by` had the ability: a link is made by a cost, an "as it enters" exile and an ability's
/// `exile_with`, CR 607.2q from the spell that became it) and from the history's record of the move that put it there: the
/// object whose replacement effect sent it (`sent_by`; a printed replacement effect is one of its text), or else the ability
/// resolving (`Cause::Ability`), how its source had it read from that resolution's end (its `Resolved`, then `ResolvedAs`).
fn exiled_by(q: &dyn Query, this: ObjectId, card: ObjectId) -> bool {
    let by_this = |by: ObjectId| by == this || (q.obj(by).zone.is(ZoneKind::Stack) && q.obj(by).became == Some(this));
    if let Some(link) = q.obj(card).exiled.filter(|link| by_this(link.by)) {
        return own_ability(link.acquired);
    }
    let records = q.history().all();
    let exiled = records.iter().rposition(|r| matches!(r.event, Event::ZoneChange { new, to, .. } if new == card && to.is(ZoneKind::Exile)));
    exiled.is_some_and(|at| match (records[at].event, records[at].cause) {
        (Event::ZoneChange { sent_by, .. }, _) if sent_by != ObjectId::NONE => sent_by == this,
        // The resolution's records end at its `Resolved`; a `ResolvedAs` right after says how its source had the ability, and
        // none says it was its own text.
        (_, Cause::Ability { source, ability }) if source == this => {
            let mut after = records[at + 1..].iter();
            after.position(|r| matches!(r.event, Event::Resolved { source: s, ability: a, .. } if s == source && a == ability)).is_some()
                && !matches!(after.next().map(|r| r.event), Some(Event::ResolvedAs { acquired }) if !own_ability(acquired))
        }
        _ => false,
    })
}

/// "Target creature card exiled with it" (CR 115.1): a creature card in exile that this Mimeoplasm exiled (`exiled_by`).
fn creature_card_exiled_by_this(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Exile) && q.chars(id).is(CardType::Creature) && exiled_by(q, asker.source, id))
}

/// "{2}: Mimeoplasm becomes a copy of target creature card exiled with it, except it's 0/0 and has this ability" (CR 707.9b,
/// 707.2): the card's copiable values with the base size 0/0, and this ability added so a later copy has it. It states no
/// duration (CR 611.2a), and the +1/+1 counters stay. As a copy it still reads every card an ability of its own or of its
/// copiable values exiled (`exiled_by`), the ones it exiled as it entered among them, so it can become a copy again and again.
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
