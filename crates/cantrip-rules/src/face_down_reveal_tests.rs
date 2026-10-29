//! CR 708.9: face-down permanents and face-down spells are revealed to all players when their owner leaves the game, and
//! every one left is revealed at the end of the game - recorded in the history (`Event::Revealed`, by the owner) before the
//! objects cease to exist and before the game's end is final, and seen by every seat from then on (`visibility::knows_face`,
//! tested at the table). Over test cards: a morph creature cast face down and manifested ones.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

static FILLER: CardDef = CardDef { name: "Reveal Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Morph {3}{W}": a 4/5 for {4}{W}.
static SKYWARDEN: CardDef = CardDef {
    name: "Reveal Test Skywarden",
    cost: mana("{4}{W}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(5),
    text: "Morph {3}{W}",
    ways: &[face_down_way(WayKind::Morph, Cost::mana(mana("{3}{W}")), "Morph {3}{W}")],
    ..CardDef::BLANK
};

/// A vanilla 3/3, to manifest.
static OGRE: CardDef =
    CardDef { name: "Reveal Test Ogre", cost: generic(3), types: TypeSet::of(&[CardType::Creature]), power: Some(3), toughness: Some(3), ..CardDef::BLANK };

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// A game of `players`, `active`'s main phase with priority and an empty stack.
fn game(players: usize, active: PlayerId) -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &SKYWARDEN, &OGRE])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: (0..players).map(|_| deck()).collect(), ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = active;
        t.priority = active;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    id
}

/// An Ogre of `owner`'s manifested (CR 701.40a) under `controller`: a face-down permanent.
fn manifested(g: &mut Game, owner: PlayerId, controller: PlayerId) -> ObjectId {
    let card = put(g, &OGRE, owner, Zone::library(owner));
    let permanent = cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(g), card, controller, FaceDown::Manifested, false)
        .expect("nothing is asked")
        .expect("it enters face down");
    assert!(g.state.obj(permanent).face_down);
    permanent
}

/// The active player casts a Skywarden face down (CR 702.37c), paying {3}: the face-down spell, left on the stack.
fn face_down_spell(g: &mut Game) -> ObjectId {
    let caster = g.state.turn.active;
    let card = put(g, &SKYWARDEN, caster, Zone::hand(caster));
    let source = g.state.zone(Zone::library(caster))[0];
    g.state.add_mana(caster, Mana::C, 3, source, None, ManaLasts::Step);
    if let Some(at) = g.state.agenda.len().checked_sub(1) {
        g.state.update_frame(at, |f| f.pending = None);
    }
    g.run();
    let r = g.pending().cloned().expect("a question");
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == 1).expect("the morph cast").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        g.run();
        let r = g.pending().cloned().expect("a question");
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    let spell = g.state.stack.last().expect("the spell is on the stack").object;
    assert!(g.state.obj(spell).face_down);
    spell
}

/// Who revealed `card`, each time it was revealed.
fn reveals(g: &Game, card: ObjectId) -> Vec<PlayerId> {
    g.log
        .iter()
        .filter_map(|r| match r.event {
            Event::Revealed { player, card: c } if c == card => Some(player),
            _ => None,
        })
        .collect()
}

/// Whether every seat has seen what `id`, a face-down object that has ceased to exist, was (`visibility::knows_face`).
fn seen_by_all(g: &Game, id: ObjectId) -> bool {
    g.state.seats().all(|p| g.state.obj(id).seen_by & (1 << p.0) != 0)
}

/// CR 708.9, 104.3a, 104.2a: in a duel, a concession ends the game, which reveals every face-down permanent and spell - the
/// conceder's and the winner's - each once, by its owner, where it is: the last player standing wins at once, so nothing
/// leaves with the conceder (`ops::leave_game`). Both ways round: the one conceding with the spell on the stack, and the one
/// conceding against it.
#[test]
fn a_concession_that_ends_a_duel_reveals_every_face_down_permanent_and_spell() {
    for conceder in [P0, P1] {
        let mut g = game(2, P0);
        let mine = manifested(&mut g, P0, P0);
        let theirs = manifested(&mut g, P1, P1);
        let spell = face_down_spell(&mut g);
        g.concede(conceder);
        assert!(g.state.is_over(), "conceder {conceder:?}");
        for (id, owner) in [(mine, P0), (spell, P0), (theirs, P1)] {
            assert_eq!(reveals(&g, id), vec![owner], "conceder {conceder:?}: {id:?}");
            assert!(g.state.obj(id).face_down && !g.state.obj(id).zone.is(ZoneKind::Gone), "conceder {conceder:?}: {id:?}");
        }
    }
}

/// CR 708.9, 800.4a: in a game of three, the player who concedes with the turn reveals the face-down spell they cast and the
/// face-down permanents they own - one another player controls among them - as they leave; the game goes on, and nothing of
/// anyone else's is revealed. One they control and another owns is exiled, revealed as it leaves the battlefield.
#[test]
fn a_player_leaving_a_game_that_goes_on_reveals_what_they_own_face_down() {
    let mut g = game(3, P1);
    let theirs = manifested(&mut g, P1, P1);
    let theirs_held = manifested(&mut g, P1, P2);
    let borrowed = manifested(&mut g, P0, P1);
    let others = [manifested(&mut g, P0, P0), manifested(&mut g, P2, P2)];
    let spell = face_down_spell(&mut g);
    assert_eq!(g.state.obj(spell).owner, P1);
    g.concede(P1);
    assert!(!g.state.is_over());
    for id in [theirs, theirs_held, spell] {
        assert_eq!(reveals(&g, id), vec![P1], "{id:?}");
        assert!(g.state.obj(id).zone.is(ZoneKind::Gone), "{id:?}");
        assert!(seen_by_all(&g, id), "{id:?}");
    }
    let exiled = g.state.obj(borrowed).became.expect("exiled");
    assert!(g.state.obj(exiled).zone.is(ZoneKind::Exile) && !g.state.obj(exiled).face_down);
    assert_eq!(reveals(&g, exiled), vec![P0]);
    for id in others {
        assert!(reveals(&g, id).is_empty(), "{id:?}");
        assert!(g.state.obj(id).face_down, "{id:?}");
    }
}

/// CR 708.9, 104.2b: a game that ends some other way - an effect saying a player wins - reveals every face-down permanent
/// and spell still in it, each by its owner, whoever controls it.
#[test]
fn the_end_of_the_game_reveals_every_face_down_permanent_and_spell() {
    let mut g = game(2, P0);
    let mine = manifested(&mut g, P0, P0);
    let theirs = manifested(&mut g, P1, P1);
    let stolen = manifested(&mut g, P1, P0);
    let spell = face_down_spell(&mut g);
    crate::ops::win_game(&mut cx(&mut g), P1);
    assert_eq!(g.state.outcome, Outcome::Won(P1));
    for (id, owner) in [(mine, P0), (theirs, P1), (stolen, P1), (spell, P0)] {
        assert_eq!(reveals(&g, id), vec![owner], "{id:?}");
        assert!(g.state.obj(id).face_down, "{id:?}");
    }
}

/// CR 708.9, 104.3b, 104.4a, 704.5a: a game the state-based actions end reveals every face-down permanent and spell, each by
/// its owner - won by the one player left at more than 0 life, or drawn when both are at 0 together.
#[test]
fn a_game_the_state_based_actions_end_reveals_every_face_down_permanent_and_spell() {
    for (dead, outcome) in [(vec![P1], Outcome::Won(P0)), (vec![P0, P1], Outcome::Draw)] {
        let mut g = game(2, P0);
        let mine = manifested(&mut g, P0, P0);
        let theirs = manifested(&mut g, P1, P1);
        let spell = face_down_spell(&mut g);
        for &p in &dead {
            g.state.update_player(p, |pl| pl.life = 0);
        }
        crate::settle::state_based_actions(&mut cx(&mut g)).expect("nothing is asked");
        assert_eq!(g.state.outcome, outcome);
        for (id, owner) in [(mine, P0), (theirs, P1), (spell, P0)] {
            assert_eq!(reveals(&g, id), vec![owner], "{outcome:?}: {id:?}");
        }
    }
}

/// CR 708.9, RX-1, RX-4: the engine's own draws - a game as long as it records, a loop every player only passes in -
/// reveal every face-down permanent and spell as every end of a game does.
#[test]
fn the_engines_own_draws_reveal_every_face_down_permanent_and_spell() {
    for by_loop in [false, true] {
        let mut g = game(2, P0);
        let mine = manifested(&mut g, P0, P0);
        let theirs = manifested(&mut g, P1, P1);
        let spell = face_down_spell(&mut g);
        if by_loop {
            cantrip_api::history::take_reads();
            let drawn = (0..(crate::loops::LOOP_ROUNDS + 3) * 2).any(|_| {
                let mut drawn = false;
                for p in [P0, P1] {
                    crate::loops::after_frame(&mut g.state);
                    g.state.update_turn(|t| t.priority = p);
                    drawn |= crate::loops::at_priority(&mut cx(&mut g));
                }
                drawn
            });
            assert!(drawn, "a loop of passes is a draw");
        } else {
            if let Some(at) = g.state.agenda.len().checked_sub(1) {
                g.state.update_frame(at, |f| f.pending = None);
            }
            g.max_events = g.log.len();
            g.run();
        }
        assert_eq!(g.state.outcome, Outcome::Draw, "by loop: {by_loop}");
        for (id, owner) in [(mine, P0), (theirs, P1), (spell, P0)] {
            assert_eq!(reveals(&g, id), vec![owner], "by loop {by_loop}: {id:?}");
        }
    }
}

/// CR 708.9, 702.26a: a face-down permanent phased out is still a face-down permanent on the battlefield, revealed at the end
/// of the game.
#[test]
fn a_phased_out_face_down_permanent_is_revealed_at_the_end_of_the_game() {
    let mut g = game(2, P0);
    let phased = manifested(&mut g, P1, P1);
    crate::phasing::phase_out(&mut cx(&mut g), &[phased], None, false);
    assert!(g.state.obj(phased).zone.is(ZoneKind::PhasedOut));
    crate::ops::win_game(&mut cx(&mut g), P0);
    assert_eq!(reveals(&g, phased), vec![P1]);
}
