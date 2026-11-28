//! "Players may spend mana as though it were mana of any color" (Mycosynth Lattice's, CR 609.4b) is read as each unit of mana is
//! spent, over test cards: the payer pays the parts of a total cost in any order (CR 601.2h), so another part may be paid
//! between two units of its mana, and a static ability's effect applies only while it is there (CR 611.3a). A unit spent while
//! the permanent that gave the rule was there pays as it may then; one spent after it was sacrificed pays as mana ordinarily
//! does. No printed card in the engine has a cost whose payment asks its order with mana of two or more units and a part that
//! could sacrifice the rule's permanent, so a scenario cannot show it.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn any_payment(_: &dyn Query, _: &Asker, _: &Spending) -> AnyMana {
    AnyMana::Color
}

fn is_artifact(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Artifact)
}

fn is_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

fn gain_three(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Spend Timing Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Players may spend mana as though it were mana of any color."
static LATTICE: CardDef = CardDef {
    name: "Spend Timing Test Lattice",
    types: TypeSet::of(&[CardType::Artifact]),
    spend_any_mana: &[any_payment],
    ..CardDef::BLANK
};

/// An artifact creature: both of the engine's sacrifices may take it, so which is paid first matters and is asked.
static CONSTRUCT: CardDef = CardDef {
    name: "Spend Timing Test Construct",
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "{R}{R}, Sacrifice an artifact, Sacrifice a creature: You gain 3 life."
static ENGINE: CardDef = CardDef {
    name: "Spend Timing Test Engine",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(
        Cost::sequence(&[cost("{R}{R}"), Cost::sacrifice(1, is_artifact), Cost::sacrifice(1, is_creature)]),
        gain_three,
        "{R}{R}, Sacrifice an artifact, Sacrifice a creature: You gain 3 life.",
    )],
    ..CardDef::BLANK
};

/// Your main phase with priority, the Lattice, the Construct and the Engine on the battlefield and {C}{C} in your pool.
fn game() -> (Game, ObjectId, ObjectId, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &LATTICE, &CONSTRUCT, &ENGINE])));
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
    let mut put = |def: &'static CardDef| {
        let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let (lattice, construct, engine) = (put(&LATTICE), put(&CONSTRUCT), put(&ENGINE));
    g.state.add_mana(YOU, Mana::C, 2, engine, None, ManaLasts::Step, false);
    (g, lattice, construct, engine)
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

/// Activates the Engine and gives back the first question of its payment.
fn activate(g: &mut Game, engine: ObjectId) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(engine)).cloned().expect("the Engine's ability is offered");
    answer(g, &r, Answer::Act(Action::of(&option)));
    next(g)
}

/// Answers the question of which part to pay next with the first of `kind` (CR 601.2h).
fn pay_next(g: &mut Game, r: &Request, kind: CostPaymentKind) -> Request {
    let RequestKind::ChooseCostPayment { choices, .. } = &r.kind else { panic!("which part is paid next is asked: {:?}", r.kind) };
    let at = choices.iter().position(|c| c.kind == kind).expect("that part is offered");
    answer(g, r, Answer::CostPayment(at));
    next(g)
}

/// Answers a choice of permanents for a sacrifice with `chosen`.
fn sacrifice(g: &mut Game, r: &Request, chosen: ObjectId) -> Request {
    let RequestKind::ChooseCards { cards, .. } = &r.kind else { panic!("the permanent to sacrifice is asked: {:?}", r.kind) };
    assert!(cards.contains(&chosen), "it may be sacrificed");
    answer(g, r, Answer::Cards(vec![chosen]));
    next(g)
}

/// CR 609.4b, 601.2h: both units paid while the Lattice is there each pay an {R}; the Lattice may then be sacrificed to the
/// same cost, and the ability is activated.
#[test]
fn mana_paid_before_the_lattice_is_sacrificed_pays_as_though_it_were_red() {
    let (mut g, lattice, construct, engine) = game();
    let r = activate(&mut g, engine);
    let r = pay_next(&mut g, &r, CostPaymentKind::Mana);
    let r = pay_next(&mut g, &r, CostPaymentKind::Mana);
    let r = pay_next(&mut g, &r, CostPaymentKind::Sacrifice);
    let r = sacrifice(&mut g, &r, lattice);
    let r = if matches!(r.kind, RequestKind::ChooseCards { .. }) { sacrifice(&mut g, &r, construct) } else { r };
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "the activation is made: {:?}", r.kind);
    assert_eq!(g.state.stack.len(), 1, "the ability is on the stack");
    assert!(!g.state.battlefield().contains(&lattice), "the Lattice was sacrificed");
}

/// CR 609.4b, 611.3a, 601.2h: one {C} paid while the Lattice is there pays an {R}; the Lattice sacrificed next, the other {C}
/// can't pay the other {R}, which only red mana may now pay - the payment can't be completed, and nothing else in the pool
/// is offered for it.
#[test]
fn mana_paid_after_the_lattice_is_sacrificed_pays_only_as_what_it_is() {
    let (mut g, lattice, _, engine) = game();
    let r = activate(&mut g, engine);
    let r = pay_next(&mut g, &r, CostPaymentKind::Mana);
    let r = pay_next(&mut g, &r, CostPaymentKind::Sacrifice);
    let r = sacrifice(&mut g, &r, lattice);
    // The other sacrifice and the mana: their order no longer matters with the Lattice gone, so the mana is paid next.
    let r = if matches!(r.kind, RequestKind::ChooseCostPayment { .. }) { pay_next(&mut g, &r, CostPaymentKind::Mana) } else { r };
    let RequestKind::PayCosts { remaining, options, .. } = &r.kind else { panic!("the {{R}} is still owed: {:?}", r.kind) };
    assert_eq!(*remaining, ManaCost { symbols: [0, 0, 0, 1, 0, 0], ..ManaCost::ZERO }, "colorless mana was spent as though it were red after the Lattice left");
    assert!(options.iter().all(|o| o.kind != ActionKind::AssignMana && o.kind != ActionKind::ConfirmCasting), "the {{C}} is offered for the {{R}}: {options:?}");
}
