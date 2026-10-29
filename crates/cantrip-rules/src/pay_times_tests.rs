//! A cost paid up to a number of times as an effect resolves, and the reflexive ability that counts the payments (CR 118.12,
//! 603.12a): one payment whose number the payer raises and lowers within 1 to its most, the mana that many times over, and
//! "choose up to that many" modes read from that number (CR 700.2b) - Tranquil Frillback's and Hawkeye, Master Marksman's
//! shape, over a test card.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

const ASK: Question = Question::new("PayForModes", "pay {G} up to three times?", "", "Pay {cost}", "Don't pay");

/// "You may pay {G} up to three times. When you pay this cost one or more times, choose up to that many - ..."
fn pay_up_to_three(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let paid = g.may_pay_times(item.controller, item.object, item.object, mana("{G}"), 3, &ASK)?;
    if paid > 0 {
        let this = g.card_id(&HATCH);
        g.reflexive_trigger_counted(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None, paid);
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(|g, item| {
        g.gain_life(item.controller, 1);
        Ok(())
    }, "You gain 1 life."),
    mode(|g, item| {
        g.gain_life(item.controller, 2);
        Ok(())
    }, "You gain 2 life."),
    mode(|g, item| {
        g.gain_life(item.controller, 4);
        Ok(())
    }, "You gain 4 life."),
])
.up_to_that_many();

static FILLER: CardDef = CardDef { name: "Times Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static HATCH: CardDef = CardDef {
    name: "Times Test Hatch",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], pay_up_to_three),
    delayed: &[modal_delayed_trigger(&MODAL, "When you pay this cost one or more times, choose up to that many.")],
    ..CardDef::BLANK
};

/// Your precombat main phase with priority, the Hatch in your hand and `green` {G} in your pool.
fn game(green: u32) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &HATCH])));
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
    let hatch = g.state.push_obj(Obj::new(g.cards.id_of(&HATCH), YOU, Zone::hand(YOU)));
    g.state.zone_push(Zone::hand(YOU), hatch);
    g.state.add_mana(YOU, Mana::G, green, hatch, None, ManaLasts::Step);
    (g, hatch)
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

/// Casts the Hatch and passes until it resolves, giving back the payment question.
fn to_payment(g: &mut Game, hatch: ObjectId) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let cast = options.iter().find(|o| o.source == Some(hatch)).cloned().expect("the Hatch is offered");
    answer(g, &r, Answer::Act(Action::of(&cast)));
    loop {
        let r = next(g);
        if !matches!(r.kind, RequestKind::Priority { .. }) {
            assert!(matches!(r.kind, RequestKind::UnlessPays { .. }), "expected the payment, got {:?}", r.kind);
            return r;
        }
        answer(g, &r, Answer::Pass);
    }
}

fn option(r: &Request, kind: ActionKind) -> Option<ActionOption> {
    match &r.kind {
        RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.kind == kind).cloned(),
        _ => None,
    }
}

/// The number of times and the cost at it, as the payment question shows them.
fn shown(r: &Request) -> (Option<u16>, Option<u16>, ManaCost) {
    match &r.kind {
        RequestKind::UnlessPays { x, times, total, .. } => (*x, *times, *total),
        other => panic!("expected the payment, got {other:?}"),
    }
}

/// Raises the number of times to `n` and pays it, giving back the next question.
fn pay_times(g: &mut Game, mut r: Request, n: u16) -> Request {
    while shown(&r).0 != Some(n) {
        let raise = option(&r, ActionKind::IncreaseX).expect("the number can be raised");
        answer(g, &r, Answer::Act(Action::of(&raise)));
        r = next(g);
    }
    let pay = option(&r, ActionKind::PayCost).expect("the pool pays it");
    answer(g, &r, Answer::Act(Action::of(&pay)));
    next(g)
}

#[test]
fn the_number_of_times_runs_from_one_to_the_most_and_the_cost_follows_it() {
    let (mut g, hatch) = game(3);
    let mut r = to_payment(&mut g, hatch);
    assert_eq!(shown(&r), (Some(1), Some(3), mana("{G}")), "it starts at once, the cost paid once");
    assert!(option(&r, ActionKind::DecreaseX).is_none(), "no fewer than once: declining is paying it no times");
    for (n, cost) in [(2, mana("{G}{G}")), (3, mana("{G}{G}{G}"))] {
        let raise = option(&r, ActionKind::IncreaseX).expect("raised up to three");
        answer(&mut g, &r, Answer::Act(Action::of(&raise)));
        r = next(&mut g);
        assert_eq!(shown(&r), (Some(n), Some(3), cost));
    }
    assert!(option(&r, ActionKind::IncreaseX).is_none(), "no more than three times");
    let lower = option(&r, ActionKind::DecreaseX).expect("lowered from three");
    answer(&mut g, &r, Answer::Act(Action::of(&lower)));
    let r = next(&mut g);
    assert_eq!(shown(&r), (Some(2), Some(3), mana("{G}{G}")));
}

#[test]
fn paying_twice_spends_the_mana_twice_and_the_ability_triggers_once_choosing_up_to_two_modes() {
    let (mut g, hatch) = game(3);
    let r = to_payment(&mut g, hatch);
    let r = pay_times(&mut g, r, 2);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1, "{{G}} paid two times of three");
    match &r.kind {
        RequestKind::ChooseModes { count, at_most, .. } => assert_eq!((*count, *at_most), (0, 2), "choose up to two"),
        other => panic!("expected the modes, got {other:?}"),
    }
    assert!(g.answer(r.id, r.player, Answer::Modes(vec![0, 1, 2])).is_err(), "three modes for two payments are refused");
    answer(&mut g, &r, Answer::Modes(vec![1, 2]));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "one reflexive ability, one choice of modes: got {:?}", r.kind);
    assert_eq!(g.state.stack.len(), 1, "it triggered once for the two payments");
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Pass);
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Pass);
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 6, "the two modes chosen, 2 and 4 life");
}

#[test]
fn paying_once_chooses_one_mode_and_declining_triggers_nothing() {
    let (mut g, hatch) = game(3);
    let r = to_payment(&mut g, hatch);
    let r = pay_times(&mut g, r, 1);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 2);
    match &r.kind {
        RequestKind::ChooseModes { count, at_most, .. } => assert_eq!((*count, *at_most), (0, 1), "choose up to one"),
        other => panic!("expected the modes, got {other:?}"),
    }
    assert!(g.answer(r.id, r.player, Answer::Modes(vec![0, 2])).is_err(), "two modes for one payment are refused");

    let (mut g, hatch) = game(3);
    let r = to_payment(&mut g, hatch);
    let decline = option(&r, ActionKind::DeclineCost).expect("declining is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&decline)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "no modes to choose: got {:?}", r.kind);
    assert!(g.state.stack.is_empty(), "nothing triggered");
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 3, "nothing paid");
}

#[test]
fn paying_three_times_may_choose_every_mode() {
    let (mut g, hatch) = game(3);
    let r = to_payment(&mut g, hatch);
    let r = pay_times(&mut g, r, 3);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 0);
    match &r.kind {
        RequestKind::ChooseModes { count, at_most, .. } => assert_eq!((*count, *at_most), (0, 3), "choose up to three"),
        other => panic!("expected the modes, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Modes(vec![0, 1, 2]));
    next(&mut g);
    assert_eq!(g.state.stack.len(), 1);
}

/// Whether the pool pays a cost whose symbols may be paid more than one way (CR 118.13): a {2/G} paid twice over by {2}{2},
/// {G}{2} or {G}{G} - so whether "pay" is offered never reads an unannounced symbol as free - and a {G/P} with {G} or 2 life; with mana spent as though it were any color, any mana for {G}.
#[test]
fn the_pool_pays_a_cost_of_symbols_paid_more_than_one_way_only_with_enough_of_some_way() {
    let pool = |c: u32, g: u32| ManaPool([0, 0, 0, 0, g, c]);
    let twice = mana("{2/G}").times(2).expect("fits");
    assert!(crate::actions::pool_pays(&pool(4, 0), &twice, 0, false), "{{2}}{{2}}");
    assert!(crate::actions::pool_pays(&pool(2, 1), &twice, 0, false), "{{G}}{{2}}");
    assert!(crate::actions::pool_pays(&pool(0, 2), &twice, 0, false), "{{G}}{{G}}");
    assert!(!crate::actions::pool_pays(&pool(3, 0), &twice, 0, false), "three colorless pay neither way");
    assert!(!crate::actions::pool_pays(&pool(1, 0), &mana("{G/P}"), 1, false), "no {{G}} and no 2 life");
    assert!(crate::actions::pool_pays(&pool(0, 0), &mana("{G/P}"), 2, false), "2 life");
    assert!(crate::actions::pool_pays(&pool(0, 1), &mana("{G/P}"), 0, false), "{{G}}");
    // Mana spent as though it were mana of any color (CR 609.4b): colorless pays a {G}, and a {G/P} announced as its {G}.
    assert!(crate::actions::pool_pays(&pool(1, 0), &mana("{G}"), 0, true), "colorless as {{G}}");
    assert!(crate::actions::pool_pays(&pool(1, 0), &mana("{G/P}"), 1, true), "colorless as the {{G}} of {{G/P}}");
    assert!(!crate::actions::pool_pays(&pool(1, 0), &mana("{G}{G}"), 0, true), "one mana for two symbols");
}

/// How the pool pays a cost with a symbol nobody has said how to pay (CR 107.4e, 107.4f): never as if it were free. A {W/P}
/// is paid by mana only with {W}, and a {2/G} with {G} or {2}, which is the payer's choice.
#[test]
fn the_pool_never_pays_an_unannounced_phyrexian_or_monocolored_hybrid_symbol_for_nothing() {
    use crate::actions::{ways, Ways};
    let pool = |w: u32, g: u32, c: u32| ManaPool([w, 0, 0, 0, g, c]);
    assert!(matches!(ways(&pool(0, 0, 0), &mana("{W/P}")), Ways::None), "an empty pool pays no {{W/P}}");
    assert!(matches!(ways(&pool(0, 0, 3), &mana("{1}{W/P}")), Ways::None), "colorless pays no {{W/P}}");
    assert!(matches!(ways(&pool(1, 0, 1), &mana("{1}{W/P}")), Ways::One([1, 0, 0, 0, 0, 1])), "{{W}} and {{1}}");
    assert!(matches!(ways(&pool(0, 0, 1), &mana("{2/G}")), Ways::None), "one colorless pays no {{2/G}}");
    assert!(matches!(ways(&pool(0, 1, 2), &mana("{2/G}")), Ways::Many), "{{G}} or {{2}}");
}
