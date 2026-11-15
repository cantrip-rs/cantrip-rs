//! Speculative resolution (CR 608.2, `speculate`): each way a stack object would come out if it resolved now, by every legal
//! answer and every chance outcome, on a hidden copy that leaves the game as it was. Test-only cards reach what no
//! implemented card does yet: chance, both kinds of visitor, a payment, a shuffle read and unread.

use std::ops::ControlFlow;

use crate::{Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Speculation filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static WASTE: CardDef = CardDef { name: "Speculation waste", types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")], ..CardDef::BLANK };

const SORCERY: TypeSet = TypeSet::of(&[CardType::Sorcery]);

fn land(g: &dyn cantrip_api::Game, item: &StackItem) -> Option<ObjectId> {
    cantrip_mechanics::targets::object(g, item, 0)
}

fn destroy_it(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(land) = land(g, item) {
        g.destroy(land);
    }
    Ok(())
}

fn may_destroy_it(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = land(g, item) else { return Ok(()) };
    if g.ask(item.controller, RequestKind::May { source: item.object, purpose: Purpose::Optional })? == Answer::Yes {
        g.destroy(land);
    }
    Ok(())
}

fn destroy_it_on_heads(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = land(g, item) else { return Ok(()) };
    if g.flip_coin(item.controller) {
        g.destroy(land);
    }
    Ok(())
}

fn roll_then_destroy_it(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = land(g, item) else { return Ok(()) };
    let rolled = g.roll_die(item.controller, 6).natural;
    g.gain_life(item.controller, rolled as i32);
    g.destroy(land);
    Ok(())
}

static DESTROY_UNLESS: Question = Question::new("Destroyed", "pay {cost}, or {subject} is destroyed?", "", "Pay {cost}", "Let it be destroyed");

fn destroy_it_unless_paid(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = land(g, item) else { return Ok(()) };
    let payer = g.controller(land);
    let mana = ManaCost { generic: 1, ..ManaCost::ZERO };
    if !g.may_pay(payer, item.object, land, Cost { mana, ..Cost::NONE }, &DESTROY_UNLESS)? {
        g.destroy(land);
    }
    Ok(())
}

fn destroy_it_then_shuffle(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(land) = land(g, item) {
        g.destroy(land);
    }
    g.shuffle(item.controller);
    Ok(())
}

fn shuffle_then_draw(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.shuffle(item.controller);
    g.draw(item.controller, 1);
    Ok(())
}

fn destroy_it_then_ask_thrice(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(land) = land(g, item) {
        g.destroy(land);
    }
    for _ in 0..3 {
        if g.ask(item.controller, RequestKind::May { source: item.object, purpose: Purpose::Optional })? == Answer::Yes {
            g.gain_life(item.controller, 1);
        }
    }
    Ok(())
}

fn ask_eighteen_times(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    for _ in 0..18 {
        if g.ask(item.controller, RequestKind::May { source: item.object, purpose: Purpose::Optional })? == Answer::Yes {
            g.gain_life(item.controller, 1);
        }
    }
    Ok(())
}

fn destroy_every_land(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    g.destroy_all(&lands);
    Ok(())
}

static ORDER_READER: CardDef = CardDef { name: "Speculation graveyard reader", types: TypeSet::of(&[CardType::Artifact]),
    reads_graveyard_order: true, ..CardDef::BLANK };
static DESTROY_ALL: CardDef = CardDef { name: "Speculation destroy all", types: SORCERY, spell: spell(&[], destroy_every_land), ..CardDef::BLANK };
static DESTROY_ALL_READING: CardDef = CardDef { name: "Speculation destroy all, reading", types: SORCERY,
    spell: spell(&[], destroy_every_land), reads_graveyard_order: true, ..CardDef::BLANK };

macro_rules! sorcery {
    ($name:ident, $text:literal, $effect:ident) => {
        static $name: CardDef = CardDef { name: $text, types: SORCERY,
            spell: spell(&[cantrip_mechanics::targets::LAND], $effect), ..CardDef::BLANK };
    };
}
sorcery!(DESTROY, "Speculation destroy", destroy_it);
sorcery!(MAY_DESTROY, "Speculation may destroy", may_destroy_it);
sorcery!(COIN_DESTROY, "Speculation coin destroy", destroy_it_on_heads);
sorcery!(ROLL_DESTROY, "Speculation roll destroy", roll_then_destroy_it);
sorcery!(UNLESS_DESTROY, "Speculation unless destroy", destroy_it_unless_paid);
sorcery!(SHUFFLE_DESTROY, "Speculation shuffle destroy", destroy_it_then_shuffle);
sorcery!(DESTROY_ASK, "Speculation destroy then ask", destroy_it_then_ask_thrice);
static MANY_WAYS: CardDef = CardDef { name: "Speculation many ways", types: SORCERY, spell: spell(&[], ask_eighteen_times), ..CardDef::BLANK };
static SHUFFLE_DRAW: CardDef = CardDef { name: "Speculation shuffle draw", types: SORCERY, spell: spell(&[], shuffle_then_draw), ..CardDef::BLANK };

fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &FILLER, &WASTE, &DESTROY, &MAY_DESTROY, &COIN_DESTROY, &ROLL_DESTROY, &UNLESS_DESTROY, &SHUFFLE_DESTROY, &SHUFFLE_DRAW, &DESTROY_ASK, &MANY_WAYS,
        &ORDER_READER, &DESTROY_ALL, &DESTROY_ALL_READING,
    ])));
    Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Default::default() })
}

fn put(g: &mut Game, def: &'static CardDef, player: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), player, Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

/// `def` cast by `caster`, on the stack with `targets`.
fn cast(g: &mut Game, def: &'static CardDef, caster: PlayerId, targets: &[Target]) -> ObjectId {
    let card = g.cards.id_of(def);
    let spell = g.state.push_obj(Obj::new(card, caster, Zone::STACK));
    g.state.zone_push(Zone::STACK, spell);
    let item = crate::actions::stack_item(StackKind::Spell, spell, AbilityRef { card, index: 0 }, caster, targets);
    g.state.stack_push(item);
    spell
}

/// What a game shows of itself: every state field but a cache the layers fill, its history and its place in its tree.
fn snapshot(g: &Game) -> String {
    let mut state = g.state.clone();
    state.static_sources = std::cell::RefCell::new(cantrip_state::StaticSources::none());
    format!("{state:?} {:?} {:?}", g.log, g.at)
}

fn destroyed_by(outcome: &Speculated<'_>, player: PlayerId) -> bool {
    outcome.records.iter().any(|r| matches!(r.event, Event::ZoneChange { old, from, reason: ZoneReason::Destroy, .. }
        if from == Zone::BATTLEFIELD && outcome.game.obj(old).controller == player))
}

/// "Would": every outcome destroys a land of `player`'s.
fn would(g: &Game, spell: ObjectId, player: PlayerId) -> Speculation<bool> {
    speculate(g, Target::Object(spell), |o| if destroyed_by(o, player) { ControlFlow::Continue(()) } else { ControlFlow::Break(false) })
}

/// "Could": some outcome destroys a land of `player`'s.
fn could(g: &Game, spell: ObjectId, player: PlayerId) -> Speculation<bool> {
    speculate(g, Target::Object(spell), |o| if destroyed_by(o, player) { ControlFlow::Break(true) } else { ControlFlow::Continue(()) })
}

/// Every outcome, each as the visitor sees it.
fn outcomes(g: &Game, spell: ObjectId, mut seen: impl FnMut(&Speculated<'_>)) -> usize {
    let mut n = 0;
    let came = speculate(g, Target::Object(spell), |o| {
        n += 1;
        seen(o);
        ControlFlow::<()>::Continue(())
    });
    assert_eq!(came, Speculation::Undecided);
    n
}

#[test]
fn a_spell_that_destroys_the_target_would_and_could_and_leaves_the_game_as_it_was() {
    let mut g = game();
    let mine = put(&mut g, &FILLER, YOU);
    let spell = cast(&mut g, &DESTROY, THEM, &[Target::Object(mine)]);
    let before = snapshot(&g);
    assert_eq!(would(&g, spell, YOU), Speculation::Undecided, "every outcome destroys it: it would");
    assert_eq!(could(&g, spell, YOU), Speculation::Decided(true));
    assert_eq!(would(&g, spell, THEM), Speculation::Decided(false), "not their land");
    assert_eq!(outcomes(&g, spell, |o| assert!(o.game.state().stack.iter().all(|i| i.object != spell))), 1);
    assert_eq!(snapshot(&g), before, "the game, its history and its tree are as they were");
    assert!(g.state.agenda.iter().all(|f| f.pending.is_none()), "no one was asked anything");
    assert_eq!(speculate(&g, Target::Object(mine), |_| ControlFlow::<()>::Continue(())), Speculation::NotOnStack, "a land is no stack object");
}

#[test]
fn a_choice_is_tried_both_ways_and_the_first_break_ends_the_search() {
    let mut g = game();
    let mine = put(&mut g, &FILLER, YOU);
    let spell = cast(&mut g, &MAY_DESTROY, THEM, &[Target::Object(mine)]);
    let before = snapshot(&g);
    assert_eq!(would(&g, spell, YOU), Speculation::Decided(false), "it destroys it only if its controller chooses to");
    assert_eq!(could(&g, spell, YOU), Speculation::Decided(true));
    let mut destroyed = Vec::new();
    assert_eq!(outcomes(&g, spell, |o| destroyed.push(destroyed_by(o, YOU))), 2);
    destroyed.sort();
    assert_eq!(destroyed, [false, true]);
    // Breaking off at the first outcome visits no other.
    let mut visits = 0;
    let came = speculate(&g, Target::Object(spell), |_| {
        visits += 1;
        ControlFlow::Break(visits)
    });
    assert_eq!((came, visits), (Speculation::Decided(1), 1));
    assert_eq!(snapshot(&g), before);
}

#[test]
fn chance_is_tried_every_way_and_takes_nothing_from_the_games_generator() {
    let mut g = game();
    let mine = put(&mut g, &FILLER, YOU);
    let coin = cast(&mut g, &COIN_DESTROY, THEM, &[Target::Object(mine)]);
    let before = snapshot(&g);
    assert_eq!(would(&g, coin, YOU), Speculation::Decided(false), "a random chance of destroying it is not \"would\"");
    assert_eq!(could(&g, coin, YOU), Speculation::Decided(true));
    assert_eq!(outcomes(&g, coin, |_| {}), 2);
    assert_eq!(snapshot(&g), before, "the generator and everything else as it was");
    let roll = cast(&mut g, &ROLL_DESTROY, THEM, &[Target::Object(mine)]);
    let mut gained = Vec::new();
    let n = outcomes(&g, roll, |o| gained.push(o.records.iter().find_map(|r| match r.event {
        Event::LifeChange { player, delta } if player == THEM => Some(delta),
        _ => None,
    })));
    assert_eq!(n, 6, "each face of the die");
    gained.sort();
    assert_eq!(gained, (1..=6).map(Some).collect::<Vec<_>>());
    assert_eq!(would(&g, roll, YOU), Speculation::Undecided, "it destroys it whatever the roll");
}

#[test]
fn a_payment_is_tried_paid_and_unpaid_once_for_each_way_of_paying() {
    let mut g = game();
    let mine = put(&mut g, &FILLER, YOU);
    let first = put(&mut g, &WASTE, YOU);
    let second = put(&mut g, &WASTE, YOU);
    let spell = cast(&mut g, &UNLESS_DESTROY, THEM, &[Target::Object(mine)]);
    let before = snapshot(&g);
    assert_eq!(would(&g, spell, YOU), Speculation::Decided(false), "its controller may pay");
    assert_eq!(could(&g, spell, YOU), Speculation::Decided(true), "or decline");
    let mut ways = Vec::new();
    let n = outcomes(&g, spell, |o| ways.push((destroyed_by(o, YOU), o.game.state().obj(first).tapped, o.game.state().obj(second).tapped)));
    ways.sort();
    ways.dedup();
    // Declined (destroyed; nothing tapped, or one or both lands tapped for mana that empties unspent), and paid with the
    // first land, the second, or both tapped: never one of them twice, whatever order they were tapped in.
    assert!(ways.contains(&(true, false, false)));
    assert!(ways.contains(&(false, true, false)));
    assert!(ways.contains(&(false, false, true)));
    assert!(ways.contains(&(false, true, true)));
    assert!(n <= 8, "{n} outcomes: the orders of tapping the two lands are one way of paying");
    assert_eq!(snapshot(&g), before, "nothing was tapped or paid in the game");
    g.state.update_obj(first, |o| o.tapped = true);
    g.state.update_obj(second, |o| o.tapped = true);
    assert_eq!(would(&g, spell, YOU), Speculation::Undecided, "with no mana to pay, it would");
}

#[test]
fn a_settled_way_is_visited_once_in_place_of_every_way_on_from_it() {
    let mut g = game();
    let mine = put(&mut g, &FILLER, YOU);
    let spell = cast(&mut g, &DESTROY_ASK, THEM, &[Target::Object(mine)]);
    let mut ended = Vec::new();
    assert_eq!(outcomes(&g, spell, |o| ended.push(o.ended)), 8, "three choices, each both ways");
    assert!(ended.iter().all(|&e| e));
    let mut visits = Vec::new();
    let came = speculate_settled(&g, Target::Object(spell), |o| destroyed_by(o, YOU), |o| {
        visits.push((o.ended, destroyed_by(o, YOU)));
        ControlFlow::<()>::Continue(())
    });
    assert_eq!(came, Speculation::Undecided);
    assert_eq!(visits, [(false, true)], "settled at the first question: the land is destroyed whatever is answered");
}

#[test]
fn a_resolution_with_more_ways_than_are_followed_is_unjudged_or_under_verification_a_panic() {
    let mut g = game();
    let spell = cast(&mut g, &MANY_WAYS, THEM, &[]);
    let before = snapshot(&g);
    let started = std::time::Instant::now();
    let came = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        speculate(&g, Target::Object(spell), |_| ControlFlow::<()>::Continue(()))
    }));
    eprintln!("2^18 ways, followed until the bound: {:?}", started.elapsed());
    match came {
        Ok(came) => {
            assert!(!crate::layers::verifying());
            assert_eq!(came, Speculation::Unjudged);
        }
        Err(_) => assert!(crate::layers::verifying(), "only verification panics at the bound"),
    }
    // Breaking off early never reaches the bound.
    assert_eq!(speculate(&g, Target::Object(spell), |_| ControlFlow::Break(())), Speculation::Decided(()));
    assert_eq!(snapshot(&g), before);
}

#[test]
fn a_graveyards_order_is_tried_only_where_the_resolving_object_reads_it() {
    let mut g = game();
    put(&mut g, &ORDER_READER, YOU);
    for _ in 0..4 {
        put(&mut g, &FILLER, YOU);
    }
    // The order its owner arranges four lands in (CR 404.3) is asked, since something in the game reads it, but nothing that
    // resolves here does: one outcome.
    let spell = cast(&mut g, &DESTROY_ALL, THEM, &[]);
    assert_eq!(outcomes(&g, spell, |o| assert!(destroyed_by(o, YOU))), 1);
    // A spell that reads it itself tells every order apart.
    let reading = cast(&mut g, &DESTROY_ALL_READING, THEM, &[]);
    let mut orders = Vec::new();
    assert_eq!(outcomes(&g, reading, |o| orders.push(o.game.state().zone(Zone::graveyard(YOU)).to_vec())), 24);
    orders.sort();
    orders.dedup();
    assert_eq!(orders.len(), 24, "each order of the four");
}

#[test]
fn a_shuffle_is_tried_only_for_the_places_the_resolution_reads() {
    let mut g = game();
    let mine = put(&mut g, &FILLER, YOU);
    let spell = cast(&mut g, &SHUFFLE_DESTROY, THEM, &[Target::Object(mine)]);
    assert_eq!(outcomes(&g, spell, |_| {}), 1, "nothing reads the order the shuffle left");
    assert_eq!(would(&g, spell, YOU), Speculation::Undecided);
    let draw = cast(&mut g, &SHUFFLE_DRAW, THEM, &[]);
    let library = g.state.zone(Zone::library(THEM)).to_vec();
    let before = snapshot(&g);
    let mut drawn = Vec::new();
    let n = outcomes(&g, draw, |o| drawn.extend(o.records.iter().find_map(|r| match r.event {
        Event::ZoneChange { old, from, .. } if from == Zone::library(THEM) => Some(old),
        _ => None,
    })));
    drawn.sort();
    let mut every = library.clone();
    every.sort();
    assert_eq!((n, drawn), (library.len(), every), "each card of the library is the one drawn in one outcome");
    assert_eq!(snapshot(&g), before);
}
