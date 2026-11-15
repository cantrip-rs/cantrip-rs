//! An X the text of an activated ability defines (CR 107.3c), over a test card with no targets ("{X}, {T}: ... X is the
//! number of cards in your hand", as Bargaining Table's "in an opponent's hand"): its controller announces no X; X is read
//! as the total cost is determined (CR 601.2f by way of 602.2b) and is the ability's X on the stack; and the ability is
//! offered only where the cost at that X could be paid. Reflecting Mirror's scenarios show one that reads its target.

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Defined X Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "X is the number of cards in your hand."
fn cards_in_your_hand(q: &dyn Query, asker: &Asker, _: &[Target]) -> u32 {
    cantrip_state::count_u32(q.state().zone(Zone::hand(asker.controller)).len())
}

/// "{X}, {T}: Nothing. X is the number of cards in your hand."
static TABLE: CardDef = CardDef {
    name: "Defined X Table",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(of_mana(mana("{X}")).and(TAP), nothing, "{X}, {T}: Nothing. X is the number of cards in your hand.").x_is(cards_in_your_hand)],
    ..CardDef::BLANK
};

/// "This ability costs {1} less to activate" (CR 601.2f by way of 602.2b): its own cost change.
fn one_less(_: &dyn Query, _: &Asker, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(1)
}

/// "{X}{W/P}, {T}: Nothing. X is the number of cards in your hand. This ability costs {1} less to activate." An X the text
/// defines beside a Phyrexian symbol announced as the ability is proposed (CR 107.4f, 601.2b) and its own cost change.
static TOLL: CardDef = CardDef {
    name: "Defined X Toll",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(
        of_mana(mana("{X}{W/P}")).and(TAP),
        nothing,
        "{X}{W/P}, {T}: Nothing. X is the number of cards in your hand. This ability costs {1} less to activate.",
    )
    .x_is(cards_in_your_hand)
    .cost_change(one_less)],
    ..CardDef::BLANK
};

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &TABLE, &TOLL])));
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
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

fn add_mana(g: &mut Game, pool: &[Mana]) {
    let source = g.state.battlefield().first().copied().unwrap_or(ObjectId(0));
    for &m in pool {
        g.state.add_mana(YOU, m, 1, source, None, ManaLasts::Step, false);
    }
}

fn offered(g: &mut Game, source: ObjectId) -> bool {
    let mut cx = Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    priority_options(&mut cx, YOU).iter().any(|o| o.source == Some(source))
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

#[test]
fn an_x_the_text_defines_is_not_announced_and_is_what_the_cost_is_paid_with() {
    let mut g = game();
    let table = put(&mut g, &TABLE, Zone::BATTLEFIELD);
    // Your hand holds whatever the game dealt; three more make sure X is not 0.
    for _ in 0..3 {
        put(&mut g, &FILLER, Zone::hand(YOU));
    }
    let hand = cantrip_state::count_u32(g.state.zone(Zone::hand(YOU)).len());
    add_mana(&mut g, &vec![Mana::C; hand as usize - 1]);
    assert!(!offered(&mut g, table), "one mana short of the cards in your hand");
    add_mana(&mut g, &[Mana::C, Mana::C]);
    assert!(offered(&mut g, table), "enough for the cards in your hand");

    let r = next(&mut g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(table)).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    let r = next(&mut g);
    assert!(!matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "X is not announced (CR 107.3c), got {:?}", r.kind);
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&confirm))).expect("a legal answer");
        next(&mut g);
    }
    assert_eq!(g.state.stack.last().map(|i| (i.object, i.x)), Some((table, hand)), "X is the cards in your hand");
    assert_eq!(g.state.player(YOU).pool.total(), 1, "{{X}} paid as that many, one mana left");
}

#[test]
fn an_x_the_text_defines_composes_with_a_phyrexian_symbol_paid_with_life_and_the_abilitys_own_cost_change() {
    let mut g = game();
    let toll = put(&mut g, &TOLL, Zone::BATTLEFIELD);
    for _ in 0..3 {
        put(&mut g, &FILLER, Zone::hand(YOU));
    }
    let hand = cantrip_state::count_u32(g.state.zone(Zone::hand(YOU)).len());
    // No white mana: {W/P} is paid with 2 life, and the total is X ({hand}) less {1}.
    add_mana(&mut g, &vec![Mana::C; hand as usize - 2]);
    assert!(!offered(&mut g, toll), "one mana short of X less {{1}}");
    add_mana(&mut g, &[Mana::C]);
    assert!(offered(&mut g, toll), "X less {{1}}, the Phyrexian symbol paid with life");

    let r = next(&mut g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(toll)).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        let r = next(&mut g);
        match &r.kind {
            RequestKind::PayCosts { announcing: true, .. } => panic!("X is not announced (CR 107.3c)"),
            RequestKind::PayCosts { options, .. } => {
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool and life pay it").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&confirm))).expect("a legal answer");
            }
            // How {W/P} is paid, announced as the ability is proposed (CR 601.2b): with 2 life.
            RequestKind::ChooseModes { .. } => g.answer(r.id, r.player, Answer::Modes(vec![0])).expect("paying life"),
            RequestKind::Priority { .. } => break,
            other => panic!("unexpected question {other:?}"),
        }
    }
    assert_eq!(g.state.stack.last().map(|i| (i.object, i.x)), Some((toll, hand)), "X is the cards in your hand");
    assert_eq!(g.state.player(YOU).pool.total(), 0, "X less {{1}} paid with mana");
    assert_eq!(g.state.player(YOU).life, 18, "{{W/P}} paid with 2 life");
}
