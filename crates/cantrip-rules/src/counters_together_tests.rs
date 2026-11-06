//! Counters put on several objects by one instruction, or of several kinds on one, are one simultaneous event (CR 608.2,
//! 101.4, 122.6): a "whenever one or more counters are put on one or more ..." ability (Cloaked Cadet, Invisible Woman, Sue
//! Storm) triggers once for all of them (CR 603.2c), and each object's own replacement effects of counters being put apply
//! to it alone, before any counter is put on (CR 614.1a). Over test cards: a watcher with such an ability, creatures, one
//! with infect, and an enchantment that doubles the counters put on the other player's permanents.

use crate::{Cx, Game, Setup, counters, ops};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// "Whenever one or more counters are put on one or more permanents you control" (CR 603.2c), about each of them.
fn counters_on_yours(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, .. } if q.state().obj(*object).controller == me.controller => Fired::about(Target::Object(*object)),
        _ => None,
    }
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static WATCHER: CardDef = CardDef {
    name: "Counters Together Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(
        when_asked(EventMask::COUNTERS, counters_on_yours),
        nothing,
        "Whenever one or more counters are put on one or more permanents you control, do nothing.",
    )
    .once_per_batch()],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "Counters Together Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

static BLIGHTER: CardDef = CardDef { name: "Counters Together Blighter", keywords: Keywords::of(&[Keyword::Infect]), ..BEAR };

/// "If one or more counters would be put on a permanent an opponent controls, twice that many are put on it instead."
fn on_theirs(_: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(c) if c.controller != me.controller)
}

fn twice(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        for n in &mut c.counts {
            *n *= 2;
        }
    }
}

static DOUBLER: CardDef = CardDef {
    name: "Counters Together Doubler",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, on_theirs, twice, "double the counters put on an opponent's permanents")],
    ..CardDef::BLANK
};

/// "If one or more counters would be put on a permanent, that many plus one of each of those kinds are put on it instead"
/// (Winding Constrictor's, Pir's).
fn any_permanent(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(_))
}

fn plus_one(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        for n in c.counts.iter_mut().filter(|n| **n > 0) {
            *n += 1;
        }
    }
}

static PLUS_ONE: CardDef = CardDef {
    name: "Counters Together Plus One",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, any_permanent, plus_one, "that many plus one of each kind")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&WATCHER, &BEAR, &BLIGHTER, &DOUBLER, &PLUS_ONE])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}

fn put(g: &mut Game, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), controller, Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn one(kind: CounterKind, n: i32) -> [i32; CounterKind::COUNT] {
    let mut counts = [0; CounterKind::COUNT];
    counts[kind as usize] = n;
    counts
}

/// The watcher's triggers waiting to be put on the stack, each with the permanents it was triggered about.
fn waiting(g: &Game, watcher: ObjectId) -> Vec<Vec<ObjectId>> {
    g.state.triggers.iter().filter(|t| t.source == watcher).map(|t| g.state.members(t.members).to_vec()).collect()
}

/// The counters announced, in order: on what, of which kind, how many.
fn announced(g: &Game) -> Vec<(ObjectId, CounterKind, i32)> {
    g.log
        .iter()
        .filter_map(|r| match r.event {
            Event::CountersPut { object, kind, amount, .. } => Some((object, kind, amount)),
            _ => None,
        })
        .collect()
}

/// A +1/+1 counter on each of two creatures, as one instruction: the watcher triggers once, about both.
#[test]
fn counters_on_two_creatures_trigger_one_or_more_once() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let (a, b) = (put(&mut g, &BEAR, YOU), put(&mut g, &BEAR, YOU));
    let placements = [a, b].map(|object| counters::Placement { object, counts: one(CounterKind::PlusOne, 1), by: YOU });
    counters::put_together(&mut cx(&mut g), &placements, counters::How::Effect);
    assert_eq!(waiting(&g, watcher), vec![vec![a, b]]);
    assert_eq!(announced(&g), vec![(a, CounterKind::PlusOne, 1), (b, CounterKind::PlusOne, 1)]);
}

/// Two kinds put on one creature together ("two +1/+1 counters and a flying counter on it") are one event: the watcher
/// triggers once.
#[test]
fn two_kinds_on_one_creature_trigger_one_or_more_once() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let bear = put(&mut g, &BEAR, YOU);
    let mut counts = one(CounterKind::PlusOne, 2);
    counts[CounterKind::Flying as usize] = 1;
    counters::put_all(&mut cx(&mut g), bear, counts, YOU, counters::How::Effect);
    assert_eq!(waiting(&g, watcher), vec![vec![bear]]);
    assert_eq!(g.state.obj(bear).counters(CounterKind::PlusOne), 2);
    assert_eq!(g.state.obj(bear).counters(CounterKind::Flying), 1);
}

/// Proliferating two permanents (CR 701.34a): each gets one more of each kind it has, and all of it is one event.
#[test]
fn proliferating_two_permanents_triggers_one_or_more_once() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let (a, b) = (put(&mut g, &BEAR, YOU), put(&mut g, &BEAR, YOU));
    let mut counts = one(CounterKind::PlusOne, 1);
    counts[CounterKind::Shield as usize] = 1;
    counters::put_all(&mut cx(&mut g), a, counts, YOU, counters::How::Effect);
    counters::put(&mut cx(&mut g), b, CounterKind::Stun, 1, YOU, counters::How::Effect);
    while !g.state.triggers.is_empty() {
        g.state.trigger_remove(0);
    }
    g.state.update_frame(0, |f| f.answers.push((Some(YOU), Answer::Targets(vec![Target::Object(a), Target::Object(b)]))));
    assert!(ops::proliferate(&mut cx(&mut g), YOU, watcher).is_ok());
    assert_eq!(waiting(&g, watcher), vec![vec![a, b]]);
    assert_eq!([CounterKind::PlusOne, CounterKind::Shield].map(|k| g.state.obj(a).counters(k)), [2, 2]);
    assert_eq!(g.state.obj(b).counters(CounterKind::Stun), 2);
}

/// Damage from a source with infect dealt to two creatures at once is dealt as -1/-1 counters (CR 702.90c, 120.3d), put on
/// both at once and announced with the damage: the watcher triggers once.
#[test]
fn infect_damage_to_two_creatures_puts_its_counters_on_once() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, THEM);
    let blighter = put(&mut g, &BLIGHTER, YOU);
    let (a, b) = (put(&mut g, &BEAR, THEM), put(&mut g, &BEAR, THEM));
    ops::damage_together(&mut cx(&mut g), &[(blighter, Target::Object(a), 1), (blighter, Target::Object(b), 1)]);
    assert_eq!(waiting(&g, watcher), vec![vec![a, b]]);
    assert_eq!([a, b].map(|id| g.state.obj(id).counters(CounterKind::MinusOne)), [1, 1]);
}

/// A replacement effect of counters being put applies to the objects it applies to, and only to them (CR 614.1a): of a
/// counter on each of two creatures, the opponent's gets two; and the event is still one.
#[test]
fn a_replacement_applies_to_its_own_object_of_several() {
    let mut g = game();
    put(&mut g, &DOUBLER, YOU);
    let watcher = put(&mut g, &WATCHER, YOU);
    let theirs_watcher = put(&mut g, &WATCHER, THEM);
    let (mine, theirs) = (put(&mut g, &BEAR, YOU), put(&mut g, &BEAR, THEM));
    let placements = [mine, theirs].map(|object| counters::Placement { object, counts: one(CounterKind::PlusOne, 1), by: YOU });
    counters::put_together(&mut cx(&mut g), &placements, counters::How::Effect);
    assert_eq!(g.state.obj(mine).counters(CounterKind::PlusOne), 1);
    assert_eq!(g.state.obj(theirs).counters(CounterKind::PlusOne), 2);
    assert_eq!(announced(&g), vec![(mine, CounterKind::PlusOne, 1), (theirs, CounterKind::PlusOne, 2)]);
    assert_eq!(waiting(&g, watcher), vec![vec![mine]]);
    assert_eq!(waiting(&g, theirs_watcher), vec![vec![theirs]]);
}

/// What one instruction puts on one object is one placement, whose replacements apply once: "that many plus one" adds one
/// of each kind for it, however many entries name the object (Winding Constrictor's and Pir's rulings).
#[test]
fn what_one_instruction_puts_on_one_object_is_one_placement() {
    let mut g = game();
    put(&mut g, &PLUS_ONE, YOU);
    let bear = put(&mut g, &BEAR, YOU);
    let mut c = cx(&mut g);
    c.doer = Some(YOU);
    cantrip_api::Game::add_counters_together(
        &mut c,
        &[(bear, CounterKind::PlusOne, 1), (bear, CounterKind::Flying, 1), (bear, CounterKind::PlusOne, 1)],
    );
    assert_eq!(announced(&g), vec![(bear, CounterKind::PlusOne, 3), (bear, CounterKind::Flying, 2)]);
}

/// Each source's damage has its own result (CR 120.3d, 120.4c): two infect sources of one controller dealing damage to one
/// creature at once put two placements of -1/-1 counters on it, each replaced on its own - "that many plus one" adds one
/// for each - and both are announced with the damage, in one event (CR 603.2c).
#[test]
fn two_infect_sources_on_one_creature_are_each_replaced() {
    let mut g = game();
    put(&mut g, &PLUS_ONE, YOU);
    let watcher = put(&mut g, &WATCHER, THEM);
    let (one, two) = (put(&mut g, &BLIGHTER, YOU), put(&mut g, &BLIGHTER, YOU));
    let bear = put(&mut g, &BEAR, THEM);
    ops::damage_together(&mut cx(&mut g), &[(one, Target::Object(bear), 1), (two, Target::Object(bear), 1)]);
    assert_eq!(g.state.obj(bear).counters(CounterKind::MinusOne), 4);
    assert_eq!(announced(&g), vec![(bear, CounterKind::MinusOne, 2), (bear, CounterKind::MinusOne, 2)]);
    assert_eq!(waiting(&g, watcher), vec![vec![bear]]);
}
