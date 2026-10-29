//! A special action's cost changes announced and split as a cast's are (CR 116.2b, 601.2b, 118.7e), over test cards: a
//! permanent makes turning face-down creatures face up cost {U/R} less. Turning a cloaked card face up (CR 701.58b), the
//! payer chooses which half of the hybrid reduction takes mana (`Label::ReduceAs`), and each choice changes what is paid;
//! a hybrid symbol of the card's mana cost the reduction could meet is announced first (`Label::PayHybridAs`).

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Special Cost Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A creature card costing {2}{U}{R}.
static PAIR: CardDef = CardDef {
    name: "Special Cost Test Pair",
    cost: mana("{2}{U}{R}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// A creature card costing {1}{U/R}.
static HYBRID: CardDef = CardDef {
    name: "Special Cost Test Hybrid",
    cost: mana("{1}{U/R}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// "Turning face-down creatures you control face up costs {U/R} less": the one special action a face-down permanent has.
fn face_up_costs_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::SpecialAction(card, _) if player == q.controller(source) && q.obj(card).face_down => {
            CostDelta { less: mana("{U/R}"), ..CostDelta::NONE }
        }
        _ => CostDelta::NONE,
    }
}

static TAILOR: CardDef = CardDef {
    name: "Special Cost Test Tailor",
    types: TypeSet::of(&[CardType::Artifact]),
    cost_changes: &[face_up_costs_less],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// Your main phase with priority, the Tailor yours, `def` cloaked under your control and `pool` in your mana pool.
fn game(def: &'static CardDef, pool: &[Mana]) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &PAIR, &HYBRID, &TAILOR])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let tailor = g.state.push_obj(Obj::new(table.id_of(&TAILOR), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, tailor);
    g.state.see_statics();
    let card = g.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::library(YOU)));
    g.state.zone_push(Zone::library(YOU), card);
    let cloaked = cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(&mut g), card, YOU, FaceDown::Cloaked, false)
        .expect("nothing is asked")
        .expect("it enters face down");
    for &m in pool {
        g.state.add_mana(YOU, m, 1, tailor, None, ManaLasts::Step);
    }
    (g, cloaked)
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

/// Takes the special action of turning `cloaked` face up, and gives back the next question.
fn turn_up(g: &mut Game, cloaked: ObjectId) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(cloaked) && o.kind == ActionKind::ActivateAbility).expect("turning it face up is offered").clone();
    answer(g, &r, Answer::Act(Action::of(&option)));
    next(g)
}

fn modes(r: &Request) -> Vec<Label> {
    match &r.kind {
        RequestKind::ChooseModes { modes, .. } => modes.clone(),
        other => panic!("expected a choice of modes, got {other:?}"),
    }
}

fn face_up(g: &Game, cloaked: ObjectId) -> bool {
    !g.state.obj(cloaked).face_down
}

/// {2}{U}{R} less {U/R}: the reduction taking {U} leaves {2}{R}, which C, C, R pays exactly; taking {R} leaves {2}{U},
/// which C, C, U pays exactly.
#[test]
fn each_split_of_a_hybrid_reduction_changes_what_turning_face_up_costs() {
    for (second_half, pool) in [(false, [Mana::C, Mana::C, Mana::R]), (true, [Mana::C, Mana::C, Mana::U])] {
        let (mut g, cloaked) = game(&PAIR, &pool);
        let r = turn_up(&mut g, cloaked);
        assert_eq!(modes(&r), vec![Label::ReduceAs(Mana::U, Mana::R, Mana::R)], "the payer splits the reduction");
        answer(&mut g, &r, Answer::Modes(if second_half { vec![0] } else { vec![] }));
        g.run();
        assert!(face_up(&g, cloaked), "the pool paid what the split left");
        assert_eq!(g.state.player(YOU).pool.total(), 0);
    }
    // The split that leaves {2}{U} with C, C, R in the pool: the {U} is still owed.
    let (mut g, cloaked) = game(&PAIR, &[Mana::C, Mana::C, Mana::R]);
    let r = turn_up(&mut g, cloaked);
    answer(&mut g, &r, Answer::Modes(vec![0]));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::PayCosts { total, .. } => assert_eq!(*total, mana("{2}{U}")),
        other => panic!("expected the payment, got {other:?}"),
    }
    assert!(!face_up(&g, cloaked));
}

/// {1}{U/R} less {U/R}: the {U/R} the reduction could meet is announced first. Announced as {U} and reduced by {U}, {1}
/// is left, which C pays; announced as {R} and reduced by {U} (which the cost then lacks, so it takes generic mana), {R}
/// is left, which R pays.
#[test]
fn a_hybrid_symbol_a_reduction_meets_is_announced_before_the_split() {
    for (as_other, pool) in [(false, Mana::C), (true, Mana::R)] {
        let (mut g, cloaked) = game(&HYBRID, &[pool]);
        let r = turn_up(&mut g, cloaked);
        let asked = modes(&r);
        assert_eq!(asked.len(), 1);
        assert!(matches!(asked[0], Label::PayHybridAs(Mana::U, Mana::R, _)), "the hybrid symbol is announced: {asked:?}");
        answer(&mut g, &r, Answer::Modes(if as_other { vec![0] } else { vec![] }));
        let r = next(&mut g);
        assert_eq!(modes(&r), vec![Label::ReduceAs(Mana::U, Mana::R, Mana::R)]);
        answer(&mut g, &r, Answer::Modes(vec![]));
        g.run();
        assert!(face_up(&g, cloaked), "the pool paid what was left");
        assert_eq!(g.state.player(YOU).pool.total(), 0);
    }
}
