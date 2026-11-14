//! Text-defined X is not chosen freely; printed limits constrain announcement and availability.
use crate::{Cx, Game, Setup};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}
fn counters(q: &dyn Query, a: &Asker, _: &[Target]) -> u32 {
    q.state().obj(a.source).counters(CounterKind::Charge).max(0) as u32
}
fn two(_: &dyn Query, _: &Asker, _: &[Target]) -> u32 {
    2
}
fn zero(_: &dyn Query, _: &Asker, _: &[Target]) -> u32 {
    0
}
fn opponents(_: &dyn Query, a: &Asker, p: PlayerId) -> bool {
    p != a.controller
}
fn graveyard(q: &dyn Query, _: &Asker, p: PlayerId) -> u32 {
    q.state().zone(Zone::graveyard(p)).len() as u32
}
fn a_spell(q: &dyn Query, _: &Asker, t: Target) -> bool {
    t.object()
        .is_some_and(|id| q.state().obj(id).zone.is(ZoneKind::Stack))
}
fn twice_mana_value(q: &dyn Query, _: &Asker, t: &[Target]) -> u32 {
    t.first()
        .and_then(|t| t.object())
        .map_or(0, |id| q.mana_value(id).saturating_mul(2))
}
static FILL: CardDef = CardDef {
    name: "Computed X filler",
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};
static FIXED: CardDef = CardDef {
    name: "Computed X source",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(
        cost("{X}"),
        nothing,
        "X is this permanent's charge counters.",
    )
    .x_is(counters)],
    ..CardDef::BLANK
};
static DOUBLE: CardDef = CardDef {
    name: "Computed X double",
    abilities: &[ability(
        cost("{X}{X}, {T}"),
        nothing,
        "X is this permanent's charge counters.",
    )
    .x_is(counters)],
    ..FIXED
};
static SELF_MANA: CardDef = CardDef {
    name: "Computed X changes during payment",
    abilities: &[
        ability(
            cost("{X}"),
            nothing,
            "X is this permanent's charge counters.",
        )
        .x_is(counters),
        mana_ability(
            remove_counters(CounterKind::Charge, 1),
            add_n(Mana::C, 3),
            "Remove a charge counter: Add {C}{C}{C}.",
        ),
    ],
    ..FIXED
};
static MIRROR: CardDef = CardDef {
    name: "Computed X targeted source",
    abilities: &[ability(
        cost("{X}, {T}"),
        nothing,
        "X is twice the target spell's mana value.",
    )
    .targets(&[target(a_spell)])
    .x_is(twice_mana_value)],
    ..FIXED
};
static TARGET: CardDef = CardDef {
    name: "Computed X target spell",
    cost: mana("{2}"),
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], nothing),
    ..CardDef::BLANK
};
static LIMITED: CardDef = CardDef {
    name: "Computed X limited spell",
    cost: mana("{X}"),
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], nothing),
    x_rule: Some(XRule::AtMost(two)),
    ..CardDef::BLANK
};
static PLAYER: CardDef = CardDef {
    name: "Computed X chosen graveyard spell",
    x_rule: Some(XRule::OfPlayer {
        players: opponents,
        value: graveyard,
    }),
    ..LIMITED
};
static FREE_FIXED: CardDef = CardDef {
    name: "Computed X fixed free alternative",
    x_rule: Some(XRule::Is(two)),
    x_cant_be_zero: true,
    ways: &[way(Cost::NONE, ZoneKind::Hand, "Pay nothing instead.")],
    ..LIMITED
};
static ZERO_LIMIT: CardDef = CardDef {
    name: "Computed X zero upper bound",
    x_cant_be_zero: true,
    x_rule: Some(XRule::AtMost(zero)),
    ..LIMITED
};
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: usize::MAX,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: Vec::new(),
        damage: None,
        asked_at: None,
        doer: Some(A),
        producing: None,
    }
}
fn board() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &FILL,
        &FIXED,
        &DOUBLE,
        &SELF_MANA,
        &MIRROR,
        &TARGET,
        &LIMITED,
        &PLAYER,
        &FREE_FIXED,
        &ZERO_LIMIT,
    ])));
    let deck = vec![cards.id_of(&FILL); 30];
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![deck; 3],
            shuffle: false,
            ..Default::default()
        },
    );
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 2;
        t.active = A;
        t.priority = A;
        t.began = true;
        t.set_step(Step::PrecombatMain);
    });
    g.state.update_player(A, |p| p.turn_began = 2);
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}
fn put(g: &mut Game, def: &'static CardDef, p: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), p, zone));
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}
fn charges(g: &mut Game, id: ObjectId, n: i32) {
    cx(g).add_counters(id, CounterKind::Charge, n);
}
fn pool(g: &mut Game, n: u32) {
    g.state
        .add_mana(A, Mana::C, n, ObjectId::NONE, None, ManaLasts::Step, false);
}
fn options(g: &mut Game) -> Vec<ActionOption> {
    crate::actions::priority_options(&mut cx(g), A)
}
fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("request")
}
fn answer(g: &mut Game, a: Answer) {
    let r = g.pending().cloned().unwrap();
    g.answer(r.id, r.player, a).unwrap();
}
fn act(g: &mut Game, id: ObjectId, index: u8, targets: Vec<Target>) {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else {
        panic!("{:?}", r.kind)
    };
    let o = options
        .iter()
        .find(|o| o.source == Some(id) && o.index == index)
        .expect("offered");
    let mut a = Action::of(o);
    a.targets = targets;
    answer(g, Answer::Act(a));
}
fn pay(g: &mut Game) {
    for _ in 0..30 {
        let r = next(g);
        match &r.kind {
            RequestKind::PayCosts {
                announcing: false,
                options,
                ..
            } => {
                let o = [
                    ActionKind::AssignMana,
                    ActionKind::ConfirmCasting,
                    ActionKind::PayCost,
                ]
                .into_iter()
                .find_map(|k| options.iter().find(|o| o.kind == k))
                .expect("payable");
                answer(g, Answer::Act(Action::of(o)));
            }
            RequestKind::Priority { .. } => return,
            other => panic!("unexpected payment request: {other:?}"),
        }
    }
    panic!("payment did not finish")
}
#[test]
fn fixed_x_requires_the_defined_cost_and_does_not_offer_a_free_zero_activation() {
    let mut g = board();
    let s = put(&mut g, &FIXED, A, Zone::BATTLEFIELD);
    charges(&mut g, s, 3);
    pool(&mut g, 2);
    assert!(!options(&mut g).iter().any(|o| o.source == Some(s)));
    pool(&mut g, 1);
    assert!(options(&mut g).iter().any(|o| o.source == Some(s)));
    act(&mut g, s, 0, vec![]);
    pay(&mut g);
    assert_eq!(g.state.stack.last().unwrap().x, 3);
    assert_eq!(g.state.player(A).pool.total(), 0);
}
#[test]
fn multiple_x_symbols_pay_the_same_defined_value() {
    let mut g = board();
    let s = put(&mut g, &DOUBLE, A, Zone::BATTLEFIELD);
    charges(&mut g, s, 3);
    pool(&mut g, 5);
    assert!(!options(&mut g).iter().any(|o| o.source == Some(s)));
    pool(&mut g, 1);
    act(&mut g, s, 0, vec![]);
    pay(&mut g);
    assert_eq!(g.state.stack.last().unwrap().x, 3);
    assert!(g.state.obj(s).tapped);
    assert_eq!(g.state.player(A).pool.total(), 0);
}
#[test]
fn defined_cost_is_locked_before_a_mana_ability_changes_its_counter_count() {
    let mut g = board();
    let s = put(&mut g, &SELF_MANA, A, Zone::BATTLEFIELD);
    charges(&mut g, s, 3);
    act(&mut g, s, 0, vec![]);
    let r = next(&mut g);
    let RequestKind::PayCosts {
        announcing: false,
        options,
        ..
    } = &r.kind
    else {
        panic!("{:?}", r.kind)
    };
    let o = options
        .iter()
        .find(|o| o.source == Some(s) && o.index == 1)
        .expect("mana ability");
    answer(&mut g, Answer::Act(Action::of(o)));
    pay(&mut g);
    assert_eq!(g.state.obj(s).counters(CounterKind::Charge), 2);
    assert_eq!(g.state.stack.last().unwrap().x, 3);
    assert_eq!(g.state.player(A).pool.total(), 0);
}
#[test]
fn a_preselected_target_defines_x_and_unpayable_target_choices_are_not_offered() {
    let mut g = board();
    let s = put(&mut g, &MIRROR, A, Zone::BATTLEFIELD);
    let t = put(&mut g, &TARGET, A, Zone::hand(A));
    pool(&mut g, 5);
    act(&mut g, t, 0, vec![]);
    pay(&mut g);
    let t = g.state.stack.last().unwrap().object;
    assert!(!options(&mut g).iter().any(|o| o.source == Some(s)));
    pool(&mut g, 1);
    let o = options(&mut g)
        .into_iter()
        .find(|o| o.source == Some(s))
        .expect("four mana now available");
    assert!(o.target_candidates.contains(&Target::Object(t)));
    let mut g = board();
    let s = put(&mut g, &MIRROR, A, Zone::BATTLEFIELD);
    let t = put(&mut g, &TARGET, A, Zone::hand(A));
    pool(&mut g, 6);
    act(&mut g, t, 0, vec![]);
    pay(&mut g);
    let t = g.state.stack.last().unwrap().object;
    act(&mut g, s, 0, vec![Target::Object(t)]);
    pay(&mut g);
    assert_eq!(g.state.stack.last().unwrap().x, 4);
    assert_eq!(g.state.player(A).pool.total(), 0);
}
#[test]
fn a_spell_upper_bound_refuses_an_out_of_range_announcement() {
    let mut g = board();
    let s = put(&mut g, &LIMITED, A, Zone::hand(A));
    pool(&mut g, 10);
    act(&mut g, s, 0, vec![]);
    let r = next(&mut g);
    let RequestKind::PayCosts {
        announcing: true,
        most,
        ..
    } = r.kind
    else {
        panic!("{:?}", r.kind)
    };
    assert_eq!(most, 2);
    assert!(g.answer(r.id, r.player, Answer::Announce(3)).is_err());
    answer(&mut g, Answer::Announce(2));
    pay(&mut g);
    assert_eq!(g.state.stack.last().unwrap().x, 2);
    assert_eq!(g.state.player(A).pool.total(), 8);
}
#[test]
fn a_zero_upper_bound_conflicts_with_x_cant_be_zero() {
    let mut g = board();
    let s = put(&mut g, &ZERO_LIMIT, A, Zone::hand(A));
    pool(&mut g, 10);
    assert!(!options(&mut g).iter().any(|o| o.source == Some(s)));
}
#[test]
fn choosing_an_opponent_defines_x_from_that_graveyard_as_cast() {
    let mut g = board();
    let s = put(&mut g, &PLAYER, A, Zone::hand(A));
    put(&mut g, &FILL, B, Zone::graveyard(B));
    for _ in 0..3 {
        put(&mut g, &FILL, C, Zone::graveyard(C));
    }
    pool(&mut g, 3);
    act(&mut g, s, 0, vec![]);
    let r = next(&mut g);
    let RequestKind::ChoosePlayer { candidates, .. } = &r.kind else {
        panic!("{:?}", r.kind)
    };
    assert_eq!(candidates.len(), 2);
    assert!(candidates.contains(&B) && candidates.contains(&C));
    answer(&mut g, Answer::Player(C));
    pay(&mut g);
    let item = g.state.stack.last().unwrap();
    assert_eq!(item.x, 3);
    assert_eq!(g.state.obj(item.object).x, 3);
    assert_eq!(g.state.player(A).pool.total(), 0);
    let card = g.state.zone(Zone::graveyard(C))[0];
    cx(&mut g).exile(card);
    assert_eq!(g.state.stack.last().unwrap().x, 3, "as cast is frozen");
}
#[test]
fn a_player_choice_with_an_unpayable_defined_value_is_not_offered() {
    let mut g = board();
    let s = put(&mut g, &PLAYER, A, Zone::hand(A));
    put(&mut g, &FILL, B, Zone::graveyard(B));
    for _ in 0..3 {
        put(&mut g, &FILL, C, Zone::graveyard(C));
    }
    pool(&mut g, 1);
    act(&mut g, s, 0, vec![]);
    pay(&mut g);
    assert_eq!(g.state.stack.last().unwrap().x, 1);
}
#[test]
fn text_defined_x_is_retained_for_a_free_alternative_cost() {
    let mut g = board();
    let s = put(&mut g, &FREE_FIXED, A, Zone::hand(A));
    let offered = options(&mut g);
    assert!(offered.iter().any(|o| o.source == Some(s) && o.index == 1));
    act(&mut g, s, 1, vec![]);
    pay(&mut g);
    assert_eq!(g.state.stack.last().unwrap().x, 2);
    assert_eq!(g.state.player(A).pool.total(), 0);
}
#[test]
fn defined_native_values_above_the_ordinary_numeric_prompt_cap_are_not_clamped() {
    let mut g = board();
    let s = put(&mut g, &FIXED, A, Zone::BATTLEFIELD);
    charges(&mut g, s, 1500);
    pool(&mut g, 1500);
    act(&mut g, s, 0, vec![]);
    pay(&mut g);
    assert_eq!(g.state.stack.last().unwrap().x, 1500);
    assert_eq!(g.state.player(A).pool.total(), 0);
}
