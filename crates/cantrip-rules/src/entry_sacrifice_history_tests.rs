//! History of sacrifices made by an entering object's own replacement (CR 614.1c, 607.2, 400.7).
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

fn entry(g: &mut dyn cantrip_api::Game, e: &mut Entering) -> Result<(), Suspend> {
    let land = g.state().battlefield().iter().copied().find(|&id| g.chars(id).is(CardType::Land));
    if let Some(land) = land {
        let left = g.sacrifice_as_enters(e, e.controller, &[land]);
        let linked = Linked { object: e.object, text: g.card_id(&ENTRY), acquired: Acquired::OWN };
        assert_eq!(g.history().sacrificed_as_entered(g.state(), linked).count(), left.count() as usize, "visible before entry completes");
    }
    Ok(())
}
fn define(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let count = q.history().sacrificed_as_entered(q.state(), q.linked_to(source)).count() as i32;
    chars.power = Some(count);
    chars.toughness = Some(count);
}
static ENTRY: CardDef = CardDef {
    name: "Entry sacrifice history", types: TypeSet::of(&[CardType::Creature]),
    power: Some(0), toughness: Some(0), cost: ManaCost::ZERO, has_cost: true, enters: Some(entry),
    statics: &[characteristic_defining(Layer::PtDefine, define, "sacrificed as it entered")], ..CardDef::BLANK
};
static FOREST: CardDef = CardDef { name: "Entry history Forest", types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Forest]), ..CardDef::BLANK };
static ISLAND: CardDef = CardDef { name: "Entry history Island", types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Island]), ..CardDef::BLANK };
fn rejected_entry(g: &mut dyn cantrip_api::Game, e: &mut Entering) -> Result<(), Suspend> {
    let land = g.state().battlefield().iter().copied().find(|&id| g.chars(id).is(CardType::Land));
    if let Some(land) = land { g.sacrifice_as_enters(e, e.controller, &[land]); }
    e.instead = Some(Zone::graveyard(e.controller));
    Ok(())
}
static REJECTED: CardDef = CardDef { name: "Entry history rejected", enters: Some(rejected_entry), ..ENTRY };
fn stops_entry(q: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Battlefield) && q.obj(z.object).card == q.card_id(&ENTRY))
}
fn stop_entry(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ZoneChange(z) = p { z.replaced = true; }
}
static STOPPER: CardDef = CardDef {
    name: "Entry history stopper", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, stops_entry, stop_entry, "stop entry")], ..CardDef::BLANK
};
fn exiles_sacrifice(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ZoneChange(z) if z.from.is(ZoneKind::Battlefield) && z.reason == ZoneReason::Sacrifice)
}
fn exile_instead(q: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ZoneChange(z) = p { z.to = Zone::exile(q.obj(z.object).owner); }
}
static EXILER: CardDef = CardDef {
    name: "Entry history exiler", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, exiles_sacrifice, exile_instead, "exile instead")], ..CardDef::BLANK
};
static UNSACRIFIABLE: CardDef = CardDef { name: "Entry history forbidden land", types: TypeSet::of(&[CardType::Land]), keywords: Keywords::of(&[Keyword::CantBeSacrificed]), ..CardDef::BLANK };
static OTHER: CardDef = CardDef { name: "Entry history other", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

fn game() -> (Game, &'static CardTable) {
    let cards = Box::leak(Box::new(CardTable::new([&ENTRY, &FOREST, &ISLAND, &OTHER, &UNSACRIFIABLE, &REJECTED, &STOPPER, &EXILER])));
    let mut game = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&ISLAND); 20]; 2], ..Default::default() });
    game.state.update_turn(|t| { t.number = 1; t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    (game, cards)
}
fn put(g: &mut Game, t: &CardTable, card: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(t.id_of(card), YOU, zone));
    g.state.zone_push(zone, id);
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None }
}

#[test]
fn entry_associations_keep_the_actual_last_known_rows_and_distinguish_incarnations_and_text() {
    let (mut g, t) = game();
    let forest = put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    let first = put(&mut g, t, &ENTRY, Zone::graveyard(YOU));
    let first = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), first, YOU, false).unwrap();
    assert_eq!(g.chars(first).power, Some(1));
    let island = put(&mut g, t, &ISLAND, Zone::BATTLEFIELD);
    let second = put(&mut g, t, &ENTRY, Zone::graveyard(YOU));
    let second = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), second, YOU, false).unwrap();
    let link = g.text_linked(first);
    let rows: Vec<_> = g.history().sacrificed_as_entered(&g.state, link).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, forest);
    assert!(rows[0].subtypes.has(Subtype::Forest));
    let rows: Vec<_> = g.history().sacrificed_as_entered(&g.state, g.text_linked(second)).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, island);
    assert!(rows[0].subtypes.has(Subtype::Island));
    assert_eq!(g.history().sacrificed_as_entered(&g.state, Linked { text: t.id_of(&OTHER), ..link }).count(), 0);
    let acquisition = Acquired { by: AcquiredBy::Copy, stamp: 1, part: 0 };
    assert_eq!(g.history().sacrificed_as_entered(&g.state, Linked { acquired: acquisition, ..link }).count(), 0);
    let card = g.state.obj(forest).became.unwrap();
    cantrip_api::Game::exile_all(&mut cx(&mut g), &[card]);
    assert!(g.history().sacrificed_as_entered(&g.state, link).next().unwrap().subtypes.has(Subtype::Forest));
    let grave = cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[first]).objects()[0];
    let again = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), grave, YOU, false).unwrap();
    let next_link = Linked { object: again, ..link };
    assert_eq!(g.history().sacrificed_as_entered(&g.state, next_link).count(), 0, "a later incarnation never inherits an entry");
}

#[test]
fn a_token_copy_enters_with_its_own_linked_sacrifice() {
    let (mut g, t) = game();
    let forest = put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    let token = cantrip_api::Game::create_token_copies_of(&mut cx(&mut g), Copiable::of(t.id_of(&ENTRY)), YOU, 1).unwrap()[0];
    assert_eq!(g.chars(token).power, Some(1));
    assert_eq!(g.history().sacrificed_as_entered(&g.state, g.text_linked(token)).next().unwrap().id, forest);
}

#[test]
fn entry_sacrifice_does_not_widen_each_history_record() {
    assert_eq!(std::mem::size_of::<cantrip_api::history::Record>(), 52);
}

#[test]
fn simultaneous_entries_keep_separate_sacrifices() {
    let (mut g, t) = game();
    let forest = put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    let island = put(&mut g, t, &ISLAND, Zone::BATTLEFIELD);
    let a = put(&mut g, t, &ENTRY, Zone::graveyard(YOU));
    let b = put(&mut g, t, &ENTRY, Zone::graveyard(YOU));
    crate::ops::put_onto_battlefield(&mut cx(&mut g), &[(a, YOU, false), (b, YOU, false)], ZoneReason::Other).unwrap();
    let a = g.state.obj(a).became.unwrap();
    let b = g.state.obj(b).became.unwrap();
    let victims = |source| g.history().sacrificed_as_entered(&g.state, g.text_linked(source)).map(|r| r.id).collect::<Vec<_>>();
    assert_eq!(victims(a), vec![forest]);
    assert_eq!(victims(b), vec![island]);
}

#[test]
fn a_later_copy_of_the_same_text_is_a_different_acquisition() {
    let (mut g, t) = game();
    put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    let original = put(&mut g, t, &ENTRY, Zone::graveyard(YOU));
    let entered = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), original, YOU, false).unwrap();
    assert_eq!(g.chars(entered).power, Some(1));
    cantrip_api::Game::become_copy(&mut cx(&mut g), entered, entered, Copiable::of(t.id_of(&OTHER)), Duration::Indefinite);
    assert_eq!(g.chars(entered).power, Some(2));
    cantrip_api::Game::become_copy(&mut cx(&mut g), entered, entered, Copiable::of(t.id_of(&ENTRY)), Duration::Indefinite);
    assert_eq!(g.chars(entered).power, Some(0));
    assert_eq!(g.history().sacrificed_as_entered(&g.state, g.text_linked(entered)).count(), 0);
}

#[test]
fn a_permanent_that_cannot_be_sacrificed_is_not_recorded() {
    let (mut g, t) = game();
    let land = put(&mut g, t, &UNSACRIFIABLE, Zone::BATTLEFIELD);
    let object = put(&mut g, t, &ENTRY, Zone::graveyard(YOU));
    let entered = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), object, YOU, false).unwrap();
    assert!(g.state.battlefield().contains(&land));
    assert_eq!(g.history().sacrificed_as_entered(&g.state, g.text_linked(entered)).count(), 0);
}

#[test]
fn entry_sacrifice_history_survives_tree_undo_redo_and_search_fork() {
    use cantrip_state::{Frame, FrameKind};
    let (mut g, t) = game();
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let forest = put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    let card = put(&mut g, t, &ENTRY, Zone::hand(YOU));
    g.run();
    let before = g.checkpoint();
    let request = g.pending().cloned().unwrap();
    let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
    let cast = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).unwrap().clone();
    g.answer(request.id, request.player, Answer::Act(Action::of(&cast))).unwrap();
    loop {
        g.run();
        let request = g.pending().cloned().unwrap();
        match &request.kind {
            RequestKind::PayCosts { options, .. } => {
                let option = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).unwrap();
                g.answer(request.id, request.player, Answer::Act(Action::of(option))).unwrap();
            }
            RequestKind::Priority { .. } if g.state.stack.is_empty() && g.state.obj(card).became.is_some() && request.player == YOU => break,
            RequestKind::Priority { .. } => g.answer(request.id, request.player, Answer::Pass).unwrap(),
            other => panic!("unexpected {other:?}"),
        }
    }
    let stack = g.state.obj(card).became.unwrap();
    let entered = g.state.obj(stack).became.unwrap();
    let after = g.checkpoint();
    let history = g.log.clone();
    assert_eq!(g.history().sacrificed_as_entered(&g.state, g.text_linked(entered)).next().unwrap().id, forest);
    let fork = g.fork_for_search();
    assert_eq!(fork.history().sacrificed_as_entered(&fork.state, fork.text_linked(entered)).count(), 1);
    g.goto(before);
    assert!(g.state.battlefield().contains(&forest));
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::EntrySacrifice { .. })));
    g.goto(after);
    assert_eq!(g.log, history);
    assert_eq!(g.history().sacrificed_as_entered(&g.state, g.text_linked(entered)).next().unwrap().id, forest);
}

#[test]
fn a_replaced_entry_does_not_belong_to_the_object_in_the_other_zone() {
    let (mut g, t) = game();
    put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    let original = put(&mut g, t, &REJECTED, Zone::hand(YOU));
    let result = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), original, YOU, false).unwrap();
    assert!(g.state.obj(result).zone.is(ZoneKind::Graveyard));
    assert_eq!(g.history().sacrificed_as_entered(&g.state, g.text_linked(result)).count(), 0);
}

#[test]
fn a_canceled_attempt_does_not_leak_into_a_later_entry_of_the_same_object() {
    use cantrip_state::{Frame, FrameKind};
    let (mut g, t) = game();
    let forest = put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    let stopper = put(&mut g, t, &STOPPER, Zone::BATTLEFIELD);
    let entering = put(&mut g, t, &ENTRY, Zone::hand(YOU));
    g.state.see_statics();
    g.state.frame_push(Frame::new(FrameKind::Settle));
    let frame = g.state.agenda.len() - 1;
    g.state.update_frame(frame, |f| f.answers = vec![(None, Answer::Index(0))]);
    let mut context = Cx { g: &mut g, frame, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None };
    cantrip_api::Game::put_onto_battlefield(&mut context, entering, YOU, false).unwrap();
    assert!(context.s().obj(entering).became.is_none(), "the first attempt did not move");
    assert!(context.g.log.iter().any(|r| matches!(r.event, Event::EntrySacrifice { sacrificed, .. } if sacrificed == forest)));
    assert!(context.g.log.iter().any(|r| matches!(r.event, Event::EntryAttemptFailed { entering: id } if id == entering)));
    context.st().zone_remove(Zone::BATTLEFIELD, stopper);
    context.st().update_obj(stopper, |o| o.zone = Zone::GONE);
    let island = context.st().push_obj(Obj::new(t.id_of(&ISLAND), YOU, Zone::BATTLEFIELD));
    context.st().zone_push(Zone::BATTLEFIELD, island);
    let new = cantrip_api::Game::put_onto_battlefield(&mut context, entering, YOU, false).unwrap();
    let rows: Vec<_> = context.history().sacrificed_as_entered(context.s(), context.linked_to(&Asker::new(new, YOU))).map(|row| row.id).collect();
    assert_eq!(rows, vec![island], "the canceled attempt's Forest is not inherited");
}

#[test]
fn a_sacrifice_redirected_to_exile_still_contributes_its_battlefield_history() {
    let (mut g, t) = game();
    let forest = put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    put(&mut g, t, &EXILER, Zone::BATTLEFIELD);
    g.state.see_statics();
    let object = put(&mut g, t, &ENTRY, Zone::graveyard(YOU));
    let entered = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), object, YOU, false).unwrap();
    assert!(g.state.obj(g.state.obj(forest).became.unwrap()).zone.is(ZoneKind::Exile));
    let rows: Vec<_> = g.history().sacrificed_as_entered(&g.state, g.text_linked(entered)).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, forest);
    assert!(rows[0].subtypes.has(Subtype::Forest));
}

#[test]
fn sacrificed_token_can_cease_without_erasing_its_battlefield_receipt() {
    let (mut g, t) = game();
    let forest = put(&mut g, t, &FOREST, Zone::BATTLEFIELD);
    g.state.update_obj(forest, |o| o.token = true);
    let object = put(&mut g, t, &ENTRY, Zone::graveyard(YOU));
    let entered = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), object, YOU, false).unwrap();
    let grave_token = g.state.obj(forest).became.unwrap();
    crate::settle::state_based_actions(&mut cx(&mut g)).unwrap();
    assert!(g.state.obj(grave_token).zone.is(ZoneKind::Gone));
    assert_eq!(g.history().sacrificed_as_entered(&g.state, g.text_linked(entered)).map(|row| row.id).collect::<Vec<_>>(), [forest]);
    assert_eq!(g.chars(entered).power, Some(1));
}
