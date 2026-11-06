//! CR 400.7f: an enchanted permanent's leave ability can find only its Aura's permitted first graveyard incarnation.
use crate::{Cx, Game, Setup};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::PendingTrigger;
const YOU: PlayerId = PlayerId(0);
static HOST: CardDef = CardDef {
    name: "Aura identity host",
    types: TypeSet::of(&[CardType::Creature, CardType::Land]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    t.object()
        .is_some_and(|o| q.is_on_battlefield(o) && q.chars(o).is(CardType::Creature))
}
static AURA: CardDef = CardDef {
    name: "Aura identity Aura",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(creature)),
    ..CardDef::BLANK
};
static EQUIPMENT: CardDef = CardDef {
    name: "Aura identity Equipment",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    ..CardDef::BLANK
};
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &HOST,
        &AURA,
        &EQUIPMENT,
        &GRAVEYARD_AURA,
    ])));
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    );
    for i in 0..2 {
        g.state
            .update_player(PlayerId(i), |p| p.drew_from_empty = false);
    }
    g
}
fn put(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let o = g
        .state
        .push_obj(Obj::new(g.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, o);
    g.state.see_statics();
    o
}
fn position() -> (Game, ObjectId, ObjectId) {
    let mut g = game();
    let h = put(&mut g, &HOST);
    let a = put(&mut g, &AURA);
    g.state.update_obj(a, |o| o.attached_to = Some(h));
    (g, h, a)
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: usize::MAX,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: vec![], damage: None,
        asked_at: None,
        doer: None,
        producing: None,
    }
}
fn moved(g: &mut Game, ids: &[ObjectId], to: Zone, reason: ZoneReason) {
    let moves: Vec<_> = ids.iter().map(|&o| crate::ops::Moving::to(o, to)).collect();
    crate::ops::move_together(&mut cx(g), &moves, reason);
}
fn find(q: &dyn Query, a: ObjectId, host: ObjectId) -> Option<ObjectId> {
    q.found_aura_after_enchanted_left(a, host)
}
#[test]
fn simultaneous_host_and_aura_departures_are_found() {
    let (mut g, h, a) = position();
    moved(&mut g, &[h, a], Zone::graveyard(YOU), ZoneReason::Destroy);
    assert_eq!(find(&cx(&mut g), a, h), g.state.obj(a).became);
}
#[test]
fn unattached_aura_sba_after_host_leaves_is_found() {
    let (mut g, h, a) = position();
    moved(&mut g, &[h], Zone::graveyard(YOU), ZoneReason::Destroy);
    crate::settle::state_based_actions(&mut cx(&mut g)).unwrap();
    assert_eq!(find(&cx(&mut g), a, h), g.state.obj(a).became);
}
#[test]
fn separate_same_cause_moves_do_not_qualify() {
    let (mut g, h, a) = position();
    moved(&mut g, &[h], Zone::graveyard(YOU), ZoneReason::Other);
    moved(&mut g, &[a], Zone::graveyard(YOU), ZoneReason::Other);
    assert_eq!(find(&cx(&mut g), a, h), None);
}
#[test]
fn aura_departing_before_host_does_not_qualify() {
    let (mut g, h, a) = position();
    moved(&mut g, &[a], Zone::graveyard(YOU), ZoneReason::Other);
    moved(&mut g, &[h], Zone::graveyard(YOU), ZoneReason::Other);
    assert_eq!(find(&cx(&mut g), a, h), None);
}
#[test]
fn wrong_old_host_does_not_qualify() {
    let (mut g, h, a) = position();
    let wrong = put(&mut g, &HOST);
    moved(&mut g, &[h, a], Zone::graveyard(YOU), ZoneReason::Destroy);
    assert_eq!(find(&cx(&mut g), a, wrong), None);
}
#[test]
fn unattached_aura_sba_with_host_still_present_does_not_qualify() {
    let (mut g, h, a) = position();
    g.state.update_obj(a, |o| o.attached_to = None);
    crate::settle::state_based_actions(&mut cx(&mut g)).unwrap();
    assert_eq!(find(&cx(&mut g), a, h), None);
}
#[test]
fn equipment_does_not_get_aura_exception() {
    let (mut g, h, _) = position();
    let a = put(&mut g, &EQUIPMENT);
    g.state.update_obj(a, |o| o.attached_to = Some(h));
    moved(&mut g, &[h, a], Zone::graveyard(YOU), ZoneReason::Other);
    assert_eq!(find(&cx(&mut g), a, h), None);
}
#[test]
fn first_incarnation_moved_again_is_not_followed() {
    let (mut g, h, a) = position();
    moved(&mut g, &[h, a], Zone::graveyard(YOU), ZoneReason::Other);
    let first = g.state.obj(a).became.unwrap();
    moved(&mut g, &[first], Zone::hand(YOU), ZoneReason::Other);
    let second = g.state.obj(first).became.unwrap();
    moved(&mut g, &[second], Zone::graveyard(YOU), ZoneReason::Other);
    assert_eq!(find(&cx(&mut g), a, h), None);
}
#[test]
fn aura_exiled_instead_is_not_found() {
    let (mut g, h, a) = position();
    moved(&mut g, &[h, a], Zone::exile(YOU), ZoneReason::Exile);
    assert_eq!(find(&cx(&mut g), a, h), None);
}

#[test]
fn simultaneous_aura_first_in_move_order_still_finds_host() {
    let (mut g, h, a) = position();
    moved(&mut g, &[a, h], Zone::graveyard(YOU), ZoneReason::Other);
    assert_eq!(find(&cx(&mut g), a, h), g.state.obj(a).became);
}
#[test]
fn aura_uses_owners_graveyard_not_trigger_controllers() {
    let (mut g, h, a) = position();
    g.state.update_obj(a, |o| o.owner = PlayerId(1));
    moved(&mut g, &[h, a], Zone::graveyard(YOU), ZoneReason::Other);
    let found = find(&cx(&mut g), a, h).unwrap();
    assert_eq!(g.state.obj(found).zone, Zone::graveyard(PlayerId(1)));
}
#[test]
fn host_reentry_does_not_erase_original_departure() {
    let (mut g, h, a) = position();
    moved(&mut g, &[h], Zone::graveyard(YOU), ZoneReason::Other);
    let first = g.state.obj(h).became.unwrap();
    cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), first, YOU, false).unwrap();
    crate::settle::state_based_actions(&mut cx(&mut g)).unwrap();
    assert_eq!(find(&cx(&mut g), a, h), g.state.obj(a).became);
}
#[test]
fn reattached_aura_cannot_be_found_by_original_hosts_ability() {
    let (mut g, h, a) = position();
    let next = put(&mut g, &HOST);
    moved(&mut g, &[h], Zone::graveyard(YOU), ZoneReason::Other);
    cantrip_api::Game::attach(&mut cx(&mut g), a, next);
    moved(&mut g, &[next, a], Zone::graveyard(YOU), ZoneReason::Other);
    assert_eq!(find(&cx(&mut g), a, h), None);
    assert_eq!(find(&cx(&mut g), a, next), g.state.obj(a).became);
}
#[test]
fn replaced_host_destruction_does_not_qualify() {
    let (mut g, h, a) = position();
    let mut c = cx(&mut g);
    c.add_effect(h, h, EffectKind::RegenerationShield, Duration::EndOfTurn);
    c.end_sentence();
    drop(c);
    moved(&mut g, &[h, a], Zone::graveyard(YOU), ZoneReason::Destroy);
    assert!(g.state.obj(h).became.is_none());
    assert_eq!(find(&cx(&mut g), a, h), None);
}
#[test]
fn aura_departure_replaced_by_exile_does_not_qualify() {
    let (mut g, h, a) = position();
    let moves = [
        crate::ops::Moving::to(h, Zone::graveyard(YOU)),
        crate::ops::Moving::to(a, Zone::exile(YOU)),
    ];
    crate::ops::move_together(&mut cx(&mut g), &moves, ZoneReason::Other);
    assert_eq!(find(&cx(&mut g), a, h), None);
}
#[test]
fn metadata_survives_copy_undo_redo_and_goto() {
    let (mut g, h, a) = position();
    let before = g.at;
    let start = (g.state.mark(), g.log.len());
    let child = g.tree.add(before, (0, YOU, Answer::Pass), start, &[]);
    g.at = child;
    moved(&mut g, &[h], Zone::graveyard(YOU), ZoneReason::Other);
    crate::settle::state_based_actions(&mut cx(&mut g)).unwrap();
    let arrived = find(&cx(&mut g), a, h).unwrap();
    let state = g.state.clone();
    let history = g.log.clone();
    let mut copy = g.clone();
    assert_eq!(find(&cx(&mut copy), a, h), Some(arrived));
    g.goto(before);
    assert_eq!(find(&cx(&mut g), a, h), None);
    assert!(g.state.obj(a).became.is_none());
    g.goto(child);
    assert_eq!(g.state, state);
    assert_eq!(g.log, history);
    assert_eq!(find(&cx(&mut g), a, h), Some(arrived));
}
#[test]
fn aura_metadata_fits_the_combined_event_budget() {
    // The combined domain includes snow counts and independent acquired abilities.
    assert_eq!(std::mem::size_of::<Event>(), 40);
    assert_eq!(std::mem::size_of::<Record>(), 72);
    println!(
        "Aura metadata layout: Event={} Record={} StackItem={} PendingTrigger={}",
        std::mem::size_of::<Event>(),
        std::mem::size_of::<Record>(),
        std::mem::size_of::<StackItem>(),
        std::mem::size_of::<PendingTrigger>()
    );
}

#[test]
fn attached_but_illegal_aura_sba_does_not_get_the_exception() {
    let (mut g, h, a) = position();
    let card = g.cards.id_of(&EQUIPMENT);
    g.state.update_obj(h, |o| o.copy = Some(Copiable::of(card)));
    g.state.see_statics();
    crate::settle::state_based_actions(&mut cx(&mut g)).unwrap();
    assert!(g.state.obj(h).became.is_none());
    assert!(g.state.obj(a).became.is_some());
    assert_eq!(find(&cx(&mut g), a, h), None);
}

fn graveyard_card(q: &dyn Query, _: &Asker, t: Target) -> bool {
    t.object().is_some_and(|id| {
        is_live(q, id)
            && q.obj(id).zone.is(ZoneKind::Graveyard)
            && q.chars(id).is(CardType::Creature)
    })
}
// Animate Dead and Spellweaver Volute establish that an Aura's enchanted object need not be a permanent.
static GRAVEYARD_AURA: CardDef = CardDef {
    name: "Aura identity graveyard-card Aura",
    enchant: Some(target(graveyard_card)),
    ..AURA
};
fn graveyard_position() -> (Game, ObjectId, ObjectId) {
    let mut g = game();
    let former = put(&mut g, &HOST);
    moved(&mut g, &[former], Zone::graveyard(YOU), ZoneReason::Other);
    let card = g.state.obj(former).became.unwrap();
    let aura = put(&mut g, &GRAVEYARD_AURA);
    g.state.update_obj(aura, |o| o.attached_to = Some(card));
    assert!(
        crate::attach::host_allows(&cx(&mut g), aura, card, false),
        "a legal graveyard-card attachment, not an illegal forced position"
    );
    assert!(!crate::settle::state_based_actions(&mut cx(&mut g)).unwrap());
    (g, card, aura)
}
fn assert_no_origin(g: &mut Game, aura: ObjectId, card: ObjectId) {
    let actual = g
        .log
        .iter()
        .find_map(|r| match r.event {
            Event::ZoneChange {
                old,
                enchanted_left,
                ..
            } if old == aura => Some(enchanted_left),
            _ => None,
        })
        .expect("Aura actually departed");
    assert_eq!(
        actual, None,
        "producer must not stamp a nonpermanent host's departure"
    );
    assert_eq!(find(&cx(g), aura, card), None);
}
#[test]
fn graveyard_card_departure_followed_by_aura_sba_is_not_battlefield_exception() {
    let (mut g, card, aura) = graveyard_position();
    moved(&mut g, &[card], Zone::hand(YOU), ZoneReason::Other);
    crate::settle::state_based_actions(&mut cx(&mut g)).unwrap();
    assert_no_origin(&mut g, aura, card);
}
#[test]
fn simultaneous_graveyard_card_and_aura_departure_is_not_battlefield_exception() {
    let (mut g, card, aura) = graveyard_position();
    let moves = [
        crate::ops::Moving::to(card, Zone::hand(YOU)),
        crate::ops::Moving::to(aura, Zone::graveyard(YOU)),
    ];
    crate::ops::move_together(&mut cx(&mut g), &moves, ZoneReason::Other);
    assert_no_origin(&mut g, aura, card);
}
