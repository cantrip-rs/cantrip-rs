//! The player an effect names puts the counters it has them put (CR 122.6, 608.2c, 701.68a): "target opponent blights 2",
//! "its controller amasses". Over test cards: a spell of yours resolving, two watchers with "whenever you put one or more
//! counters on a permanent" (Nest of Scarabs', Hapatra's kind), and an enchantment with "if you would put one or more
//! counters on a permanent, put twice that many instead" (Innkeeper's Talent's, Vorinclex's kind).

use crate::actions::stack_item;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// "Whenever you put one or more counters on a permanent" (CR 122.6): the putter is the watcher's controller.
fn you_put(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, by, .. } if *by == me.controller => Fired::about(Target::Object(*object)),
        _ => None,
    }
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static WATCHER: CardDef = CardDef {
    name: "Counter Putter Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when_asked(EventMask::COUNTERS, you_put), nothing, "Whenever you put one or more counters on a permanent, do nothing.")
        .once_per_batch()],
    ..CardDef::BLANK
};

/// "If you would put one or more counters on a permanent, put twice that many instead": keyed to the putter, never to the
/// permanent's controller.
fn you_would_put(_: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(c) if c.effect && c.putter == me.controller)
}

fn twice(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        for n in &mut c.counts {
            *n *= 2;
        }
    }
}

static DOUBLER: CardDef = CardDef {
    name: "Counter Putter Doubler",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, you_would_put, twice, "put twice the counters you would put")],
    ..CardDef::BLANK
};

static BEAR: CardDef =
    CardDef { name: "Counter Putter Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

/// The spell resolving: whose effect it is, of YOU's.
static SPELL: CardDef = CardDef { name: "Counter Putter Spell", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&WATCHER, &DOUBLER, &BEAR, &SPELL])));
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

/// YOU's spell put on the stack and resolving: the frame a resolving effect's card code runs in (`Cx::doer` is YOU).
fn resolving(g: &mut Game) -> usize {
    let card = g.cards.id_of(&SPELL);
    let spell = g.state.push_obj(Obj::new(card, YOU, Zone::STACK));
    g.state.zone_push(Zone::STACK, spell);
    let item = stack_item(StackKind::Spell, spell, AbilityRef { card, index: 0 }, YOU, &[]);
    g.state.stack_push(item);
    let serial = g.state.stack.last().expect("the spell").serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.state.agenda.len() - 1
}

fn cx(g: &mut Game, frame: usize) -> Cx<'_> {
    Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The watchers' triggers waiting to be put on the stack, by watcher.
fn fired(g: &Game, watcher: ObjectId) -> usize {
    g.state.triggers.iter().filter(|t| t.source == watcher).count()
}

/// The putters of the counters announced, in order.
fn putters(g: &Game) -> Vec<(ObjectId, i32, PlayerId)> {
    g.log
        .iter()
        .filter_map(|r| match r.event {
            Event::CountersPut { object, amount, by, .. } => Some((object, amount, by)),
            _ => None,
        })
        .collect()
}

/// YOU's spell has THEM put counters on THEM's creature ("target opponent blights 2"): THEM puts them, so THEM's watcher
/// triggers and YOU's does not, and THEM's "if you would put" doubles them while YOU's does not apply.
#[test]
fn counters_an_effect_has_the_opponent_put_are_the_opponents() {
    let mut g = game();
    let (mine, theirs) = (put(&mut g, &WATCHER, YOU), put(&mut g, &WATCHER, THEM));
    put(&mut g, &DOUBLER, THEM);
    let bear = put(&mut g, &BEAR, THEM);
    let frame = resolving(&mut g);
    let mut c = cx(&mut g, frame);
    assert_eq!(c.doer(), YOU, "the spell's effect is its controller's");
    cantrip_api::Game::add_counters_by(&mut c, THEM, bear, CounterKind::MinusOne, 2);
    assert_eq!(putters(&g), vec![(bear, 4, THEM)]);
    assert_eq!(g.state.obj(bear).counters(CounterKind::MinusOne), 4);
    assert_eq!((fired(&g, mine), fired(&g, theirs)), (0, 1));
}

/// The same counters put by the spell's own effect (`add_counters`) are YOU's: YOU's watcher triggers, THEM's does not, and
/// THEM's doubler does not apply.
#[test]
fn counters_the_effect_puts_itself_are_its_controllers() {
    let mut g = game();
    let (mine, theirs) = (put(&mut g, &WATCHER, YOU), put(&mut g, &WATCHER, THEM));
    put(&mut g, &DOUBLER, THEM);
    let bear = put(&mut g, &BEAR, THEM);
    let frame = resolving(&mut g);
    cantrip_api::Game::add_counters(&mut cx(&mut g, frame), bear, CounterKind::MinusOne, 2);
    assert_eq!(putters(&g), vec![(bear, 2, YOU)]);
    assert_eq!((fired(&g, mine), fired(&g, theirs)), (1, 0));
}

/// "Each player puts a counter on a creature they control" as one instruction (CR 101.4): each player's counters are theirs,
/// each replaced by their own "if you would put" alone: each watcher triggers once.
#[test]
fn counters_put_together_by_several_players_are_each_theirs() {
    let mut g = game();
    let (mine, theirs) = (put(&mut g, &WATCHER, YOU), put(&mut g, &WATCHER, THEM));
    put(&mut g, &DOUBLER, THEM);
    let (my_bear, their_bear) = (put(&mut g, &BEAR, YOU), put(&mut g, &BEAR, THEM));
    let frame = resolving(&mut g);
    cantrip_api::Game::add_counters_together_by(
        &mut cx(&mut g, frame),
        &[(YOU, my_bear, CounterKind::PlusOne, 1), (THEM, their_bear, CounterKind::PlusOne, 1)],
    );
    assert_eq!(putters(&g), vec![(my_bear, 1, YOU), (their_bear, 2, THEM)]);
    assert_eq!((fired(&g, mine), fired(&g, theirs)), (1, 1));
}
