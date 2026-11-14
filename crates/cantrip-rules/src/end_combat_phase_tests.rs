//! An effect ends the combat phase (CR 724.2): the game skips straight to the next phase, the end of combat step skipped so
//! "at end of combat" abilities don't trigger (724.2d-e), and an ability that triggers during the process goes on the stack
//! in that next phase, the active player then getting priority (724.2f). Over test cards: the spell ends the combat phase
//! and then makes its controller lose 1 life, which a watcher's "whenever a player loses life" sees while the process is
//! still under way; the same watcher's "at end of combat" ability would make that player lose 5 more.

use crate::{Game, Setup};
use cantrip_api::*;

const P0: PlayerId = PlayerId(0);

fn end_combat_then_lose_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.end_the_combat_phase()?;
    g.lose_life(item.controller, 1);
    Ok(())
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn lose_five(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 5);
    Ok(())
}

fn life_lost(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    if matches!(event, Event::LifeChange { negative: true, .. }) { Fired::PLAIN } else { None }
}

fn end_of_combat(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    if matches!(event, Event::StepBegan { step: Step::EndOfCombat }) { Fired::PLAIN } else { None }
}

static FILLER: CardDef = CardDef { name: "End Combat Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static ENDER: CardDef = CardDef {
    name: "End Combat Test Instant",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], end_combat_then_lose_one),
    ..CardDef::BLANK
};
static WATCHER: CardDef = CardDef {
    name: "End Combat Test Watcher",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        triggered(when(EventMask::LIFE_CHANGE, life_lost), nothing, "Whenever a player loses life, nothing happens."),
        triggered(when(EventMask::STEP_BEGAN, end_of_combat), lose_five, "At end of combat, you lose 5 life."),
    ],
    ..CardDef::BLANK
};

fn game() -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &ENDER, &WATCHER])));
    let filler = table.id_of(&FILLER);
    let mut first = vec![table.id_of(&ENDER)];
    first.extend(std::iter::repeat_n(filler, 30));
    let mut g = Game::new(table, &Setup { decks: vec![first, vec![filler; 30]], ..Default::default() });
    let watcher = g.state.push_obj(Obj::new(table.id_of(&WATCHER), P0, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, watcher);
    (g, watcher)
}

/// Runs the game, passing and declaring nothing, until `stop` says the question asked is the one wanted.
fn until(g: &mut Game, stop: impl Fn(&Game, &Request) -> bool) -> Request {
    for _ in 0..200 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        if stop(g, &request) {
            return request;
        }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the question never came");
}

#[test]
fn ending_the_combat_phase_skips_end_of_combat_and_puts_what_triggered_on_the_stack_in_the_next_phase() {
    let (mut g, watcher) = game();
    let ask = until(&mut g, |g, r| {
        r.player == P0 && g.state.turn.step() == Step::BeginCombat && matches!(r.kind, RequestKind::Priority { .. })
    });
    let RequestKind::Priority { options } = &ask.kind else { unreachable!() };
    let ender = g.cards.id_of(&ENDER);
    let option = options
        .iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| g.state.obj(s).card == ender))
        .expect("the instant is offered");
    g.answer(ask.id, P0, Answer::Act(Action::of(option))).unwrap();

    // The instant resolves; the next priority is the active player's, in the postcombat main phase, with the life-loss
    // trigger on the stack (CR 724.2f) and the instant exiled (724.2b).
    let next = until(&mut g, |g, _| g.state.turn.step() != Step::BeginCombat);
    assert_eq!(g.state.turn.step(), Step::PostcombatMain, "the game did not skip straight to the next phase");
    assert_eq!(next.player, P0, "the active player was not given priority");
    assert!(matches!(next.kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.stack.len(), 1, "the ability that triggered during the process is not on the stack");
    assert_eq!(g.state.stack[0].object, watcher);
    assert!(g.state.zone(Zone::exile(P0)).iter().any(|&id| g.state.obj(id).card == ender), "the instant was not exiled");
    assert_eq!(g.state.player(P0).life, 19);

    // On to the next turn: "at end of combat" never triggered, the end of combat step having been skipped (CR 724.2e).
    until(&mut g, |g, _| g.state.turn.number == 2);
    assert_eq!(g.state.player(P0).life, 19, "an \"at end of combat\" ability triggered");
    assert!(
        !g.log.iter().any(|r| matches!(r.event, Event::StepBegan { step: Step::EndOfCombat }) && r.turn == 1),
        "the end of combat step was not skipped"
    );
}
