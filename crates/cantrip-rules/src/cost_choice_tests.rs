//! An activated ability whose cost is a choice between ways of paying it, one of them mana (`ActivatedDef::cost_choice`:
//! Bloodthorn Flail's "Equip—Pay {3} or discard a card"), over test cards. The way is chosen as the ability is announced
//! (CR 601.2b by way of 602.2b); its mana is part of the total then determined, with what increases or reduces it (CR
//! 601.2f), and paid after mana abilities (CR 601.2g-h). The ability is offered only where some way could be paid (CR
//! 118.3), and an alternative cost is paid rather than the whole cost, so no way is chosen (CR 118.9).

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn gain_three(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Way Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

const WAYS: &[ExtraCost] = &[
    ExtraCost { text: "Pay {3}", cost: cost("{3}") },
    ExtraCost { text: "Discard a card", cost: Cost { discard: Discards::one(1, None), ..Cost::NONE } },
];

/// "Pay {3} or discard a card: You gain 3 life."
static FLAIL: CardDef = CardDef {
    name: "Way Test Flail",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::NONE, gain_three, "Pay {3} or discard a card: You gain 3 life.").pay_one_of(WAYS)],
    ..CardDef::BLANK
};

fn tax(_: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if matches!(costed, Costed::Ability(..)) { CostDelta::more(1) } else { CostDelta::NONE }
}

fn discount(_: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if matches!(costed, Costed::Ability(..)) { CostDelta::less(2) } else { CostDelta::NONE }
}

/// "Activated abilities cost {1} more to activate."
static TAX: CardDef = CardDef { name: "Way Test Tax", types: TypeSet::of(&[CardType::Enchantment]), cost_changes: &[tax], ..CardDef::BLANK };
/// "Activated abilities cost {2} less to activate."
static DISCOUNT: CardDef = CardDef { name: "Way Test Discount", cost_changes: &[discount], ..TAX };

/// "You may pay {0} rather than pay the activation cost of abilities you activate" (CR 118.9, 602.2b).
fn for_nothing(_: &dyn Query, asker: &Asker, activator: PlayerId, _: ObjectId, _: &ActivatedDef) -> Option<ManaCost> {
    (activator == asker.controller).then_some(ManaCost::ZERO)
}

static SMITH: CardDef = CardDef { name: "Way Test Smith", types: TypeSet::of(&[CardType::Artifact]), ability_alternatives: &[for_nothing], ..CardDef::BLANK };

fn permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield))
}

/// "Pay {3} or discard a card: Choose target permanent. You gain 3 life." Its target is announced with it, so where a cost
/// change is in play each candidate is judged with each way (CR 601.2c, 601.2f by way of 602.2b).
static HOOK: CardDef = CardDef {
    name: "Way Test Hook",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::NONE, gain_three, "Pay {3} or discard a card: Choose target permanent. You gain 3 life.")
        .targets(&[target(permanent)])
        .pay_one_of(WAYS)],
    ..CardDef::BLANK
};

/// "Activated abilities that target a permanent you control cost {2} less to activate."
fn less_if_yours(q: &dyn Query, _: ObjectId, player: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    let yours = targets.iter().any(|&t| matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && q.obj(id).controller == player));
    if matches!(costed, Costed::Ability(..)) && yours { CostDelta::less(2) } else { CostDelta::NONE }
}

static LURE: CardDef = CardDef { name: "Way Test Lure", cost_changes: &[less_if_yours], ..TAX };

/// Your precombat main phase with priority, the Flail on the battlefield, `cards` cards in your hand and `mana` colorless
/// mana in your pool.
fn game(cards: usize, mana: u32) -> (Game, ObjectId) {
    game_with(&FLAIL, cards, mana)
}

/// `game`, with `def` on the battlefield in place of the Flail.
fn game_with(def: &'static CardDef, cards: usize, mana: u32) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &FLAIL, &TAX, &DISCOUNT, &SMITH, &HOOK, &LURE])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    g.verify = true;
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    // An empty hand, so that what is in it is only what a test puts there.
    for id in g.state.zone(Zone::hand(YOU)).to_vec() {
        g.state.zone_remove(Zone::hand(YOU), id);
        g.state.update_obj(id, |o| o.zone = Zone::exile(YOU));
        g.state.zone_push(Zone::exile(YOU), id);
    }
    let flail = put(&mut g, def, Zone::BATTLEFIELD);
    for _ in 0..cards {
        put(&mut g, &FILLER, Zone::hand(YOU));
    }
    add_mana(&mut g, flail, mana);
    (g, flail)
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}

fn add_mana(g: &mut Game, source: ObjectId, n: u32) {
    if n > 0 {
        g.state.add_mana(YOU, Mana::C, n, source, None, ManaLasts::Step, false);
    }
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// The options `source` has at `YOU`'s priority, with what pays for each instead of its cost.
fn offered(g: &mut Game, source: ObjectId) -> Vec<Option<Grant>> {
    priority_options(&mut cx(g), YOU).iter().filter(|o| o.source == Some(source)).map(|o| o.paid_by).collect()
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

/// Activates the Flail's ability for its own cost (`paid_by` none) or an alternative, and gives back the question after it.
fn activate(g: &mut Game, flail: ObjectId, alternative: bool) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(flail) && o.paid_by.is_some() == alternative).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    next(g)
}

/// Pays what the payment asks with the pool and the hand, and gives back the question after it.
fn pay(g: &mut Game, mut r: Request) -> Request {
    loop {
        match &r.kind {
            RequestKind::PayCosts { announcing: false, options, .. } => {
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&confirm))).expect("a legal answer");
            }
            RequestKind::ChooseCostPayment { .. } => g.answer(r.id, r.player, Answer::CostPayment(0)).expect("a legal answer"),
            // The one card in the hand, discarded.
            RequestKind::ChooseCards { cards, .. } => g.answer(r.id, r.player, Answer::Cards(vec![cards[0]])).expect("a legal answer"),
            _ => return r,
        }
        r = next(g);
    }
}

fn on_stack(g: &Game, flail: ObjectId) -> bool {
    g.state.stack.last().is_some_and(|i| i.object == flail)
}

fn hand(g: &Game) -> usize {
    g.state.zone(Zone::hand(YOU)).len()
}

#[test]
fn it_is_offered_only_where_some_way_could_be_paid_its_mana_included() {
    // No card to discard and no mana: neither way.
    let (mut g, flail) = game(0, 0);
    assert!(offered(&mut g, flail).is_empty());
    let (mut g, flail) = game(0, 2);
    assert!(offered(&mut g, flail).is_empty(), "{{2}} doesn't pay {{3}}");
    let (mut g, flail) = game(0, 3);
    assert_eq!(offered(&mut g, flail), vec![None], "{{3}} pays the mana way");
    let (mut g, flail) = game(1, 0);
    assert_eq!(offered(&mut g, flail), vec![None], "a card pays the discard way");
}

#[test]
fn a_lone_way_in_mana_is_taken_and_its_mana_paid_with_the_total() {
    let (mut g, flail) = game(0, 3);
    let r = activate(&mut g, flail, false);
    // No question of the way: the discard can't be paid (CR 118.3), so "Pay {3}" is the one way, and the total is {3}.
    assert!(!matches!(r.kind, RequestKind::ChooseModes { .. }), "no way is asked: {:?}", r.kind);
    if let RequestKind::PayCosts { total, .. } = &r.kind {
        assert_eq!(total.generic, 3);
    }
    pay(&mut g, r);
    assert!(on_stack(&g, flail));
    assert_eq!(g.state.player(YOU).pool.total(), 0, "{{3}} paid");
}

#[test]
fn both_ways_are_offered_and_those_whose_mana_could_be_paid_advised() {
    let (mut g, flail) = game(1, 3);
    let r = activate(&mut g, flail, false);
    let RequestKind::ChooseModes { modes, eligible, affordable, .. } = &r.kind else { panic!("expected the choice of way, got {:?}", r.kind) };
    assert_eq!(modes.len(), 2);
    assert_eq!((eligible.clone(), affordable.clone()), (vec![0, 1], vec![0, 1]));
    // Paying {3}: the card stays in the hand.
    g.answer(r.id, r.player, Answer::Modes(vec![0])).expect("a legal answer");
    let r = next(&mut g);
    pay(&mut g, r);
    assert!(on_stack(&g, flail));
    assert_eq!((g.state.player(YOU).pool.total(), hand(&g)), (0, 1));

    // Discarding: the mana stays in the pool.
    let (mut g, flail) = game(1, 3);
    let r = activate(&mut g, flail, false);
    g.answer(r.id, r.player, Answer::Modes(vec![1])).expect("a legal answer");
    let r = next(&mut g);
    pay(&mut g, r);
    assert!(on_stack(&g, flail));
    assert_eq!((g.state.player(YOU).pool.total(), hand(&g)), (3, 0));
}

#[test]
fn a_way_whose_mana_cant_be_paid_may_be_announced_and_is_taken_back() {
    let (mut g, flail) = game(1, 0);
    let r = activate(&mut g, flail, false);
    let RequestKind::ChooseModes { eligible, affordable, .. } = &r.kind else { panic!("expected the choice of way, got {:?}", r.kind) };
    assert_eq!((eligible.clone(), affordable.clone()), (vec![0, 1], vec![1]), "only the discard could be paid");
    // Announced, {3} can't be paid (CR 601.2h): the activation is taken back (CR 733.1), the card still in the hand.
    g.answer(r.id, r.player, Answer::Modes(vec![0])).expect("a legal answer");
    let r = next(&mut g);
    let r = match r.kind {
        RequestKind::PayCosts { announcing: false, ref options, .. } => {
            let cancel = options.iter().find(|o| o.kind == ActionKind::CancelCasting).expect("it may be taken back").clone();
            g.answer(r.id, r.player, Answer::Act(Action::of(&cancel))).expect("a legal answer");
            next(&mut g)
        }
        _ => r,
    };
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "expected priority again, got {:?}", r.kind);
    assert!(g.state.stack.is_empty());
    assert_eq!(hand(&g), 1);
}

#[test]
fn a_cost_increase_applies_to_the_total_with_the_ways_mana() {
    // {3} and the tax's {1}: three mana is not enough.
    let (mut g, flail) = game(0, 3);
    put(&mut g, &TAX, Zone::BATTLEFIELD);
    assert!(offered(&mut g, flail).is_empty());
    // Nor does a card alone pay the discard way, whose total is the tax's {1} (CR 601.2f).
    let (mut g, flail) = game(1, 0);
    put(&mut g, &TAX, Zone::BATTLEFIELD);
    assert!(offered(&mut g, flail).is_empty());
    // Four mana: offered, and all four paid.
    let (mut g, flail) = game(0, 4);
    put(&mut g, &TAX, Zone::BATTLEFIELD);
    let r = activate(&mut g, flail, false);
    pay(&mut g, r);
    assert!(on_stack(&g, flail));
    assert_eq!(g.state.player(YOU).pool.total(), 0);
}

#[test]
fn a_cost_reduction_applies_to_the_ways_mana() {
    // {3} less {2}: one mana pays it, and only one is spent.
    let (mut g, flail) = game(0, 2);
    put(&mut g, &DISCOUNT, Zone::BATTLEFIELD);
    assert_eq!(offered(&mut g, flail), vec![None]);
    let r = activate(&mut g, flail, false);
    pay(&mut g, r);
    assert!(on_stack(&g, flail));
    assert_eq!(g.state.player(YOU).pool.total(), 1);
}

#[test]
fn an_alternative_cost_is_paid_rather_than_any_way() {
    // No card and no mana: the ability's own cost can't be paid, but {0} rather than it can (CR 118.9).
    let (mut g, flail) = game(0, 0);
    put(&mut g, &SMITH, Zone::BATTLEFIELD);
    assert!(matches!(offered(&mut g, flail)[..], [Some(_)]));
    // With a card and {3}, both; for {0}, no way is chosen and nothing is paid.
    let (mut g, flail) = game(1, 3);
    put(&mut g, &SMITH, Zone::BATTLEFIELD);
    assert!(matches!(offered(&mut g, flail)[..], [None, Some(_)]));
    let r = activate(&mut g, flail, true);
    assert!(!matches!(r.kind, RequestKind::ChooseModes { .. }), "no way is chosen: {:?}", r.kind);
    pay(&mut g, r);
    assert!(on_stack(&g, flail));
    assert_eq!((g.state.player(YOU).pool.total(), hand(&g)), (3, 1));
}

/// The candidates the ability of `source` is offered with at `YOU`'s priority, if it is offered.
fn candidates(g: &mut Game, source: ObjectId) -> Option<Vec<Target>> {
    priority_options(&mut cx(g), YOU).iter().find(|o| o.source == Some(source)).map(|o| o.target_candidates.clone())
}

#[test]
fn with_a_cost_change_in_play_each_target_is_judged_with_each_way() {
    // The tax makes the discard way {1} and the mana way {4}: one mana and a card pay the discard way, so the ability is
    // offered with its target - though the first way, in mana, could not be paid.
    let (mut g, hook) = game_with(&HOOK, 1, 1);
    put(&mut g, &TAX, Zone::BATTLEFIELD);
    assert!(candidates(&mut g, hook).is_some_and(|c| c.contains(&Target::Object(hook))), "the discard way pays for it");
    // Without the card, neither way: the mana way is {4}.
    let (mut g, hook) = game_with(&HOOK, 0, 1);
    put(&mut g, &TAX, Zone::BATTLEFIELD);
    assert_eq!(candidates(&mut g, hook), None);

    // A reduction that reads the target makes the mana way {1} only for a permanent of yours: with one mana and no card, the
    // ability is offered with those targets alone, not the opponent's land, for which it would cost {3}.
    let (mut g, hook) = game_with(&HOOK, 0, 1);
    let lure = put(&mut g, &LURE, Zone::BATTLEFIELD);
    let theirs = g.state.push_obj(Obj::new(g.cards.id_of(&FILLER), PlayerId(1), Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, theirs);
    g.state.see_statics();
    let offered = candidates(&mut g, hook).expect("offered for a permanent of yours");
    assert!(offered.contains(&Target::Object(hook)) && offered.contains(&Target::Object(lure)), "{offered:?}");
    assert!(!offered.contains(&Target::Object(theirs)), "{{3}} for the opponent's land can't be paid: {offered:?}");
}
