//! Expend (CR 700.14), over test cards: "A player expends N if they pay a cost to cast a spell and the amount of mana that
//! player spent this turn to cast spells prior to paying that cost was less than N and became at least N after paying that
//! cost." The watcher gains 1 life whenever its controller expends 4 and 10 whenever they expend 8, so its life says which
//! triggered, and how often.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::expend::expend;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn gain_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

fn gain_ten(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 10);
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Expend Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Whenever you expend 4, you gain 1 life. Whenever you expend 8, you gain 10 life."
static WATCHER: CardDef = CardDef {
    name: "Expend Test Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[
        triggered(expend::<4>(), gain_one, "Whenever you expend 4, you gain 1 life."),
        triggered(expend::<8>(), gain_ten, "Whenever you expend 8, you gain 10 life."),
    ],
    ..CardDef::BLANK
};

const fn sorcery(name: &'static str, cost: ManaCost) -> CardDef {
    CardDef { name, cost, types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK }
}

static ONE: CardDef = sorcery("Expend Test One", mana("{1}"));
static THREE: CardDef = sorcery("Expend Test Three", mana("{3}"));
static EIGHT: CardDef = sorcery("Expend Test Eight", mana("{8}"));
static FREE: CardDef = sorcery("Expend Test Free", mana("{0}"));
static X: CardDef = sorcery("Expend Test X", mana("{X}"));

/// "{2}: Nothing." Mana spent on an ability is not spent to cast a spell.
static ENGINE: CardDef = CardDef {
    name: "Expend Test Engine",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(of_mana(mana("{2}")), nothing, "{2}: Nothing.")],
    ..CardDef::BLANK
};

/// "Sorcery spells you cast cost {1} less to cast."
fn sorceries_cost_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(spell) if player == q.controller(source) && q.chars(spell).is(CardType::Sorcery) => CostDelta { less: mana("{1}"), ..CostDelta::NONE },
        _ => CostDelta::NONE,
    }
}

static REDUCER: CardDef = CardDef { name: "Expend Test Reducer", types: TypeSet::of(&[CardType::Artifact]), cost_changes: &[sorceries_cost_less], ..CardDef::BLANK };

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new([&FILLER, &WATCHER, &ONE, &THREE, &EIGHT, &FREE, &X, &ENGINE, &REDUCER])));
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

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    refresh(g);
    id
}

fn add_mana(g: &mut Game, n: u32) {
    g.state.add_mana(YOU, Mana::C, n, ObjectId::NONE, None, ManaLasts::Step, false);
    refresh(g);
}

/// The priority question already asked is asked again, of the changed game.
fn refresh(g: &mut Game) {
    if let Some(at) = g.state.agenda.len().checked_sub(1)
        && g.state.agenda[at].pending.as_ref().is_some_and(|r| matches!(r.kind, RequestKind::Priority { .. }))
    {
        g.state.update_frame(at, |f| f.pending = None);
    }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

/// Takes `source`'s first option at priority, announces X as `x` if asked, pays from the pool, and lets everything on the
/// stack resolve, so priority is back with nothing on the stack.
fn play(g: &mut Game, source: ObjectId, x: u32) {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(source) && o.index == 0).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    for _ in 0..40 {
        let r = next(g);
        match &r.kind {
            RequestKind::PayCosts { announcing: true, .. } => g.answer(r.id, r.player, Answer::Announce(x)).expect("a legal announcement"),
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
            }
            RequestKind::OrderTriggers { sources, .. } => g.answer(r.id, r.player, Answer::Order((0..sources.len() as u32).collect())).expect("an order"),
            RequestKind::Priority { .. } if g.state.stack.is_empty() => return,
            _ => g.answer(r.id, r.player, Answer::Pass).unwrap_or_else(|e| panic!("{e:?} for {:?}", r.kind)),
        }
    }
    panic!("the stack never emptied");
}

fn cast(g: &mut Game, def: &'static CardDef, x: u32) {
    let card = put(g, def, YOU, Zone::hand(YOU));
    add_mana(g, def.cost.generic + x);
    play(g, card, x);
}

fn life(g: &Game, player: PlayerId) -> i32 {
    g.state.player(player).life
}

/// One payment that takes the total from 0 to 8 expends 4 and 8 together: both trigger.
#[test]
fn one_payment_can_expend_four_and_eight() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let before = life(&g, YOU);
    cast(&mut g, &EIGHT, 0);
    assert_eq!(life(&g, YOU), before + 11);
    assert_eq!(g.history().expended(YOU, ObjectId::NONE), (8, 8), "the eight were spent to cast spells");
}

/// 3, 6, 9, 10: expend 4 triggers on the payment that crosses 4, expend 8 on the one that crosses 8, each once.
#[test]
fn each_number_is_expended_once_a_turn() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let start = life(&g, YOU);
    cast(&mut g, &THREE, 0);
    assert_eq!(life(&g, YOU), start, "3 is less than 4");
    cast(&mut g, &THREE, 0);
    assert_eq!(life(&g, YOU), start + 1, "3 to 6 expends 4");
    cast(&mut g, &THREE, 0);
    assert_eq!(life(&g, YOU), start + 11, "6 to 9 expends 8");
    cast(&mut g, &ONE, 0);
    assert_eq!(life(&g, YOU), start + 11, "9 to 10 expends nothing");
}

/// Mana spent on an ability is not spent to cast a spell; a spell cast for {0} spends none.
#[test]
fn ability_mana_and_free_spells_add_nothing() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let engine = put(&mut g, &ENGINE, YOU, Zone::BATTLEFIELD);
    let start = life(&g, YOU);
    for _ in 0..3 {
        add_mana(&mut g, 2);
        play(&mut g, engine, 0);
    }
    cast(&mut g, &THREE, 0);
    assert_eq!(life(&g, YOU), start, "six on abilities and three on a spell are three spent to cast spells");
    cast(&mut g, &FREE, 0);
    assert_eq!(life(&g, YOU), start, "a free spell adds none");
    cast(&mut g, &ONE, 0);
    assert_eq!(life(&g, YOU), start + 1, "the fourth mana spent on spells");
}

/// X counts the mana paid for it; a cost reduction leaves only the mana actually spent.
#[test]
fn x_and_cost_reductions_count_the_mana_spent() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let start = life(&g, YOU);
    cast(&mut g, &X, 3);
    assert_eq!(life(&g, YOU), start, "X of 3 is three");
    put(&mut g, &REDUCER, YOU, Zone::BATTLEFIELD);
    let three = put(&mut g, &THREE, YOU, Zone::hand(YOU));
    add_mana(&mut g, 2);
    play(&mut g, three, 0);
    assert_eq!(life(&g, YOU), start + 1, "{{3}} reduced to {{2}}: 3 to 5");
    cast(&mut g, &EIGHT, 0);
    assert_eq!(life(&g, YOU), start + 11, "{{8}} reduced to {{7}}: 5 to 12");
    assert_eq!(g.history().expended(YOU, ObjectId::NONE).0, 12);
}

/// Another player's spending is theirs: an opponent's watcher doesn't trigger on yours.
#[test]
fn expend_is_each_players_own() {
    let mut g = game();
    put(&mut g, &WATCHER, THEM, Zone::BATTLEFIELD);
    let (you, them) = (life(&g, YOU), life(&g, THEM));
    cast(&mut g, &EIGHT, 0);
    assert_eq!((life(&g, YOU), life(&g, THEM)), (you, them));
    assert_eq!(g.history().expended(THEM, ObjectId::NONE), (0, 0));
}

/// A permanent that enters after its controller spent 4 sees the payments before it: its expend 4 doesn't trigger on the
/// fifth mana, and its expend 8 still does on the eighth.
#[test]
fn a_permanent_entering_after_four_were_spent_waits_for_eight() {
    let mut g = game();
    let start = life(&g, YOU);
    cast(&mut g, &THREE, 0);
    cast(&mut g, &ONE, 0);
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    cast(&mut g, &ONE, 0);
    assert_eq!(life(&g, YOU), start, "4 to 5 expends nothing");
    cast(&mut g, &THREE, 0);
    assert_eq!(life(&g, YOU), start + 10, "5 to 8 expends 8");
}

/// The total is this turn's: a new turn starts it again.
#[test]
fn the_total_starts_again_each_turn() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let start = life(&g, YOU);
    cast(&mut g, &THREE, 0);
    cast(&mut g, &ONE, 0);
    assert_eq!(life(&g, YOU), start + 1);
    g.state.update_turn(|t| t.number += 1);
    cast(&mut g, &THREE, 0);
    assert_eq!(life(&g, YOU), start + 1, "three this turn");
    cast(&mut g, &ONE, 0);
    assert_eq!(life(&g, YOU), start + 2, "four this turn");
}
