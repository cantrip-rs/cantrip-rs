//! Clockwork Beast and Clockwork Avian: an ability-scoped limit on the final counter placement,
//! not a permanent-wide ban, pre-replacement amount clamp, or add-then-remove operation (CR 101.2, 608.2c, 609.3).
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
const KIND: CounterKind = CounterKind::Plus1Plus0;
fn recharge(g: &mut dyn cantrip_api::Game, item: &StackItem, maximum: i32) -> Result<(), Suspend> {
    let most = u32::from(item.x);
    let chosen = cantrip_mechanics::choose::number(g, item.controller, item.object, 0, most, "Put counters")?.min(i32::MAX as u32) as i32;
    g.add_counters_with_limit(item.object, KIND, chosen, maximum);
    Ok(())
}
fn beast(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> { recharge(g, item, 7) }
fn avian(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> { recharge(g, item, 4) }
fn beast_enters(_: &mut dyn cantrip_api::Game, entry: &mut Entering) -> Result<(), Suspend> { entry.counters[KIND as usize] += 7; Ok(()) }
fn avian_enters(_: &mut dyn cantrip_api::Game, entry: &mut Entering) -> Result<(), Suspend> { entry.counters[KIND as usize] += 4; Ok(()) }
static BEAST: CardDef = CardDef { name: "Clockwork Beast", types: TypeSet::of(&[CardType::Artifact, CardType::Creature]), power: Some(0), toughness: Some(4),
    enters: Some(beast_enters), abilities: &[ability(cost("{X}, {T}"), beast, "Put up to X +1/+0 counters on this creature. This ability can't cause more than seven.")], ..CardDef::BLANK };
static AVIAN: CardDef = CardDef { name: "Clockwork Avian", types: TypeSet::of(&[CardType::Artifact, CardType::Creature]), power: Some(0), toughness: Some(4),
    enters: Some(avian_enters), abilities: &[ability(cost("{X}, {T}"), avian, "Put up to X +1/+0 counters on this creature. This ability can't cause more than four.")], ..CardDef::BLANK };
fn effected(_: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Counters(c) if c.effect && c.controller == asker.controller && c.counts.iter().any(|&n| n > 0))
}
fn twice(_: &dyn Query, _: &Asker, p: &mut Proposal) { if let Proposal::Counters(c) = p { c.counts = c.counts.map(|n| n.saturating_mul(2)); } }
static SEASON: CardDef = CardDef { name: "Ceiling Doubling Season", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, effected, twice, "If an effect puts counters on your permanent, double them.")], ..CardDef::BLANK };
fn other_putter(_: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Counters(c) if c.putter != asker.controller && c.counts.iter().any(|&n| n > 0))
}
fn half(_: &dyn Query, _: &Asker, p: &mut Proposal) { if let Proposal::Counters(c) = p { c.counts = c.counts.map(|n| n / 2); } }
static HALVER: CardDef = CardDef { name: "Ceiling Vorinclex", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, other_putter, half, "If an opponent puts counters, half rounded down instead.")], ..CardDef::BLANK };
fn your_putter(_: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Counters(c) if c.putter == asker.controller && c.counts.iter().any(|&n| n > 0))
}
static PUTTER: CardDef = CardDef { name: "Ceiling own-putter doubler", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, your_putter, twice, "If you put counters, twice instead.")], ..CardDef::BLANK };
fn mixed_applies(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Counters(c) if c.counts[KIND as usize] > 0)
}
fn mixed(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Counters(c) = p { c.counts[KIND as usize] *= 2; c.counts[CounterKind::Shield as usize] += 1; }
}
static MIXED: CardDef = CardDef { name: "Ceiling mixed counter replacement", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, mixed_applies, mixed, "Twice those counters and a shield counter instead.")], ..CardDef::BLANK };
fn ban(_: &dyn Query, asker: &Asker, _: ObjectId, controller: PlayerId, kind: CounterKind) -> bool {
    controller == asker.controller && kind == KIND
}
static BAN: CardDef = CardDef { name: "Ceiling counter ban", types: TypeSet::of(&[CardType::Enchantment]), counter_bans: &[ban], ..CardDef::BLANK };

fn put(g: &mut Game, def: &'static CardDef, player: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), player, zone));
    g.state.zone_push(zone, id); g.state.see_statics(); id
}
fn board(def: &'static CardDef, counters: i32, x: u32, modifier: Option<&'static CardDef>) -> (Game, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&BEAST, &AVIAN, &SEASON, &HALVER, &PUTTER, &MIXED, &BAN])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::Upkeep); });
    let object = put(&mut g, def, YOU, Zone::BATTLEFIELD);
    crate::counters::put(&mut cx(&mut g), object, KIND, counters, YOU, crate::counters::How::TurnBased);
    if let Some(modifier) = modifier { put(&mut g, modifier, if std::ptr::eq(modifier, &HALVER) { THEM } else { YOU }, Zone::BATTLEFIELD); }
    g.log.clear();
    let mut item = crate::actions::stack_item(StackKind::Activated, object, AbilityRef { card: cards.id_of(def), index: 0 }, YOU, &[]);
    item.x = x; g.state.stack_push(item); let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    (g, object)
}
fn cx(g: &mut Game) -> Cx<'_> { Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None } }
fn number(g: &mut Game, value: u32) {
    g.run(); let ask = g.pending().cloned().unwrap();
    assert!(matches!(ask.kind, RequestKind::ChooseNumber { .. }));
    g.answer(ask.id, ask.player, Answer::Announce(value)).expect("the chosen number is offered");
}

#[test]
fn beast_six_plus_one_under_doubling_season_stops_at_seven() {
    let (mut g, object) = board(&BEAST, 6, 1, Some(&SEASON)); number(&mut g, 1);
    assert_eq!(g.obj(object).counters(KIND), 7);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::CountersPut { object: id, kind: KIND, amount: 1, total: 7, by: YOU, .. } if id == object)));
}

#[test]
fn a_wide_requested_placement_is_replaced_then_limited_before_physical_storage() {
    let (mut g, object) = board(&BEAST, 6, 0, Some(&HALVER));
    cantrip_api::Game::add_counters_with_limit(&mut cx(&mut g), object, KIND, 65_535, 7);
    assert_eq!(g.obj(object).counters(KIND), 7);
    assert!(g.log.iter().any(|r| matches!(r.event,
        Event::CountersPut { object: id, kind: KIND, amount: 1, total: 7, by: YOU, .. } if id == object)));
}
#[test]
fn avian_three_plus_one_under_doubling_season_stops_at_four() {
    let (mut g, object) = board(&AVIAN, 3, 1, Some(&SEASON)); number(&mut g, 1);
    assert_eq!(g.obj(object).counters(KIND), 4);
}
#[test]
fn unmodified_clockwork_recharging_is_an_ordinary_positive_control() {
    for (def, before, maximum) in [(&BEAST, 6, 7), (&AVIAN, 3, 4)] {
        let (mut g, object) = board(def, before, 1, None); number(&mut g, 1);
        assert_eq!(g.obj(object).counters(KIND), maximum);
        assert_eq!(g.chars(object).toughness, Some(4));
    }
}
#[test]
fn entering_counters_are_not_subject_to_the_activated_abilitys_limit() {
    let (mut g, _) = board(&BEAST, 1, 0, Some(&SEASON));
    for (def, expected) in [(&BEAST, 14), (&AVIAN, 8)] {
        let card = put(&mut g, def, YOU, Zone::hand(YOU));
        let entered = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), card, YOU, false).unwrap();
        assert_eq!(g.obj(entered).counters(KIND), expected);
    }
}
#[test]
fn the_full_x_choice_is_available_when_a_replacement_can_halve_it_into_the_remaining_room() {
    let (mut g, object) = board(&BEAST, 6, 2, Some(&HALVER));
    assert_eq!(g.obj(object).counters(KIND), 6, "the forced starting position predates Vorinclex"); g.run();
    assert!(matches!(g.pending().map(|r| &r.kind), Some(RequestKind::ChooseNumber { least: 0, most: 2, .. })), "X remains2, not the one remaining slot");
    number(&mut g, 2);
    assert_eq!(g.obj(object).counters(KIND), 7);
}
#[test]
fn choosing_zero_does_not_create_a_counter_placement_for_replacements_to_increase() {
    let (mut g, object) = board(&BEAST, 6, 1, Some(&SEASON)); number(&mut g, 0);
    assert_eq!(g.obj(object).counters(KIND), 6);
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::CountersPut { .. })));
}
#[test]
fn existing_counters_above_the_limit_are_not_removed() {
    let (mut g, object) = board(&BEAST, 9, 1, Some(&SEASON)); number(&mut g, 0);
    assert_eq!(g.obj(object).counters(KIND), 9);
}
#[test]
fn another_effect_can_put_the_clockwork_above_its_recharge_limit() {
    let (mut g, object) = board(&BEAST, 6, 0, Some(&SEASON));
    cantrip_api::Game::add_counters(&mut cx(&mut g), object, KIND, 1);
    assert_eq!(g.obj(object).counters(KIND), 8);
}
#[test]
fn choosing_fewer_than_x_uses_the_actual_chosen_quantity() {
    let (mut g, object) = board(&BEAST, 0, 3, None); number(&mut g, 1);
    assert_eq!(g.obj(object).counters(KIND), 1);
}
#[test]
fn an_at_or_above_cap_positive_request_does_not_remove_any_existing_counters() {
    for (def, before) in [(&BEAST, 7), (&BEAST, 9), (&AVIAN, 4), (&AVIAN, 6)] {
        let (mut g, object) = board(def, before, 1, Some(&SEASON)); number(&mut g, 1);
        assert_eq!(g.obj(object).counters(KIND), before);
        assert!(!g.log.iter().any(|r| matches!(r.event, Event::CountersPut { .. })));
    }
}
#[test]
fn the_limit_is_applied_after_the_entire_replacement_chain_not_between_replacements() {
    let (mut g, object) = board(&BEAST, 4, 2, Some(&SEASON));
    let season = g.state.battlefield().iter().copied().find(|&id| g.cards.get(g.obj(id).card).name == SEASON.name).unwrap();
    put(&mut g, &HALVER, THEM, Zone::BATTLEFIELD); number(&mut g, 2);
    let ask = g.pending().cloned().unwrap();
    let RequestKind::ChooseReplacement { sources, .. } = &ask.kind else { panic!("replacement ordering: {:?}", ask.kind) };
    let first = sources.iter().position(|&id| id == season).unwrap();
    g.answer(ask.id, ask.player, Answer::Index(first as u8)).unwrap();
    assert_eq!(g.obj(object).counters(KIND), 6, "double2 to4, then halve to2; no premature clamp of4 to remaining room3");
}
#[test]
fn the_abilitys_actual_putter_is_independent_of_the_clockworks_current_controller() {
    let (mut g, object) = board(&BEAST, 3, 1, None);
    put(&mut g, &PUTTER, THEM, Zone::BATTLEFIELD);
    let mut item = g.state.stack[0].clone(); item.controller = THEM; g.state.stack_set(0, item);
    number(&mut g, 1);
    assert_eq!(g.controller(object), YOU);
    assert_eq!(g.obj(object).counters(KIND), 5);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::CountersPut { object: id, by: THEM, amount: 2, .. } if id == object)));
}
#[test]
fn only_the_constrained_kind_is_limited_other_counters_from_replacement_survive() {
    let (mut g, object) = board(&BEAST, 6, 1, Some(&MIXED));
    crate::counters::put(&mut cx(&mut g), object, CounterKind::PlusOne, 3, YOU, crate::counters::How::TurnBased);
    g.log.clear(); number(&mut g, 1);
    assert_eq!(g.obj(object).counters(KIND), 7);
    assert_eq!(g.obj(object).counters(CounterKind::PlusOne), 3);
    assert_eq!(g.obj(object).counters(CounterKind::Shield), 1);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::CountersPut { object: id, kind: CounterKind::Shield, amount: 1, .. } if id == object)));
}
#[test]
fn existing_counter_bans_are_not_bypassed_by_the_limited_placement_or_replacements() {
    let (mut g, object) = board(&BEAST, 6, 1, Some(&MIXED));
    put(&mut g, &BAN, YOU, Zone::BATTLEFIELD); number(&mut g, 1);
    assert_eq!(g.obj(object).counters(KIND), 6);
    assert_eq!(g.obj(object).counters(CounterKind::Shield), 0);
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::CountersPut { .. })));
}
#[test]
fn stale_and_phased_recipients_receive_no_limited_placement() {
    let (mut g, object) = board(&BEAST, 6, 1, None);
    crate::phasing::phase_out(&mut cx(&mut g), &[object], None, false); let events = g.log.len();
    cantrip_api::Game::add_counters_with_limit(&mut cx(&mut g), object, KIND, 1, 7);
    assert_eq!(g.obj(object).counters(KIND), 6); assert_eq!(g.log.len(), events);
    let (mut g, object) = board(&BEAST, 6, 1, None);
    cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[object]); let events = g.log.len();
    cantrip_api::Game::add_counters_with_limit(&mut cx(&mut g), object, KIND, 1, 7);
    assert_eq!(g.obj(object).counters(KIND), 6); assert_eq!(g.log.len(), events);
}
#[test]
fn limited_counter_storage_and_events_undo_and_redo_with_the_game_log_tree() {
    let (mut g, object) = board(&BEAST, 6, 1, Some(&SEASON)); g.run(); let before = g.at;
    number(&mut g, 1); let after = g.at;
    assert_eq!(g.obj(object).counters(KIND), 7);
    g.goto(before); assert_eq!(g.obj(object).counters(KIND), 6);
    assert!(matches!(g.pending().unwrap().kind, RequestKind::ChooseNumber { most: 1, .. }));
    g.goto(after); assert_eq!(g.obj(object).counters(KIND), 7);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::CountersPut { object: id, amount: 1, total: 7, .. } if id == object)));
}
