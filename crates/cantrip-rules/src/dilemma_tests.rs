//! "You face a dilemma as you choose one or more modes for a spell or ability" (TRK; CR 700.2a, 700.2b, 601.2b, 603.3c):
//! choosing modes is an event "whenever you face a dilemma" triggers on, for the player who chose them; choosing none,
//! and a copy (CR 700.2g), is no dilemma. The count of them this turn is asked of the history with the same predicate.
use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::history::faced_a_dilemma;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Dilemma Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn noop(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static MODES: ModalDef = modes(&[mode(noop, "First mode."), mode(noop, "Second mode.")]);
static UP_TO_TWO: ModalDef = MODES.choose(0, 2);

static WATCHER: CardDef = CardDef {
    name: "Dilemma Watcher",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[triggered(triggers::YOU_FACE_A_DILEMMA, noop, "Whenever you face a dilemma, nothing happens.")],
    ..CardDef::BLANK
};

static MODAL_SPELL: CardDef = CardDef {
    name: "Dilemma Spell",
    cost: ManaCost::ZERO,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODES),
    ..CardDef::BLANK
};

static UP_TO_SPELL: CardDef = CardDef {
    name: "Dilemma Up To Spell",
    cost: ManaCost::ZERO,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: Some(SpellDef::MODAL),
    modal: Some(&UP_TO_TWO),
    ..CardDef::BLANK
};

static MODAL_ETB: CardDef = CardDef {
    name: "Dilemma Creature",
    cost: ManaCost::ZERO,
    has_cost: true,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[modal_triggered(THIS_ENTERS, &MODES, "When this creature enters, choose one — First mode; or Second mode.")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &WATCHER, &MODAL_SPELL, &UP_TO_SPELL, &MODAL_ETB])));
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

fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    id
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

/// Casts `spell` and chooses `modes` for it, confirming the casting if asked: the next priority.
fn cast(g: &mut Game, spell: ObjectId, modes: Vec<u8>) -> Request {
    let option = priority_options(&mut cx(g), YOU)
        .into_iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell))
        .expect("the spell is offered");
    let r = next(g);
    answer(g, &r, Answer::Act(Action::of(&option)));
    if !modes.is_empty() {
        let r = next(g);
        assert!(matches!(r.kind, RequestKind::ChooseModes { .. }), "expected the modes, got {:?}", r.kind);
        answer(g, &r, Answer::Modes(modes));
    }
    until_priority(g)
}

fn until_priority(g: &mut Game) -> Request {
    let mut r = next(g);
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("confirm casting");
        answer(g, &r, Answer::Act(Action::of(confirm)));
        r = next(g);
    }
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "expected priority, got {:?}", r.kind);
    r
}

/// Captain Benjamin Sisko's question: how many dilemmas `player` faced this turn.
fn dilemmas(g: &Game, player: PlayerId) -> usize {
    Query::history(g).this_turn().iter().filter(|r| faced_a_dilemma(&r.event) == Some(player)).count()
}

/// The "whenever you face a dilemma" abilities of `watcher` on the stack or waiting to be put there.
fn fired(g: &Game, watcher: ObjectId) -> usize {
    g.state.stack.iter().filter(|i| i.object == watcher).count() + g.state.triggers.iter().filter(|t| t.source == watcher).count()
}

#[test]
fn choosing_a_spells_modes_is_a_dilemma_for_its_caster_alone() {
    let mut g = game();
    let mine = put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let theirs = put(&mut g, &WATCHER, THEM, Zone::BATTLEFIELD);
    let spell = put(&mut g, &MODAL_SPELL, YOU, Zone::hand(YOU));
    cast(&mut g, spell, vec![0]);
    assert_eq!(fired(&g, mine), 1, "the caster faced a dilemma");
    assert_eq!(fired(&g, theirs), 0, "the opponent faced none");
    assert_eq!((dilemmas(&g, YOU), dilemmas(&g, THEM)), (1, 0));
}

#[test]
fn choosing_a_triggered_abilitys_mode_is_a_dilemma() {
    let mut g = game();
    let mine = put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let creature = put(&mut g, &MODAL_ETB, YOU, Zone::hand(YOU));
    cast(&mut g, creature, vec![]);
    for _ in 0..2 {
        let r = next(&mut g);
        answer(&mut g, &r, Answer::Pass);
    }
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseModes { .. }), "expected the trigger's mode, got {:?}", r.kind);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    until_priority(&mut g);
    assert_eq!(fired(&g, mine), 1, "the enter trigger's mode was a dilemma");
    assert_eq!(dilemmas(&g, YOU), 1);
}

#[test]
fn choosing_no_mode_is_no_dilemma() {
    let mut g = game();
    let mine = put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let spell = put(&mut g, &UP_TO_SPELL, YOU, Zone::hand(YOU));
    let r = priority_options(&mut cx(&mut g), YOU)
        .into_iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell))
        .expect("the spell is offered");
    let p = next(&mut g);
    answer(&mut g, &p, Answer::Act(Action::of(&r)));
    let ask = next(&mut g);
    assert!(matches!(ask.kind, RequestKind::ChooseModes { .. }), "expected the modes, got {:?}", ask.kind);
    answer(&mut g, &ask, Answer::Modes(vec![]));
    until_priority(&mut g);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::ModesChosen { player: YOU, modes: 0, .. })), "the choice of none is in the history");
    assert_eq!(fired(&g, mine), 0, "no mode chosen, no dilemma");
    assert_eq!(dilemmas(&g, YOU), 0);
}

#[test]
fn a_copy_chooses_no_modes_and_faces_no_dilemma() {
    let mut g = game();
    let mine = put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let spell = put(&mut g, &MODAL_SPELL, YOU, Zone::hand(YOU));
    cast(&mut g, spell, vec![1]);
    assert_eq!((fired(&g, mine), dilemmas(&g, YOU)), (1, 1));
    let cast = g.state.stack.iter().find(|i| i.kind == StackKind::Spell).expect("the spell is on the stack").object;
    let copy = crate::ops::copy_spell(&mut cx(&mut g), cast, YOU, false).expect("nothing is asked").expect("a copy");
    assert_eq!(g.state.stack.iter().find(|i| i.object == copy).map(|i| i.modes), Some(MODES.with(0, 1)), "the copy copied the mode");
    assert_eq!((fired(&g, mine), dilemmas(&g, YOU)), (1, 1), "the copy faced no dilemma");
}
