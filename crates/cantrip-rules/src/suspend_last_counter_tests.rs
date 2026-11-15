//! Suspend's last-counter ability (CR 702.62a), over test cards: "when the last time counter is removed from this card, if
//! it's exiled, you may play it without paying its mana cost" triggers on the last time counter coming off however it does
//! - an effect (Timecrafting, Clockspinning, Rift Elemental) as well as suspend's own upkeep ability - once however many
//! came off at once (CR 603.2), never for a removal that leaves some, never from a card that has no suspend, and never from a
//! card with suspend outside exile, where the ability does not function. Suspend is the object's (CR 702.62b), through the
//! layers: a card that gains it in exile (Delay, Epochrasite) is suspended and cast. A Siege's "when the last defense counter
//! is removed" (CR 310.12b) is the layers' too: one that has lost its abilities has none to trigger.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Last Counter Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A creature with suspend 3.
static SPRINTER: CardDef = CardDef {
    name: "Last Counter Test Sprinter",
    cost: generic(3),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    suspend: Some(SuspendDef { count: 3, cost: ManaCost::ZERO }),
    ..CardDef::BLANK
};

/// The same creature with no suspend: time counters on it in exile are only counters.
static PLAIN: CardDef = CardDef {
    name: "Last Counter Test Plain",
    cost: generic(3),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// "Remove `N` time counters from each card your opponent owns in exile."
fn remove_time_counters<const N: i32>(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = g.state().zone(Zone::exile(THEM)).iter().copied().collect();
    for card in cards {
        g.remove_counters(card, CounterKind::Time, N);
    }
    Ok(())
}

static TICK: CardDef = CardDef {
    name: "Last Counter Test Tick",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], remove_time_counters::<1>),
    ..CardDef::BLANK
};

/// "Remove a time counter from each permanent your opponent controls."
fn remove_a_time_counter_from_their_permanents(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    let permanents: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.obj(id).controller == THEM).collect();
    for permanent in permanents {
        g.remove_counters(permanent, CounterKind::Time, 1);
    }
    Ok(())
}

static WIND_DOWN: CardDef = CardDef {
    name: "Last Counter Test Wind Down",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], remove_a_time_counter_from_their_permanents),
    ..CardDef::BLANK
};

static SPIN: CardDef = CardDef {
    name: "Last Counter Test Spin",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], remove_time_counters::<5>),
    ..CardDef::BLANK
};

/// "Each card your opponent owns in exile gains suspend" (Delay's "if it doesn't have suspend, it gains suspend",
/// Epochrasite's "it gains suspend"): for as long as it stays there.
fn give_suspend(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = g.state().zone(Zone::exile(THEM)).iter().copied().collect();
    for card in cards {
        g.add_effect(item.object, card, EffectKind::AddKeywords(Keywords::of(&[Keyword::Suspend])), Duration::Indefinite);
    }
    Ok(())
}

static DELAY: CardDef = CardDef {
    name: "Last Counter Test Delay",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], give_suspend),
    ..CardDef::BLANK
};

/// A Siege with a creature back face.
static SIEGE: CardDef = CardDef {
    name: "Last Counter Test Siege",
    types: TypeSet::of(&[CardType::Battle]),
    subtypes: SubtypeSet::of(&[Subtype::Siege]),
    defense: Some(3),
    back: Some(&SIEGE_BACK),
    ..CardDef::BLANK
};
static SIEGE_BACK: CardDef = CardDef {
    name: "Last Counter Test Siege Back",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

fn battles(g: &dyn cantrip_api::Game) -> Vec<ObjectId> {
    g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Battle)).collect()
}

/// "Remove a defense counter from each battle."
fn remove_a_defense_counter(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    for battle in battles(g) {
        g.remove_counters(battle, CounterKind::Defense, 1);
    }
    Ok(())
}

static BREACH: CardDef = CardDef {
    name: "Last Counter Test Breach",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], remove_a_defense_counter),
    ..CardDef::BLANK
};

/// "Each battle loses all abilities, then remove a defense counter from each battle."
fn strip_then_remove_a_defense_counter(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    for battle in battles(g) {
        g.add_effect(item.object, battle, EffectKind::LoseAllAbilities, Duration::Indefinite);
    }
    remove_a_defense_counter(g, item)
}

static STRIP_AND_BREACH: CardDef = CardDef {
    name: "Last Counter Test Strip And Breach",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], strip_then_remove_a_defense_counter),
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// Your turn about to reach its upkeep: `def` in their exile with `time` time counters - so no upkeep ability of its own
/// removes one this turn (CR 702.62a: "your upkeep") - and `instant` in your hand.
fn game(def: &'static CardDef, time: i32, instant: &'static CardDef) -> (Game, ObjectId, ObjectId) {
    game_in(Zone::exile(THEM), def, time, instant)
}

/// `game`, with `def` in `zone` instead.
fn game_in(zone: Zone, def: &'static CardDef, time: i32, instant: &'static CardDef) -> (Game, ObjectId, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &FILLER,
        &SPRINTER,
        &PLAIN,
        &TICK,
        &SPIN,
        &WIND_DOWN,
        &DELAY,
        &SIEGE,
        &SIEGE_BACK,
        &BREACH,
        &STRIP_AND_BREACH,
    ])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::Untap);
    });
    let card = g.state.push_obj(Obj::new(table.id_of(def), THEM, zone));
    g.state.zone_push(zone, card);
    crate::counters::put(&mut cx(&mut g), card, CounterKind::Time, time, THEM, crate::counters::How::Effect);
    let spell = g.state.push_obj(Obj::new(table.id_of(instant), YOU, Zone::hand(YOU)));
    g.state.zone_push(Zone::hand(YOU), spell);
    (g, card, spell)
}

/// Plays on - you casting `instant` at your first priority, casting the suspended card when asked, everyone else passing -
/// until `done` says so.
fn play_until(g: &mut Game, instant: ObjectId, done: impl Fn(&Game) -> bool) {
    for _ in 0..200 {
        g.run();
        if done(g) {
            return;
        }
        let r = g.pending().cloned().expect("a question");
        let answer = match &r.kind {
            RequestKind::Priority { options } if r.player == YOU && in_hand(g, instant) => options
                .iter()
                .find(|o| o.source == Some(instant))
                .map_or(Answer::Pass, |o| Answer::Act(Action { targets: o.target_candidates.clone(), ..Action::of(o) })),
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::May { purpose: Purpose::CastFree, .. } => Answer::Yes,
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(r.id, r.player, answer).expect("a legal answer");
    }
    panic!("never got there");
}

fn on_battlefield(g: &Game, def: &'static CardDef) -> Option<ObjectId> {
    g.state.battlefield().iter().copied().find(|&id| g.state.obj(id).card == g.cards.id_of(def))
}

/// Suspend's last-counter abilities waiting to go on the stack or on it.
fn cast_triggers(g: &Game) -> usize {
    let ability = AbilityRef { card: g.cards.intrinsic, index: cantrip_api::intrinsic::SUSPEND_CAST };
    g.state.triggers.iter().filter(|t| t.ability == ability).count() + g.state.stack.iter().filter(|i| i.ability == ability).count()
}

fn in_hand(g: &Game, card: ObjectId) -> bool {
    g.state.zone(Zone::hand(YOU)).contains(&card)
}

/// The instant, cast in your upkeep, has resolved, and nothing it triggered is waiting or on the stack - still in your turn,
/// before any upkeep of theirs.
fn settled(instant: ObjectId) -> impl Fn(&Game) -> bool {
    move |g: &Game| g.state.turn.active == YOU && !in_hand(g, instant) && g.state.stack.is_empty() && g.state.triggers.is_empty()
}

/// An effect removes the last time counter from a suspended creature card on its owner's opponent's turn: its owner casts
/// it, and it gains haste as suspend says (CR 702.62a).
#[test]
fn an_effect_removing_the_last_time_counter_casts_the_card() {
    let (mut g, card, tick) = game(&SPRINTER, 1, &TICK);
    play_until(&mut g, tick, |g| on_battlefield(g, &SPRINTER).is_some());
    assert_eq!(g.state.turn.active, YOU, "cast on your turn: no upkeep of its owner's removed the counter");
    let sprinter = on_battlefield(&g, &SPRINTER).expect("cast and resolved");
    assert!(!g.state.zone(Zone::exile(THEM)).contains(&card), "the exiled card left exile");
    assert_eq!(g.state.obj(sprinter).controller, THEM, "cast by its owner");
    assert!(cx(&mut g).chars(sprinter).has(Keyword::Haste), "a creature cast this way has haste");
}

/// Three time counters taken off at once, down to none (CR 122.1): one "last counter removed", so one trigger.
#[test]
fn several_counters_removed_at_once_trigger_once() {
    let (mut g, _, spin) = game(&SPRINTER, 3, &SPIN);
    play_until(&mut g, spin, |g| cast_triggers(g) > 0);
    assert_eq!(cast_triggers(&g), 1, "one trigger for the counters that came off together");
    play_until(&mut g, spin, |g| on_battlefield(g, &SPRINTER).is_some());
    assert_eq!(g.state.turn.active, YOU);
    assert_eq!(cast_triggers(&g), 0, "no second trigger");
}

/// One of three taken off: the last is still on it, and nothing triggers.
#[test]
fn a_removal_that_leaves_some_triggers_nothing() {
    let (mut g, card, tick) = game(&SPRINTER, 3, &TICK);
    play_until(&mut g, tick, settled(tick));
    assert!(g.state.zone(Zone::exile(THEM)).contains(&card), "still exiled, never cast");
    assert_eq!(g.state.obj(card).counters(CounterKind::Time), 2);
    assert!(on_battlefield(&g, &SPRINTER).is_none());
}

/// A card with no suspend: its last time counter coming off in exile triggers no suspend ability (CR 702.62a, 702.62b).
#[test]
fn a_card_without_suspend_triggers_nothing() {
    let (mut g, card, tick) = game(&PLAIN, 1, &TICK);
    play_until(&mut g, tick, settled(tick));
    assert!(g.state.zone(Zone::exile(THEM)).contains(&card), "still exiled, never cast");
    assert_eq!(g.state.obj(card).counters(CounterKind::Time), 0, "the counter did come off");
    assert!(on_battlefield(&g, &PLAIN).is_none());
}

/// A permanent with suspend, its last time counter taken off on the battlefield: suspend's last-counter ability functions
/// only in exile (CR 702.62a), so nothing triggers and the permanent stays where it is.
#[test]
fn a_permanent_with_suspend_on_the_battlefield_triggers_nothing() {
    let (mut g, card, wind_down) = game_in(Zone::BATTLEFIELD, &SPRINTER, 1, &WIND_DOWN);
    // Just after the instant has resolved, as the next player gets priority: anything it triggered is on the stack.
    let resolved = |g: &Game| !in_hand(g, wind_down) && !g.state.stack.iter().any(|i| g.state.obj(i.object).card == g.cards.id_of(&WIND_DOWN));
    play_until(&mut g, wind_down, resolved);
    assert_eq!(g.state.obj(card).counters(CounterKind::Time), 0, "the counter did come off");
    assert_eq!(cast_triggers(&g), 0, "no last-counter ability triggered");
    assert_eq!(on_battlefield(&g, &SPRINTER), Some(card), "still the same permanent, never cast");
}

/// A Siege with one defense counter, which `instant` takes off: whether its defeat ability triggered (CR 310.12b), the
/// counter having come off.
fn siege_defeat_triggers(instant: &'static CardDef) -> usize {
    let (mut g, siege, instant) = game_in(Zone::BATTLEFIELD, &SIEGE, 0, instant);
    // Their Siege, which you protect (CR 310.11a).
    g.state.update_obj(siege, |o| o.marks.protector = Some(YOU));
    crate::counters::put(&mut cx(&mut g), siege, CounterKind::Defense, 1, THEM, crate::counters::How::Effect);
    let id = g.state.obj(instant).card;
    let resolved = move |g: &Game| !in_hand(g, instant) && !g.state.stack.iter().any(|i| g.state.obj(i.object).card == id);
    play_until(&mut g, instant, resolved);
    assert_eq!(g.state.obj(siege).counters(CounterKind::Defense), 0, "the counter did come off");
    let ability = AbilityRef { card: g.cards.intrinsic, index: cantrip_api::intrinsic::SIEGE_DEFEAT };
    g.state.triggers.iter().filter(|t| t.ability == ability).count() + g.state.stack.iter().filter(|i| i.ability == ability).count()
}

/// A Siege's last defense counter coming off triggers its defeat ability, which the layers give it (CR 310.12b, 613.1f).
#[test]
fn a_sieges_last_defense_counter_coming_off_triggers_its_defeat() {
    assert_eq!(siege_defeat_triggers(&BREACH), 1);
}

/// A Siege that has lost all its abilities has no defeat ability to trigger (CR 613.1f): its last defense counter coming off
/// does nothing.
#[test]
fn a_siege_that_lost_its_abilities_triggers_no_defeat() {
    assert_eq!(siege_defeat_triggers(&STRIP_AND_BREACH), 0);
}

/// A card with no suspend of its own gains it in exile, with two time counters on it (Delay, Epochrasite): it is suspended
/// (CR 702.62b), so its owner's upkeep takes a counter off, and the next one the last, and its owner casts it (CR 702.62a) -
/// suspend is the object's, through the layers, not its card's.
#[test]
fn a_card_that_gains_suspend_in_exile_is_suspended_and_cast() {
    let (mut g, card, delay) = game(&PLAIN, 2, &DELAY);
    play_until(&mut g, delay, |g| g.state.turn.active == THEM && g.state.obj(card).counters(CounterKind::Time) < 2);
    assert!(g.state.zone(Zone::exile(THEM)).contains(&card), "still exiled");
    assert_eq!(g.state.obj(card).counters(CounterKind::Time), 1, "its owner's upkeep took one counter off");
    let first = g.state.turn.number;
    play_until(&mut g, delay, |g| on_battlefield(g, &PLAIN).is_some());
    let plain = on_battlefield(&g, &PLAIN).expect("cast and resolved");
    assert_eq!(g.state.turn.active, THEM, "cast in its owner's turn");
    assert!(g.state.turn.number > first, "in its owner's next upkeep");
    assert_eq!(g.state.obj(plain).controller, THEM, "cast by its owner");
    assert!(cx(&mut g).chars(plain).has(Keyword::Haste), "a creature cast this way has haste");
}
