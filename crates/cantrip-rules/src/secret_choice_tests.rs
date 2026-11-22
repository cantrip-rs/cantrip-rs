//! A creature type chosen in secret and revealed as a cost (`Kept::SecretSubtype`, `Cost::reveal_secret_type`; CR 118.3,
//! 400.7, 601.2h, 607.2d, 608.2h), over a test permanent shaped as A Killer Among Us is: "Secretly choose Human or Goblin" and
//! "Sacrifice this, Reveal the creature type you chose: Note the type revealed". No card in content has one yet, so no scenario
//! can hold this: A Killer Among Us's own will.
//!
//! The type is kept for the player who chose it, and a later choice of the same ability replaces it. Only that player can
//! reveal it, and only while the permanent is the one that chose it: one another player has gained control of, or that chose
//! nothing, offers no such ability. Paying the cost puts the type on the ability on the stack, where all can see it, before the
//! permanent is sacrificed, so the ability resolves with the type of a permanent that is gone (CR 608.2h). Each permanent keeps
//! its own (the card's ruling), and every answer is round-tripped by `Game::verify`.

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
use std::cell::Cell;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

thread_local! {
    /// What the last resolved reveal ability read as the revealed type.
    static NOTED: Cell<Option<Subtype>> = const { Cell::new(None) };
}

static FILLER: CardDef = CardDef { name: "Secret Choice Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Secretly choose Human or Goblin."
fn choose_in_secret(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let ask = RequestKind::ChooseSubtype { source: item.object, choices: vec![Subtype::Human, Subtype::Goblin], purpose: WordUse::Chosen };
    let Answer::Subtype(chosen) = g.ask(item.controller, ask)? else { unreachable!("a subtype") };
    let by = g.linked(item);
    g.remember_secret_subtype(by, chosen, item.controller);
    Ok(())
}

/// "Note the type revealed."
fn note_the_type(_: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    NOTED.with(|n| n.set(item.announced_type()));
    Ok(())
}

const CHOOSE: u8 = 0;
const REVEAL: u8 = 1;
const REVEAL_FOR_MANA: u8 = 2;

static CHOOSER: CardDef = CardDef {
    name: "Secret Choice Test Chooser",
    cost: ManaCost::ZERO,
    has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[
        ability(cost("{0}"), choose_in_secret, "{0}: Secretly choose Human or Goblin."),
        ability(SACRIFICE_THIS.and(Cost { reveal_secret_type: true, ..Cost::NONE }), note_the_type, "Sacrifice this enchantment, Reveal the creature type you chose: Note it."),
        ability(cost("{0}").and(Cost { reveal_secret_type: true, ..Cost::NONE }), note_the_type, "{0}, Reveal the creature type you chose: Note it."),
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
    NOTED.with(|n| n.set(None));
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

/// Nothing is chosen, nothing can be revealed: the cost cannot be paid (CR 118.3).
#[test]
fn nothing_is_offered_to_reveal_before_a_choice() {
    let mut g = game();
    let c = chooser(&mut g);
    assert!(offered(&mut g, YOU, c, REVEAL).is_none());
    assert!(offered(&mut g, YOU, c, REVEAL_FOR_MANA).is_none());
    choose(&mut g, c, Subtype::Goblin);
    assert!(offered(&mut g, YOU, c, REVEAL).is_some());
    assert!(offered(&mut g, YOU, c, REVEAL_FOR_MANA).is_some());
}

/// The type is on the ability the moment the cost is paid, for all to see (CR 601.2h), before the sacrifice that follows it
/// has taken the permanent away; and the ability resolves reading it from itself (CR 608.2h, 400.7).
#[test]
fn the_revealed_type_is_on_the_ability_and_outlives_the_sacrifice() {
    let mut g = game();
    let c = chooser(&mut g);
    choose(&mut g, c, Subtype::Goblin);
    assert!(g.state.stack.is_empty(), "nothing is on the stack");
    put_on_the_stack(&mut g, c, REVEAL);
    let item = *g.state.stack.last().expect("the ability");
    assert_eq!(item.announced_type(), Some(Subtype::Goblin));
    assert!(g.state.obj(c).became.is_some(), "the sacrifice was paid too: it is another object now");
    resolve(&mut g, Subtype::Human);
    assert_eq!(NOTED.with(Cell::get), Some(Subtype::Goblin));
}

/// A reveal that is not paired with a sacrifice leaves the permanent be; the type is the one it chose.
#[test]
fn a_reveal_alone_leaves_the_permanent_and_its_choice() {
    let mut g = game();
    let c = chooser(&mut g);
    choose(&mut g, c, Subtype::Human);
    put_on_the_stack(&mut g, c, REVEAL_FOR_MANA);
    assert_eq!(g.state.stack.last().and_then(|i| i.announced_type()), Some(Subtype::Human));
    resolve(&mut g, Subtype::Goblin);
    assert_eq!(NOTED.with(Cell::get), Some(Subtype::Human));
    assert!(g.state.obj(c).became.is_none(), "not sacrificed");
    assert_eq!(g.state.secret_subtype(own(&g, c)), Some((Subtype::Human, YOU)), "revealed, still chosen");
}

/// Only the player who chose can reveal: with the permanent under another player's control it offers them nothing, and its
/// controller no longer has a choice to reveal (the card's ruling).
#[test]
fn another_player_who_gains_control_cannot_reveal() {
    let mut g = game();
    let c = chooser(&mut g);
    choose(&mut g, c, Subtype::Goblin);
    g.state.update_obj(c, |o| o.controller = THEM);
    assert!(priority_options(&mut cx(&mut g), THEM).iter().all(|o| !(o.source == Some(c) && o.index == REVEAL)), "not theirs to reveal");
    g.state.update_obj(c, |o| o.controller = YOU);
    assert!(offered(&mut g, YOU, c, REVEAL).is_some(), "theirs only for as long as they chose it");
}

/// Each permanent keeps its own choice, revealed as its own ability (the card's ruling).
#[test]
fn each_permanent_keeps_its_own_choice() {
    let mut g = game();
    let (a, b) = (chooser(&mut g), chooser(&mut g));
    choose(&mut g, a, Subtype::Goblin);
    choose(&mut g, b, Subtype::Human);
    assert_eq!(g.state.secret_subtype(own(&g, a)), Some((Subtype::Goblin, YOU)));
    assert_eq!(g.state.secret_subtype(own(&g, b)), Some((Subtype::Human, YOU)));
    put_on_the_stack(&mut g, b, REVEAL);
    resolve(&mut g, Subtype::Goblin);
    assert_eq!(NOTED.with(Cell::get), Some(Subtype::Human));
    assert_eq!(g.state.secret_subtype(own(&g, a)), Some((Subtype::Goblin, YOU)), "the other is untouched");
}
