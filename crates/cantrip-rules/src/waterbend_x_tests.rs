//! "Waterbend {X}" (CR 701.67a-b, 107.3a), over test cards: the {X} is the waterbend cost's, so tapping artifacts and
//! creatures may pay it - judged at the least X may be announced as (1, where X can't be 0), and paid at the X announced
//! (Katara, Water Tribe's Hope).

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Waterbend X Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Waterbend {X}: Nothing. X can't be 0." on an enchantment, so it is nothing to tap for it itself.
static BENDER: CardDef = CardDef {
    name: "Waterbend X Test Bender",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[ability(of_mana(mana("{X}")), nothing, "Waterbend {X}: Nothing. X can't be 0.").waterbend().x_cant_be_zero()],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef { name: "Waterbend X Test Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

/// Your precombat main phase with priority, the Bender on the battlefield and `bears` Bears beside it.
fn game(bears: usize) -> (Game, ObjectId, Vec<ObjectId>) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &BENDER, &BEAR])));
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
    let bender = put(&mut g, &BENDER);
    let bears = (0..bears).map(|_| put(&mut g, &BEAR)).collect();
    (g, bender, bears)
}

fn put(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn offered(g: &mut Game, source: ObjectId) -> bool {
    priority_options(&mut cx(g), YOU).iter().any(|o| o.source == Some(source))
}

/// CR 701.67a, 107.3a: with no mana at all, one creature to tap pays the waterbend cost at X 1, so the ability is offered;
/// with none, it is not.
#[test]
fn waterbend_x_is_offered_where_tapping_could_pay_it_at_the_least_x() {
    let (mut g, bender, _) = game(1);
    assert!(offered(&mut g, bender), "a Bear tapped for it pays Waterbend {{X}} at X 1");
    let (mut g, bender, _) = game(0);
    assert!(!offered(&mut g, bender), "nothing pays X 1");
}

/// CR 701.67b: announced as X 2, each of two creatures tapped pays {1} of it, and the activation is paid with no mana.
#[test]
fn waterbend_x_announced_as_two_is_paid_by_tapping_two_creatures() {
    let (mut g, bender, bears) = game(2);
    let r = next(&mut g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(bender)).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "expected the announcement of X, got {:?}", r.kind);
    g.answer(r.id, r.player, Answer::Announce(2)).expect("2 may be announced");
    for &bear in &bears {
        let r = next(&mut g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("expected a payment, got {:?}", r.kind) };
        let tap = options
            .iter()
            .find(|o| o.kind == ActionKind::TapToPay && o.index == PAY_WATERBEND && o.source == Some(bear))
            .unwrap_or_else(|| panic!("the Bear may be tapped for the {{X}} announced, among {options:?}"))
            .clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&tap))).expect("a legal answer");
    }
    while let Some(RequestKind::PayCosts { options, .. }) = g.pending().map(|r| r.kind.clone()) {
        let r = g.pending().cloned().unwrap();
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("nothing more is owed").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&confirm))).expect("a legal answer");
        g.run();
    }
    assert!(bears.iter().all(|&b| g.state.obj(b).tapped), "both Bears tapped for it");
    assert_eq!(g.state.stack.last().map(|i| (i.object, i.x)), Some((bender, 2)), "the ability is on the stack with X 2");
}
