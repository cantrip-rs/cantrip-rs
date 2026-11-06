//! Thelon's Chant and Tourach's Chant distinguish the player putting a land
//! onto the battlefield from its entering controller (CR 110.2a, 608.2c).
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
const THIRD: PlayerId = PlayerId(2);
static SOURCE: CardDef = CardDef { name: "Entry actor source", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static SWAMP: CardDef = CardDef { name: "Entry actor Swamp", types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Swamp]), ..CardDef::BLANK };
static FOREST: CardDef = CardDef { name: "Entry actor Forest", types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Forest]), ..CardDef::BLANK };
fn enters(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match *event { Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) => Fired::about(Target::Object(new)), _ => None }
}
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static WATCHER: CardDef = CardDef { name: "Entry batch watcher", types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, enters), nothing, "Whenever one or more permanents enter, do nothing.").once_per_batch()], ..CardDef::BLANK };
fn entering(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Battlefield))
}
fn change_controller(_: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.controller = asker.controller; }
}
static CONTROL: CardDef = CardDef { name: "Entry controller replacement", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, entering, change_controller, "It enters under your control instead.")], ..CardDef::BLANK };
fn from_hand(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.from.is(ZoneKind::Hand) && z.to.is(ZoneKind::Graveyard))
}
fn return_instead(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.to = Zone::BATTLEFIELD; z.controller = THIRD; }
}
static RETURN: CardDef = CardDef { name: "Entry redirection replacement", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, from_hand, return_instead, "Put it onto the battlefield under the third player's control instead.")], ..CardDef::BLANK };
fn exile_instead(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.to = Zone::exile(q.obj(z.object).owner); }
}
static EXILE: CardDef = CardDef { name: "Entry prohibited replacement", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, entering, exile_instead, "Exile it instead.")], ..CardDef::BLANK };

fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &SWAMP, &FOREST, &WATCHER, &CONTROL, &RETURN, &EXILE])));
    let mut game = Game::new(cards, &Setup { decks: vec![Vec::new(); 3], ..Default::default() });
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    for p in [YOU, THEM, THIRD] { game.state.update_player(p, |p| p.drew_from_empty = false); }
    game
}
fn put(g: &mut Game, def: &'static CardDef, player: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), player, zone));
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}
fn resolving(g: &mut Game, actor: PlayerId) -> usize {
    let source = put(g, &SOURCE, THIRD, Zone::BATTLEFIELD);
    let item = crate::actions::stack_item(StackKind::Activated, source, AbilityRef { card: g.cards.id_of(&SOURCE), index: 0 }, actor, &[]);
    g.state.stack_push(item);
    let serial = g.state.stack.last().unwrap().serial;
    let at = g.state.agenda.len();
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    at
}
fn cx(g: &mut Game, frame: usize) -> Cx<'_> {
    Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}
fn putter(g: &Game, permanent: ObjectId) -> Option<PlayerId> {
    g.log.iter().find_map(|r| match r.event {
        Event::ZoneChange { new, to, put_by, .. } if new == permanent && to.is(ZoneKind::Battlefield) => put_by,
        _ => None,
    })
}

#[test]
fn a_resolving_effect_puts_a_land_even_under_an_opponents_control() {
    let mut g = game(); let frame = resolving(&mut g, YOU);
    let card = put(&mut g, &SWAMP, THEM, Zone::exile(THEM));
    let entered = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g, frame), card, THEM, false).unwrap();
    assert_eq!(putter(&g, entered), Some(YOU));
    assert_eq!(g.controller(entered), THEM);
}
#[test]
fn a_normal_land_play_has_the_player_performing_that_action() {
    let mut g = game();
    g.state.frame_push(Frame::new(FrameKind::Act { player: YOU }));
    let card = put(&mut g, &FOREST, YOU, Zone::hand(YOU));
    let entered = crate::ops::put_onto_battlefield(&mut cx(&mut g, 0), &[(card, YOU, false)], ZoneReason::PlayLand).unwrap()[0];
    assert_eq!(putter(&g, entered), Some(YOU));
}
#[test]
fn an_entry_control_replacement_cannot_reassign_a_land_play_to_the_entering_controller() {
    let mut g = game(); g.state.frame_push(Frame::new(FrameKind::Act { player: YOU }));
    put(&mut g, &CONTROL, THIRD, Zone::BATTLEFIELD);
    let card = put(&mut g, &SWAMP, YOU, Zone::hand(YOU));
    let entered = crate::ops::put_onto_battlefield(&mut cx(&mut g, 0), &[(card, YOU, false)], ZoneReason::PlayLand).unwrap()[0];
    assert_eq!(g.controller(entered), THIRD);
    assert_eq!(putter(&g, entered), Some(YOU));
}
#[test]
fn a_single_simultaneous_put_keeps_its_actor_for_different_entering_controllers() {
    let mut g = game(); let frame = resolving(&mut g, YOU);
    let first = put(&mut g, &SWAMP, YOU, Zone::hand(YOU));
    let second = put(&mut g, &FOREST, THEM, Zone::hand(THEM));
    let arrived = cantrip_api::Game::put_all_onto_battlefield_as(&mut cx(&mut g, frame), &[(first, YOU, false), (second, THEM, false)]).unwrap();
    assert_eq!(arrived.iter().map(|&id| putter(&g, id)).collect::<Vec<_>>(), vec![Some(YOU), Some(YOU)]);
}
#[test]
fn a_nested_replacement_or_mana_effect_uses_its_own_doer() {
    let mut g = game(); let frame = resolving(&mut g, YOU);
    let card = put(&mut g, &FOREST, THIRD, Zone::hand(THIRD));
    let entered = cx(&mut g, frame).acting_as(THEM, |cx| cantrip_api::Game::put_onto_battlefield(cx, card, THIRD, false)).unwrap();
    assert_eq!(putter(&g, entered), Some(THEM));
}

#[test]
fn an_explicit_instructed_player_is_neither_the_effect_controller_nor_the_entering_controller() {
    let mut g = game(); let frame = resolving(&mut g, YOU);
    let card = put(&mut g, &FOREST, THIRD, Zone::hand(THIRD));
    let entered = cantrip_api::Game::put_onto_battlefield_by(&mut cx(&mut g, frame), card, THIRD, false, THEM).unwrap();
    assert_eq!(putter(&g, entered), Some(THEM));
    assert_eq!(g.controller(entered), THIRD);
}
#[test]
fn simultaneous_named_players_still_make_one_entry_batch() {
    let mut g = game(); let frame = resolving(&mut g, THIRD);
    let watcher = put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let first = put(&mut g, &SWAMP, YOU, Zone::hand(YOU));
    let second = put(&mut g, &FOREST, THEM, Zone::hand(THEM));
    let arrived = cantrip_api::Game::put_all_onto_battlefield_by(&mut cx(&mut g, frame), &[(first, THEM, false, YOU), (second, YOU, true, THEM)], &[(CounterKind::Charge, 2)]).unwrap();
    assert_eq!(arrived.iter().map(|&id| putter(&g, id)).collect::<Vec<_>>(), vec![Some(YOU), Some(THEM)]);
    assert_eq!(g.controller(arrived[0]), THEM);
    assert_eq!(g.controller(arrived[1]), YOU);
    assert!(g.obj(arrived[1]).tapped);
    assert!(arrived.iter().all(|&id| g.obj(id).counters(CounterKind::Charge) == 2));
    let triggers: Vec<_> = g.state.triggers.iter().filter(|t| t.source == watcher).collect();
    assert_eq!(triggers.len(), 1);
    assert_eq!(g.state.members(triggers[0].members), arrived);
}
#[test]
fn changing_only_entry_control_does_not_change_the_player_putting_it_there() {
    let mut g = game(); let frame = resolving(&mut g, YOU);
    put(&mut g, &CONTROL, THIRD, Zone::BATTLEFIELD);
    let card = put(&mut g, &SWAMP, THEM, Zone::exile(THEM));
    let entered = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g, frame), card, THEM, false).unwrap();
    assert_eq!(putter(&g, entered), Some(YOU));
    assert_eq!(g.controller(entered), THIRD);
    let entry = g.log.iter().find(|r| matches!(r.event, Event::Entered { permanent, .. } if permanent == entered)).unwrap();
    assert_eq!(entry.event.player(), Some(THIRD), "existing entry-controller event semantics are preserved");
}
#[test]
fn a_token_is_put_by_its_creator_even_when_an_entry_replacement_takes_control() {
    let mut g = game(); let frame = resolving(&mut g, YOU);
    put(&mut g, &CONTROL, THIRD, Zone::BATTLEFIELD);
    let def = g.cards.id_of(&FOREST);
    let entered = cantrip_api::Game::create_tokens(&mut cx(&mut g, frame), def, THEM, 1, false).unwrap()[0];
    assert_eq!(putter(&g, entered), Some(THEM));
    assert_eq!(g.controller(entered), THIRD);
    assert_eq!(g.obj(entered).owner, THEM);
}
#[test]
fn a_replacement_redirecting_a_move_to_the_battlefield_performs_that_put() {
    let mut g = game(); let frame = resolving(&mut g, YOU);
    put(&mut g, &RETURN, THEM, Zone::BATTLEFIELD);
    let card = put(&mut g, &SWAMP, THIRD, Zone::hand(THIRD));
    cantrip_api::Game::put_into_graveyard(&mut cx(&mut g, frame), &[card]);
    let entered = g.obj(card).became.unwrap();
    assert_eq!(putter(&g, entered), Some(THEM));
    assert_eq!(g.controller(entered), THIRD);
}
#[test]
fn a_move_redirected_away_from_the_battlefield_has_no_putter() {
    let mut g = game(); let frame = resolving(&mut g, YOU);
    put(&mut g, &EXILE, THEM, Zone::BATTLEFIELD);
    let card = put(&mut g, &SWAMP, THIRD, Zone::hand(THIRD));
    cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g, frame), card, THIRD, false).unwrap();
    let moved = g.obj(card).became.unwrap();
    let event = g.log.iter().find(|r| matches!(r.event, Event::ZoneChange { new, .. } if new == moved)).unwrap();
    assert!(matches!(event.event, Event::ZoneChange { put_by: None, to, .. } if to.is(ZoneKind::Exile)));
}
#[test]
fn actorless_rules_or_fixture_setup_never_substitute_the_active_or_entering_player() {
    let mut g = game(); g.state.frame_push(Frame::new(FrameKind::Settle));
    let card = put(&mut g, &SWAMP, THEM, Zone::exile(THEM));
    let entered = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g, 0), card, THEM, false).unwrap();
    assert_eq!(putter(&g, entered), None);
}
#[test]
fn an_automatic_return_keeps_its_original_effects_player_not_the_later_removal_spell() {
    let mut g = game(); let original = resolving(&mut g, YOU);
    let source = g.state.stack.last().unwrap().object;
    let card = put(&mut g, &FOREST, THEM, Zone::BATTLEFIELD);
    let by = Linked { object: source, text: g.cards.id_of(&SOURCE), acquired: Acquired::OWN };
    let exiled = cantrip_api::Game::exile_with(&mut cx(&mut g, original), &[card], by, true)[0];
    let later = g.state.agenda.len(); g.state.frame_push(Frame::new(FrameKind::Act { player: THIRD }));
    cantrip_api::Game::sacrifice(&mut cx(&mut g, later), THIRD, &[source]);
    let returned = g.obj(exiled).became.unwrap();
    assert_eq!(putter(&g, returned), Some(YOU));
    assert_eq!(g.controller(returned), THEM);
}
