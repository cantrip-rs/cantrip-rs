//! "With this artifact" (Cyclopean Tomb): which counters an object put on, and which it took off, asked of the event history
//! - each `CountersPut` and `CountersRemoved` record's `Cause` names the source of the ability resolving, gone or not - and
//! "at the beginning of each of your upkeeps for the rest of the game", a delayed triggered ability made by a "when this is
//! put into a graveyard from the battlefield" ability (CR 603.7, `DelayedWhen::EachStep`), which triggers in every upkeep of
//! its controller's and never in another player's, with its source long gone. On a table of test-only cards, until
//! Cyclopean Tomb is added:
//!
//! - Probe Tomb and Other Tomb are artifacts with "{0}: Put a mire counter on target land. That land is a Swamp for as long
//!   as it has a mire counter on it" (`EffectKind::SetLandType`, `Duration::WhileCounter`), "Sacrifice this artifact: nothing"
//!   and Cyclopean Tomb's "when this is put into a graveyard from the battlefield, at the beginning of each of your upkeeps
//!   for the rest of the game, remove all mire counters from a land that a mire counter was put onto with this artifact but
//!   that a mire counter has not been removed from with this artifact" (the first such land: the card asks its player);
//! - Probe Mire is an instant that puts a mire counter on target land, a source that is no Tomb.

use std::collections::VecDeque;
use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

const YOU: PlayerId = PlayerId(0);

fn a_land(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Land))
}

static LAND: [TargetSpec; 1] = [target(a_land)];

fn the_land(item: &StackItem) -> ObjectId {
    match item.target(0) {
        Target::Object(id) => id,
        other => panic!("a land, not {other:?}"),
    }
}

fn mire(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(the_land(item), CounterKind::Mire, 1);
    Ok(())
}

/// "Put a mire counter on target land. That land is a Swamp for as long as it has a mire counter on it." (CR 305.7, 611.2b)
fn mire_and_swamp(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    mire(g, item)?;
    g.add_effect(item.object, the_land(item), EffectKind::SetLandType(Subtype::Swamp), Duration::WhileCounter(CounterKind::Mire));
    Ok(())
}

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// The lands a mire counter was put onto with `tomb` but that a mire counter has not been removed from with it, oldest first:
/// the records whose cause is an ability of `tomb` resolving, its activated one and its delayed one alike, whether `tomb` is
/// on the battlefield or gone. A land that has left is another object (CR 400.7), and none of these.
fn mired_with(q: &dyn Query, tomb: ObjectId) -> Vec<ObjectId> {
    let with_tomb = |r: &&Record| matches!(r.cause, Cause::Ability { source, .. } if source == tomb);
    let history = q.history();
    let mut lands: Vec<ObjectId> = Vec::new();
    for r in history.select(EventQuery::kind(EventMask::COUNTERS)).filter(with_tomb) {
        if let Event::CountersPut { object, kind: CounterKind::Mire, amount, .. } = r.event
            && amount > 0
            && !lands.contains(&object)
        {
            lands.push(object);
        }
    }
    let removed = |land: ObjectId| {
        history
            .select(EventQuery::kind(EventMask::COUNTERS).object(land))
            .filter(with_tomb)
            .any(|r| matches!(r.event, Event::CountersRemoved { kind: CounterKind::Mire, amount, .. } if amount > 0))
    };
    lands.retain(|&land| q.is_on_battlefield(land) && q.chars(land).is(CardType::Land) && !removed(land));
    lands
}

/// "Remove all mire counters from a land that a mire counter was put onto with this artifact but that a mire counter has not
/// been removed from with this artifact": its source is the Tomb as it last existed (CR 603.7).
fn unmire(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(&land) = mired_with(g, item.object).first() {
        let n = g.obj(land).counters(CounterKind::Mire);
        g.remove_counters(land, CounterKind::Mire, n);
    }
    Ok(())
}

/// "At the beginning of each of your upkeeps for the rest of the game, ..." (CR 603.7): a delayed triggered ability with the
/// Tomb that died as its source.
fn each_upkeep(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let card = g.obj(item.object).card;
    g.delayed_trigger(
        DelayedWhen::EachStep { step: Step::Upkeep, on_turn_of: Some(item.controller) },
        item.object,
        item.controller,
        AbilityRef::delayed(card, 0),
        Target::None,
    );
    Ok(())
}

static TOMB_ABILITIES: [AbilityDef; 3] = [
    ability(Cost::NONE, mire_and_swamp, "{0}: Put a mire counter on target land. That land is a Swamp for as long as it has a mire counter on it.")
        .targets(&LAND),
    ability(SACRIFICE_THIS, nothing, "Sacrifice this artifact: Nothing."),
    triggered(
        cantrip_mechanics::triggers::THIS_DIES,
        each_upkeep,
        "When this artifact is put into a graveyard from the battlefield, at the beginning of each of your upkeeps for the rest of the game, remove all mire counters from a land that a mire counter was put onto with this artifact but that a mire counter has not been removed from with this artifact.",
    ),
];

static TOMB_DELAYED: [AbilityDef; 1] = [delayed_trigger(
    unmire,
    "At the beginning of your upkeep, remove all mire counters from a land that a mire counter was put onto with this artifact but that a mire counter has not been removed from with this artifact.",
)];

const fn tomb(name: &'static str) -> CardDef {
    CardDef { name, types: TypeSet::of(&[CardType::Artifact]), abilities: &TOMB_ABILITIES, delayed: &TOMB_DELAYED, ..CardDef::BLANK }
}

static TOMB: CardDef = tomb("Probe Tomb");
static OTHER: CardDef = tomb("Other Tomb");
static MIRE: CardDef = CardDef { name: "Probe Mire", types: TypeSet::of(&[CardType::Instant]), spell: spell(&LAND, mire), ..CardDef::BLANK };

const fn land(name: &'static str) -> CardDef {
    CardDef { name, has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK }
}

static LAND_A: CardDef = land("Land A");
static LAND_B: CardDef = land("Land B");
static LAND_C: CardDef = land("Land C");
static FILLER: CardDef = land("Filler");

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&TOMB, &OTHER, &MIRE, &LAND_A, &LAND_B, &LAND_C, &FILLER]))
}

/// A two-player game: each seat's library is the named cards, then Fillers, in order.
fn deal(first: &[&str], second: &[&str]) -> Game {
    let list = |names: &[&str]| -> Vec<CardId> {
        let mut deck: Vec<CardId> = names.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect();
        deck.extend(std::iter::repeat_n(table().find("Filler").unwrap(), 40 - deck.len()));
        deck
    };
    let mut game = Game::new(table(), &Setup { decks: vec![list(first), list(second)], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    game.run();
    game
}

/// One thing a seat does when it has priority and can: play a land, cast a spell, or activate the `index`th ability of a
/// permanent, at the target named.
#[derive(Clone, Copy)]
enum Do {
    Play(&'static str),
    Cast(&'static str, &'static str),
    Activate(&'static str, u8, &'static str),
}

/// Plays on: each seat does its steps in order whenever it has priority and can, and passes otherwise. Stops when `until`
/// holds at a question.
fn play(game: &mut Game, steps: [&[Do]; 2], until: impl Fn(&Game) -> bool) {
    let mut plan: [VecDeque<Do>; 2] = [steps[0].iter().copied().collect(), steps[1].iter().copied().collect()];
    for _ in 0..2000 {
        game.run();
        if until(game) {
            assert!(plan.iter().all(VecDeque::is_empty), "every step was taken");
            return;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let seat = request.player.0 as usize;
        let answer = match &request.kind {
            RequestKind::Priority { options } => {
                let found = plan[seat].front().and_then(|step| {
                    let (kind, name, index, aimed) = match *step {
                        Do::Play(name) => (ActionKind::PlayLand, name, None, ""),
                        Do::Cast(name, aimed) => (ActionKind::CastSpell, name, None, aimed),
                        Do::Activate(name, index, aimed) => (ActionKind::ActivateAbility, name, Some(index), aimed),
                    };
                    let option = options.iter().find(|o| {
                        o.kind == kind && o.source.is_some_and(|s| game.name(s) == name) && index.is_none_or(|i| o.index == i)
                    })?;
                    Some((option, aimed))
                });
                match found {
                    Some((option, aimed)) => {
                        plan[seat].pop_front();
                        let mut action = Action::of(option);
                        if option.targets_required > 0 {
                            let found = option.target_candidates.iter().copied().find(|c| matches!(c, Target::Object(id) if game.name(*id) == aimed));
                            action.targets = vec![found.unwrap_or_else(|| panic!("no candidate named {aimed}"))];
                        }
                        Answer::Act(action)
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game did not reach the position");
}

fn on_battlefield(game: &Game, name: &str) -> ObjectId {
    game.state.battlefield().iter().copied().find(|&id| game.name(id) == name).unwrap_or_else(|| panic!("no {name} on the battlefield"))
}

fn mire_on(game: &Game, name: &str) -> i32 {
    game.state.obj(on_battlefield(game, name)).counters(CounterKind::Mire)
}

/// Whether Land A, Land B and Land C are Swamps.
fn swamps(game: &Game) -> [bool; 3] {
    ["Land A", "Land B", "Land C"].map(|n| game.chars(on_battlefield(game, n)).subtypes == SubtypeSet::NONE.with(Subtype::Swamp))
}

/// Settled in turn `n`'s draw step, past its upkeep and what triggered there.
fn in_draw_step_of(n: u32) -> impl Fn(&Game) -> bool {
    move |g: &Game| g.state.turn.number == n && g.state.turn.step() == Step::Draw && g.state.stack.is_empty()
}

/// The turns the Tomb's delayed ability triggered and resolved in.
fn triggered_in(game: &Game, tomb: ObjectId) -> Vec<u32> {
    game.log
        .iter()
        .filter(|r| matches!(r.event, Event::Resolved { source, ability, .. } if source == tomb && ability.is_delayed()))
        .map(|r| r.turn)
        .collect()
}

/// Probe Tomb puts mire counters on Land A (which Probe Mire puts one more on) and on the opponent's Land B; Other Tomb puts
/// one on Land C. With Probe Tomb sacrificed, its delayed ability triggers in each of its controller's upkeeps and never in
/// the opponent's: it takes both mire counters off Land A (CR 122.1: all of them, whatever put them there, the ruling), then
/// Land B's, then finds no such land, and Land C's counter - Other Tomb's - stays. What Probe Tomb did is asked of the history
/// after it has gone, by its object, which Other Tomb's actions don't answer for. Each land a Tomb mired is a Swamp until its
/// last mire counter comes off (CR 305.7, 611.2b), whatever has become of the Tomb.
#[test]
fn a_dead_tombs_upkeep_ability_takes_the_mire_counters_it_put_off_one_land_each_upkeep() {
    use Do::*;
    let mut game = deal(&["Land A", "Probe Tomb", "Other Tomb", "Probe Mire", "Land C"], &["Land B"]);
    play(
        &mut game,
        [
            &[
                Play("Land A"),
                Cast("Probe Tomb", ""),
                Cast("Other Tomb", ""),
                Activate("Probe Tomb", 0, "Land A"),
                Cast("Probe Mire", "Land A"),
                Play("Land C"),
                Activate("Probe Tomb", 0, "Land B"),
                Activate("Other Tomb", 0, "Land C"),
            ],
            &[Play("Land B")],
        ],
        |g| g.state.turn.number == 3 && g.state.stack.is_empty() && g.state.battlefield().iter().any(|&id| g.state.obj(id).counters(CounterKind::Mire) > 0 && g.name(id) == "Land C"),
    );
    let (tomb, other) = (on_battlefield(&game, "Probe Tomb"), on_battlefield(&game, "Other Tomb"));
    let [a, b, c] = ["Land A", "Land B", "Land C"].map(|n| on_battlefield(&game, n));
    assert_eq!((mire_on(&game, "Land A"), mire_on(&game, "Land B"), mire_on(&game, "Land C")), (2, 1, 1));
    assert_eq!(swamps(&game), [true, true, true]);
    assert_eq!(mired_with(&game, tomb), vec![a, b], "Probe Mire's counter is no Tomb's");
    assert_eq!(mired_with(&game, other), vec![c]);

    play(&mut game, [&[Activate("Probe Tomb", 1, "")], &[]], |g| g.state.stack.is_empty() && !g.state.battlefield().iter().any(|&id| g.name(id) == "Probe Tomb"));
    assert_eq!(game.state.turn.number, 3);
    assert_eq!(mired_with(&game, tomb), vec![a, b], "the history answers for the Tomb that has gone");
    assert_eq!(swamps(&game), [true, true, true], "the Swamps are not tied to the Tomb (the ruling)");
    assert!(triggered_in(&game, tomb).is_empty());

    play(&mut game, [&[], &[]], in_draw_step_of(4));
    assert!(triggered_in(&game, tomb).is_empty(), "not in the opponent's upkeep");
    assert_eq!(mire_on(&game, "Land A"), 2);

    play(&mut game, [&[], &[]], in_draw_step_of(5));
    assert_eq!(triggered_in(&game, tomb), vec![5]);
    assert_eq!((mire_on(&game, "Land A"), mire_on(&game, "Land B"), mire_on(&game, "Land C")), (0, 1, 1));
    assert_eq!(swamps(&game), [false, true, true], "Land A has no mire counter, and is itself again");
    assert_eq!(mired_with(&game, tomb), vec![b]);

    play(&mut game, [&[], &[]], in_draw_step_of(7));
    assert_eq!(triggered_in(&game, tomb), vec![5, 7]);
    assert_eq!((mire_on(&game, "Land A"), mire_on(&game, "Land B"), mire_on(&game, "Land C")), (0, 0, 1));
    assert_eq!(swamps(&game), [false, false, true]);
    assert!(mired_with(&game, tomb).is_empty());

    play(&mut game, [&[], &[]], in_draw_step_of(9));
    assert_eq!(triggered_in(&game, tomb), vec![5, 7, 9], "for the rest of the game, with nothing left to do");
    assert_eq!(mire_on(&game, "Land C"), 1, "Other Tomb's counter stays");
    assert_eq!(swamps(&game), [false, false, true]);
    assert_eq!(mired_with(&game, other), vec![c]);
    // Each counter that came off is recorded, with the Tomb's delayed ability as its cause.
    let removed: Vec<(ObjectId, i32)> = game
        .log
        .iter()
        .filter_map(|r| match (r.cause, r.event) {
            (Cause::Ability { source, .. }, Event::CountersRemoved { object, kind: CounterKind::Mire, amount, .. }) if source == tomb => Some((object, amount)),
            _ => None,
        })
        .collect();
    assert_eq!(removed, vec![(a, 2), (b, 1)]);
    assert_eq!(game.state.player(YOU).life, 20);
}
