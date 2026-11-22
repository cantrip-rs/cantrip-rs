//! A creature type chosen in secret (`Kept::SecretSubtype`; CR 607.2d), over a test permanent shaped as A Killer Among Us is:
//! "Secretly choose Human or Goblin". No card in content has one yet, so no scenario can hold this: A Killer Among Us's own will.
//!
//! The type is kept for the player who chose it, a later choice of the same ability replaces it, and each permanent keeps its
//! own (the card's ruling). Every answer is round-tripped by `Game::verify`.

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Secret Choice Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Secretly choose Human or Goblin."
fn choose_in_secret(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let ask = RequestKind::ChooseSubtype { source: item.object, choices: vec![Subtype::Human, Subtype::Goblin], purpose: WordUse::Chosen };
    let Answer::Subtype(chosen) = g.ask(item.controller, ask)? else { unreachable!("a subtype") };
    let by = g.linked(item);
    g.remember_secret_subtype(by, chosen, item.controller);
    Ok(())
}

const CHOOSE: u8 = 0;

static CHOOSER: CardDef = CardDef {
    name: "Secret Choice Test Chooser",
    cost: ManaCost::ZERO,
    has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[
        ability(cost("{0}"), choose_in_secret, "{0}: Secretly choose Human or Goblin."),
    ],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &CHOOSER])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 3;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.verify = true;
    g
}

fn chooser(g: &mut Game) -> ObjectId {
    let mut obj = Obj::new(g.cards.id_of(&CHOOSER), YOU, Zone::BATTLEFIELD);
    obj.timestamp = g.state.timestamp();
    let id = g.state.push_obj(obj);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn pay(g: &mut Game) {
    loop {
        let r = next(g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("it costs nothing").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
}

/// Passes until the stack is empty again, answering a subtype question with `answer`.
fn resolve(g: &mut Game, answer: Subtype) {
    loop {
        let r = next(g);
        match &r.kind {
            RequestKind::Priority { .. } if g.state.stack.is_empty() && g.state.triggers.is_empty() && r.player == YOU => break,
            RequestKind::Priority { .. } => g.answer(r.id, r.player, Answer::Pass).expect("a legal answer"),
            RequestKind::ChooseSubtype { .. } => g.answer(r.id, r.player, Answer::Subtype(answer)).expect("a legal answer"),
            other => panic!("unexpected {other:?}"),
        }
    }
}

fn offered(g: &mut Game, player: PlayerId, source: ObjectId, index: u8) -> Option<ActionOption> {
    priority_options(&mut cx(g), player).into_iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source) && o.index == index)
}

/// Activates `index` of `source`, paying what is asked, and leaves the ability on the stack.
fn put_on_the_stack(g: &mut Game, source: ObjectId, index: u8) {
    let option = offered(g, YOU, source, index).expect("offered");
    let r = next(g);
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    pay(g);
}

fn choose(g: &mut Game, source: ObjectId, subtype: Subtype) {
    put_on_the_stack(g, source, CHOOSE);
    resolve(g, subtype);
}

fn own(g: &Game, object: ObjectId) -> Linked {
    Linked { object, text: g.cards.linked_card(g.cards.id_of(&CHOOSER)), acquired: Acquired::OWN }
}

/// The type is kept for the player who chose it, to the ability's own text, and the chooser is recorded with it.
#[test]
fn the_type_chosen_in_secret_is_kept_with_who_chose_it() {
    let mut g = game();
    let c = chooser(&mut g);
    assert_eq!(g.state.secret_subtype(own(&g, c)), None, "nothing chosen yet");
    choose(&mut g, c, Subtype::Goblin);
    assert_eq!(g.state.secret_subtype(own(&g, c)), Some((Subtype::Goblin, YOU)));
}

/// "Secretly choose" again, by the same ability, is the permanent's choice now.
#[test]
fn a_later_choice_replaces_the_earlier_one() {
    let mut g = game();
    let c = chooser(&mut g);
    choose(&mut g, c, Subtype::Goblin);
    choose(&mut g, c, Subtype::Human);
    assert_eq!(g.state.secret_subtype(own(&g, c)), Some((Subtype::Human, YOU)));
}

/// Each permanent keeps its own choice (the card's ruling).
#[test]
fn each_permanent_keeps_its_own_choice() {
    let mut g = game();
    let (a, b) = (chooser(&mut g), chooser(&mut g));
    choose(&mut g, a, Subtype::Goblin);
    choose(&mut g, b, Subtype::Human);
    assert_eq!(g.state.secret_subtype(own(&g, a)), Some((Subtype::Goblin, YOU)));
    assert_eq!(g.state.secret_subtype(own(&g, b)), Some((Subtype::Human, YOU)));
    choose(&mut g, b, Subtype::Goblin);
    assert_eq!(g.state.secret_subtype(own(&g, a)), Some((Subtype::Goblin, YOU)), "the other is untouched");
    assert_eq!(g.state.secret_subtype(own(&g, b)), Some((Subtype::Goblin, YOU)));
}
