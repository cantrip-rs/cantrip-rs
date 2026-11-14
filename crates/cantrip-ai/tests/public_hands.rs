//! Continuous public information survives history folding and world sampling.
//! Decks and actions are deliberate; no seed search or stochastic setup.
use cantrip_ai::{Knowledge, Prior, determinize};
use cantrip_api::Game as Rules;
use cantrip_engine::{
    visibility::{PUBLIC, observable},
    *,
};
use cantrip_obs::InfoMode;
use std::collections::HashSet;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn all(_: &dyn Query, _: &Asker, _: PlayerId) -> bool {
    true
}
static OPEN: CardDef = CardDef {
    name: "Public hand fixture",
    types: TypeSet::of(&[CardType::Enchantment]),
    has_cost: true,
    play_rules: &[PlayRule::HandRevealed(all)],
    ..CardDef::BLANK
};
static LAND: CardDef = CardDef {
    name: "Visibility test land",
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};
static OTHER: CardDef = CardDef {
    name: "Visibility test other land",
    ..LAND
};
fn source(g: &dyn Query) -> ObjectId {
    *g.state()
        .battlefield()
        .iter()
        .find(|&&id| g.card(g.state().obj(id).card).name == OPEN.name)
        .unwrap()
}
fn remove(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    let id = source(g);
    g.destroy_all(&[id]);
    Ok(())
}
fn draw(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    g.draw(B, 2);
    Ok(())
}
fn recycle(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    let id = g.state().zone(Zone::hand(B))[0];
    g.move_together(&[(id, Zone::library(B))], MoveReason::Other);
    Ok(())
}
fn simultaneous(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    let id = source(g);
    let top = g.state().zone(Zone::library(B))[0];
    g.move_together(
        &[(id, Zone::graveyard(A)), (top, Zone::hand(B))],
        MoveReason::Other,
    );
    Ok(())
}
static REMOVE: CardDef = CardDef {
    name: "Visibility fixture remove",
    types: TypeSet::of(&[CardType::Sorcery]),
    has_cost: true,
    spell: spell(&[], remove),
    ..CardDef::BLANK
};
static DRAW: CardDef = CardDef {
    name: "Visibility fixture draw",
    spell: spell(&[], draw),
    ..REMOVE
};
static RECYCLE: CardDef = CardDef {
    name: "Visibility fixture recycle",
    spell: spell(&[], recycle),
    ..REMOVE
};
static BATCH: CardDef = CardDef {
    name: "Visibility fixture batch",
    spell: spell(&[], simultaneous),
    ..REMOVE
};

fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &OPEN, &LAND, &OTHER, &REMOVE, &DRAW, &RECYCLE, &BATCH,
    ])));
    let mut a = vec![&OPEN, &REMOVE, &DRAW, &RECYCLE, &BATCH, &LAND, &OTHER];
    a.extend([&LAND; 23]);
    let b = (0..30)
        .map(|i| cards.id_of(if i % 2 == 0 { &LAND } else { &OTHER }))
        .collect();
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![a.into_iter().map(|c| cards.id_of(c)).collect(), b],
            ..Default::default()
        },
    );
    g.run();
    assert_eq!(g.state.zone(Zone::hand(B)).len(), 7);
    g
}
fn cast(g: &mut Game, def: &'static CardDef) {
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = A;
        t.priority = A;
        t.began = true;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.run();
    let r = g.pending().cloned().unwrap();
    let RequestKind::Priority { options } = &r.kind else {
        panic!("{:?}", r.kind)
    };
    let option = options
        .iter()
        .find(|o| {
            o.kind == ActionKind::CastSpell
                && o.source
                    .is_some_and(|id| g.state.obj(id).card == g.cards.id_of(def))
        })
        .expect("spell offered");
    g.answer(r.id, r.player, Answer::Act(Action::of(option)))
        .unwrap();
    for _ in 0..30 {
        g.run();
        let r = g.pending().cloned().unwrap();
        if g.state.stack.is_empty() && matches!(r.kind, RequestKind::Priority { .. }) {
            return;
        }
        let answer = match &r.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts {
                announcing: true, ..
            } => Answer::Announce(0),
            RequestKind::PayCosts { options, .. } => Answer::Act(Action::of(
                options
                    .iter()
                    .find(|o| o.kind == ActionKind::ConfirmCasting)
                    .unwrap(),
            )),
            other => panic!("{other:?}"),
        };
        g.answer(r.id, r.player, answer).unwrap();
    }
    panic!("spell did not finish");
}
fn fresh_agrees(g: &Game, k: &mut Knowledge) {
    k.update(g);
    assert_eq!(*k, Knowledge::of(g, A));
}
fn sample_preserves(g: &Game, k: &Knowledge, ids: &[ObjectId]) {
    for mode in [InfoMode::Decklist, InfoMode::Hidden] {
        let mut fork = g.fork_for_search();
        let before: Vec<_> = ids.iter().map(|&id| g.state.obj(id).card).collect();
        determinize(&mut fork, k, A, mode, &Prior::new(), &mut Rng::new(19));
        assert_eq!(
            before,
            ids.iter()
                .map(|&id| fork.state.obj(id).card)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn public_hands_are_visible_to_other_seats_and_spectators_and_fixed_in_worlds() {
    let mut g = game();
    let mut k = Knowledge::of(&g, A);
    cast(&mut g, &OPEN);
    fresh_agrees(&g, &mut k);
    let hand = g.state.zone(Zone::hand(B)).to_vec();
    for viewer in [A, B, PUBLIC] {
        let seen = observable(&g, viewer, &HashSet::new());
        assert!(hand.iter().all(|id| seen.contains(id)));
    }
    assert!(hand.iter().all(|&id| k.knows(id) == Some(false)));
    sample_preserves(&g, &k, &hand);
    let old_hand = hand;
    cast(&mut g, &DRAW);
    fresh_agrees(&g, &mut k);
    let drawn: Vec<_> = g
        .state
        .zone(Zone::hand(B))
        .iter()
        .copied()
        .filter(|id| !old_hand.contains(id))
        .collect();
    assert_eq!(drawn.len(), 2);
    assert!(drawn.iter().all(|&id| k.knows(id) == Some(false)));
    sample_preserves(&g, &k, g.state.zone(Zone::hand(B)));
}

#[test]
fn public_moves_preserve_other_known_cards_and_memory_outlives_visibility() {
    let mut g = game();
    cast(&mut g, &OPEN);
    let mut k = Knowledge::of(&g, A);
    let hand = g.state.zone(Zone::hand(B)).to_vec();
    cast(&mut g, &RECYCLE);
    fresh_agrees(&g, &mut k);
    let top = g.state.zone(Zone::library(B))[0];
    assert_eq!(k.knows(top), Some(true));
    assert!(hand[1..].iter().all(|&id| k.knows(id) == Some(false)));
    cast(&mut g, &REMOVE);
    fresh_agrees(&g, &mut k);
    assert_eq!(g.revealed_hands(), 0);
    let seen = observable(&g, A, &HashSet::new());
    assert!(
        g.state
            .zone(Zone::hand(B))
            .iter()
            .all(|id| !seen.contains(id))
    );
    sample_preserves(&g, &k, g.state.zone(Zone::hand(B)));
    cast(&mut g, &DRAW);
    fresh_agrees(&g, &mut k);
    let now = g.state.zone(Zone::hand(B));
    assert_eq!(k.knows(now[now.len() - 2]), Some(false));
    assert_eq!(k.knows(now[now.len() - 1]), None);
}

#[test]
fn an_arrival_simultaneous_with_the_revealer_leaving_is_hidden() {
    let mut g = game();
    cast(&mut g, &OPEN);
    let mut k = Knowledge::of(&g, A);
    let old = g.state.zone(Zone::hand(B)).to_vec();
    cast(&mut g, &BATCH);
    fresh_agrees(&g, &mut k);
    assert_eq!(g.revealed_hands(), 0);
    let added = *g
        .state
        .zone(Zone::hand(B))
        .iter()
        .find(|id| !old.contains(id))
        .unwrap();
    assert_eq!(k.knows(added), None);
    assert!(old.iter().all(|&id| k.knows(id) == Some(false)));
}
