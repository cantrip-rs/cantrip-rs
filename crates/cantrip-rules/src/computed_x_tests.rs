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
static FILL: CardDef = CardDef {
    name: "Computed X filler",
    types: TypeSet::of(&[CardType::Land]),
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
static FREE_PLAYER: CardDef = CardDef {
    name: "Computed X chosen graveyard free alternative",
    ways: &[way(Cost::NONE, ZoneKind::Hand, "Pay nothing instead.")],
    ..PLAYER
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
        reached: None,
    }
}
fn board() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &FILL,
        &LIMITED,
        &PLAYER,
        &FREE_PLAYER,
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
    let s = put(&mut g, &FREE_PLAYER, A, Zone::hand(A));
    for _ in 0..2 {
        put(&mut g, &FILL, B, Zone::graveyard(B));
    }
    let offered = options(&mut g);
    assert!(offered.iter().any(|o| o.source == Some(s) && o.index == 1));
    act(&mut g, s, 1, vec![]);
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChoosePlayer { .. }), "{:?}", r.kind);
    answer(&mut g, Answer::Player(B));
    pay(&mut g);
    assert_eq!(g.state.stack.last().unwrap().x, 2);
}
