//! The players' inherent triggered abilities - the monarch's, the initiative's and speed's (CR 725.2, 726.2, 702.179d) - found
//! from events like any triggered ability (`designations::inherent_triggers`, `settle::inherent`), over test cards: a rule that
//! stops every triggered ability stops each of them, and leaves nothing behind; the initiative's "one or more creatures"
//! triggers once for one player's creatures; speed's once each turn, a stopped one not counting; and the initiative passes on
//! when its holder leaves the game (CR 726.4).

use crate::{Cx, Game, Setup};
use cantrip_api::intrinsic::{INITIATIVE_TAKEN, INITIATIVE_VENTURE, MONARCH_TAKEN, SPEED_UP};
use cantrip_api::*;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// "Triggered abilities don't trigger."
fn nothing_triggers(_: &dyn Query, _: &Asker, _: &Event, _: &Triggering) -> bool {
    true
}

static HUSH: CardDef = CardDef {
    name: "Inherent Test Hush",
    types: TypeSet::of(&[CardType::Artifact]),
    trigger_rules: &[nothing_triggers],
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef {
    name: "Inherent Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Inherent Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&HUSH, &BEAR, &FILLER])))
}

/// A game of `players` with nothing in it, the first seat active.
fn game(players: usize) -> Game {
    Game::new(table(), &Setup { decks: vec![Vec::new(); players], ..Default::default() })
}

fn put(game: &mut Game, card: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(card), controller, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

/// The inherent triggers waiting to be put on the stack: each one's index and controller.
fn waiting(game: &Game) -> Vec<(u8, PlayerId)> {
    let intrinsic = game.cards.intrinsic;
    game.state.triggers.iter().filter(|t| t.ability.card == intrinsic).map(|t| (t.ability.index, t.controller)).collect()
}

fn combat_damage(source: ObjectId, victim: PlayerId) -> Event {
    Event::Damage { source, target: Target::Player(victim), amount: 2, kind: DamageKind::new(true, ColorSet::default()), controller: A, recipient_excess: 0, known: u32::MAX }
}

/// Each of the inherent abilities, with and without the Hush: without, it triggers; with, it doesn't, and the object of the
/// rules' own it would have come from is gone again.
#[test]
fn a_rule_that_stops_triggered_abilities_stops_the_inherent_ones() {
    for hush in [false, true] {
        let cases: [(&str, fn(&mut Game) -> Vec<Event>, (u8, PlayerId)); 5] = [
            ("the monarch's combat damage", |g| {
                g.state.update_player(B, |p| p.monarch = true);
                let bear = put(g, &BEAR, A);
                vec![combat_damage(bear, B)]
            }, (MONARCH_TAKEN, B)),
            ("the initiative's combat damage", |g| {
                g.state.update_player(B, |p| p.initiative = true);
                let bear = put(g, &BEAR, A);
                vec![combat_damage(bear, B)]
            }, (INITIATIVE_TAKEN, B)),
            ("the initiative's upkeep", |g| {
                g.state.update_player(A, |p| p.initiative = true);
                vec![Event::StepBegan { step: Step::Upkeep }]
            }, (INITIATIVE_VENTURE, A)),
            ("taking the initiative", |_| vec![Event::TookInitiative { player: A }], (INITIATIVE_VENTURE, A)),
            ("an opponent losing life", |g| {
                g.state.update_player(A, |p| p.speed = Some(1));
                vec![Event::LifeChange { player: B, delta: -1 }]
            }, (SPEED_UP, A)),
        ];
        for (name, setup, expected) in cases {
            let mut g = game(2);
            if hush {
                put(&mut g, &HUSH, B);
            }
            let events = setup(&mut g);
            let objects = g.state.objects.len();
            cx(&mut g).emit_batch(&events, None);
            if hush {
                assert_eq!(waiting(&g), Vec::new(), "{name}: the Hush stopped nothing");
                assert_eq!(g.state.objects.len(), objects, "{name}: a stopped ability left its source behind");
            } else {
                assert_eq!(waiting(&g), vec![expected], "{name}: the ability did not trigger");
            }
        }
    }
}

/// Two of one player's creatures dealing combat damage to the player with the initiative together trigger its "one or more
/// creatures" ability once (CR 726.2, 603.2c).
#[test]
fn two_creatures_of_one_player_take_the_initiative_once() {
    let mut g = game(2);
    g.state.update_player(B, |p| p.initiative = true);
    let (one, two) = (put(&mut g, &BEAR, A), put(&mut g, &BEAR, A));
    cx(&mut g).emit_batch(&[combat_damage(one, B), combat_damage(two, B)], None);
    assert_eq!(waiting(&g), vec![(INITIATIVE_TAKEN, B)]);
}

/// Speed's increase triggers only once each turn (CR 702.179d): a second loss of life that turn triggers nothing. But one the
/// Hush stopped did not trigger, so once the Hush is gone the next loss of life triggers it.
#[test]
fn speed_triggers_once_each_turn_and_a_stopped_one_does_not_count() {
    let loss = Event::LifeChange { player: B, delta: -1 };
    let mut g = game(2);
    g.state.update_player(A, |p| p.speed = Some(1));
    cx(&mut g).emit(loss);
    cx(&mut g).emit(loss);
    assert_eq!(waiting(&g), vec![(SPEED_UP, A)], "speed's increase did not trigger exactly once");

    let mut g = game(2);
    g.state.update_player(A, |p| p.speed = Some(1));
    let hush = put(&mut g, &HUSH, B);
    cx(&mut g).emit(loss);
    assert_eq!(waiting(&g), Vec::new(), "the Hush stopped nothing");
    g.state.zone_remove(Zone::BATTLEFIELD, hush);
    cx(&mut g).emit(loss);
    assert_eq!(waiting(&g), vec![(SPEED_UP, A)], "a stopped trigger counted as speed's one this turn");
}

/// The player with the initiative leaves the game on another's turn: the active player takes it (CR 726.4), and ventures into
/// Undercity as any player who takes it does (CR 726.2). If the active player is the one who leaves, the next player in turn
/// order takes it.
#[test]
fn the_initiative_passes_on_when_its_holder_leaves() {
    let mut g = game(3);
    g.state.update_player(B, |p| p.initiative = true);
    crate::ops::lose_game(&mut cx(&mut g), B);
    assert!(g.state.player(A).initiative, "the active player did not take the initiative");
    assert!(!g.state.player(B).initiative, "the player who left still has the initiative");
    assert_eq!(waiting(&g), vec![(INITIATIVE_VENTURE, A)], "taking it did not venture");

    let mut g = game(3);
    g.state.update_player(A, |p| p.initiative = true);
    crate::ops::lose_game(&mut cx(&mut g), A);
    assert!(g.state.player(B).initiative, "the next player in turn order did not take the initiative");
    assert!(!g.state.player(C).initiative && !g.state.player(A).initiative);
    assert_eq!(waiting(&g), vec![(INITIATIVE_VENTURE, B)], "taking it did not venture");
}

/// Two players lose in one check of state-based actions on the turn of one of them (CR 704.3, 800.4a): the monarch and the
/// initiative, held by the other, pass to the next player in turn order still in the game, and never on the way to the
/// active player who is leaving too - nothing records that player becoming the monarch or taking the initiative, which a
/// Palace Jailer of an opponent would see (CR 725.4, 726.4).
#[test]
fn designations_pass_once_every_loser_of_a_check_has_left() {
    let mut g = game(4);
    // The empty libraries were drawn from for the opening hands: that is no loss here.
    for p in 0..4 {
        g.state.update_player(PlayerId(p), |p| p.drew_from_empty = false);
    }
    g.state.update_turn(|t| t.active = B);
    g.state.update_player(A, |p| {
        p.monarch = true;
        p.initiative = true;
        p.life = 0;
    });
    g.state.update_player(B, |p| p.life = 0);
    crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap();
    assert!(g.state.player(A).lost && g.state.player(B).lost);
    assert!(g.state.player(C).monarch, "the next player in the game did not become the monarch");
    assert!(g.state.player(C).initiative, "the next player in the game did not take the initiative");
    let passed_through_b = g.log.iter().any(|r| {
        matches!(r.event, Event::BecameMonarch { player } | Event::TookInitiative { player } if player == B)
    });
    assert!(!passed_through_b, "a player who lost in the same check was passed a designation");
}

/// A player wins (CR 104.2b) while an opponent is the monarch on another opponent's turn: every opponent has lost before the
/// designation passes, so it goes to the winner, never on the way to the active opponent (CR 725.4).
#[test]
fn a_win_passes_the_monarch_once_every_opponent_has_lost() {
    let mut g = game(3);
    for p in 0..3 {
        g.state.update_player(PlayerId(p), |p| p.drew_from_empty = false);
    }
    g.state.update_turn(|t| t.active = C);
    g.state.update_player(B, |p| p.monarch = true);
    crate::ops::win_game(&mut cx(&mut g), A);
    assert!(g.state.player(A).monarch, "the winner did not become the monarch");
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::BecameMonarch { player } if player == C)), "a losing opponent became the monarch");
}

/// A player conceding in a game of three while another is asked at the opening priority (CR 104.3a, 800.4a): they leave at
/// once - their permanent leaves, and the monarch and the initiative they held pass to the active player (CR 725.4, 726.4) -
/// and the game goes on without them; answering on does not wind the concession back.
#[test]
fn a_concession_leaves_the_game_at_once_and_stays() {
    let table = table();
    let filler = table.id_of(&FILLER);
    let mut g = Game::new(table, &Setup { decks: vec![vec![filler; 20]; 3], ..Default::default() });
    g.state.update_player(B, |p| {
        p.monarch = true;
        p.initiative = true;
    });
    let bear = put(&mut g, &BEAR, B);
    g.run();
    let asked = g.pending().cloned().expect("a question");
    assert_ne!(asked.player, B, "the test wants another player asked");
    g.concede(B);
    assert!(g.state.player(B).lost, "the concession did not take");
    assert!(!g.state.battlefield().contains(&bear), "the conceding player's permanent stayed");
    assert!(g.state.player(A).monarch && g.state.player(A).initiative, "the active player did not take the designations");
    assert!(g.log.iter().any(|r| matches!(r.event, Event::GameLost { player, reason: LossReason::Conceded } if player == B)));
    for _ in 0..20 {
        let Some(request) = g.pending().cloned() else { break };
        assert_ne!(request.player, B, "the conceding player was asked");
        let answer = match request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            _ => break,
        };
        g.answer(request.id, request.player, answer).unwrap();
        g.run();
    }
    assert!(g.state.player(B).lost && g.state.player(A).monarch, "answering wound the concession back");
    assert!(!g.state.is_over(), "two players are left");
}
