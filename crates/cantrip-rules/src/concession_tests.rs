//! A player conceding in a game of three while they are asked about one of their own game actions (CR 104.3a, 800.4a): the
//! action ends with them, and the game goes on without them - priority passes to the next player in turn order as their own
//! (800.4a), their triggered abilities are not put on the stack (800.4d), an active player who leaves declares no attack and
//! the turn goes on (800.4j), and a defending player who leaves blocks with nothing. Over test cards.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::PendingTrigger;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn never(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> {
    None
}

static FILLER: CardDef = CardDef { name: "Concession Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static SORCERY: CardDef = CardDef { name: "Concession Test Sorcery", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };
static INSTANT: CardDef = CardDef { name: "Concession Test Instant", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], nothing), ..CardDef::BLANK };
static ECHO: CardDef = CardDef {
    name: "Concession Test Echo",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, never), nothing, "Whenever ..., nothing happens.")],
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef {
    name: "Concession Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&FILLER, &SORCERY, &INSTANT, &ECHO, &BEAR])))
}

/// A game of three on turn 1, P0's deck starting with the Sorcery and P2's with the Instant, and `permanents` on the
/// battlefield.
fn game(permanents: &[(&'static CardDef, PlayerId)]) -> (Game, Vec<ObjectId>) {
    let table = table();
    let filler = table.id_of(&FILLER);
    let deck = |first: Option<&'static CardDef>| {
        let mut d: Vec<CardId> = first.map(|c| table.id_of(c)).into_iter().collect();
        d.extend(std::iter::repeat_n(filler, 30));
        d
    };
    let mut g = Game::new(table, &Setup { decks: vec![deck(Some(&SORCERY)), deck(None), deck(Some(&INSTANT))], ..Default::default() });
    let ids = permanents
        .iter()
        .map(|&(def, owner)| {
            let id = g.state.push_obj(Obj::new(table.id_of(def), owner, Zone::BATTLEFIELD));
            g.state.zone_push(Zone::BATTLEFIELD, id);
            id
        })
        .collect();
    (g, ids)
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

fn cast(options: &[ActionOption], g: &Game, def: &'static CardDef) -> Answer {
    let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| g.state.obj(s).card == g.cards.id_of(def)));
    Answer::Act(Action::of(option.expect("the card is offered")))
}

/// P0 casts the Sorcery and passes; P1 holds priority with it on the stack and concedes. Priority passes to P2 as P2's own,
/// with P2's options - the Instant to respond with - and no pass counted for P1: P2 responds, and the response is P2's.
#[test]
fn a_player_who_concedes_holding_priority_passes_it_to_the_next_as_their_own() {
    let (mut g, _) = game(&[]);
    let main = until(&mut g, |g, r| r.player == P0 && g.state.turn.step() == Step::PrecombatMain && matches!(r.kind, RequestKind::Priority { .. }));
    let RequestKind::Priority { options } = &main.kind else { unreachable!() };
    g.answer(main.id, P0, cast(options, &g, &SORCERY)).unwrap();
    let held = until(&mut g, |g, r| r.player == P1 && !g.state.stack.is_empty());
    assert!(matches!(held.kind, RequestKind::Priority { .. }));
    g.concede(P1);
    let next = g.pending().cloned().expect("a question");
    assert_eq!(next.player, P2, "priority did not pass to P2");
    let RequestKind::Priority { options } = &next.kind else { panic!("P2 was not given priority: {:?}", next.kind) };
    assert!(
        options.iter().any(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| g.state.obj(s).controller == P2)),
        "P2 was given P1's options, not their own"
    );
    g.answer(next.id, P2, cast(options, &g, &INSTANT)).unwrap();
    g.run();
    assert_eq!(g.state.stack.len(), 2, "P2's response is not on the stack");
    assert_eq!(g.state.stack[1].controller, P2, "the response is not P2's");
}

/// Two triggered abilities of P0's and two of P1's wait to go on the stack. If P0 concedes as they order theirs, none of P0's
/// goes on and P1's both do; if P1 concedes as they order theirs, P0's both go on and none of P1's (CR 800.4d).
#[test]
fn a_player_who_concedes_as_their_triggers_go_on_the_stack_puts_none_of_them_there() {
    for leaver in [P0, P1] {
        let (mut g, ids) = game(&[(&ECHO, P0), (&ECHO, P0), (&ECHO, P1), (&ECHO, P1)]);
        until(&mut g, |_, r| matches!(r.kind, RequestKind::Priority { .. }));
        let echo = g.cards.id_of(&ECHO);
        for &id in &ids {
            let controller = g.state.obj(id).controller;
            g.state.trigger_push(PendingTrigger { event_amount: None,
                source: id,
                ability: AbilityRef { card: echo, index: 0 },
                controller,
                subject: Target::None,
                data: [0; 2],
                members: cantrip_state::Span::default(),
                count: None,
                since: None,
                acquired: Acquired::OWN,
                words: None,
            });
        }
        g.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Settle));
        g.run();
        let ask = g.pending().cloned().expect("an order asked");
        assert!(matches!(ask.kind, RequestKind::OrderTriggers { .. }) && ask.player == P0);
        if leaver == P0 {
            g.concede(P0);
        } else {
            g.answer(ask.id, P0, Answer::Order(vec![0, 1])).unwrap();
            g.run();
            let ask = g.pending().cloned().expect("an order asked");
            assert!(matches!(ask.kind, RequestKind::OrderTriggers { .. }) && ask.player == P1, "{:?}", ask.kind);
            g.concede(P1);
        }
        if leaver == P0 {
            let ask = g.pending().cloned().expect("an order asked");
            assert!(matches!(ask.kind, RequestKind::OrderTriggers { .. }) && ask.player == P1, "P1 was not asked: {:?}", ask.kind);
            g.answer(ask.id, P1, Answer::Order(vec![0, 1])).unwrap();
            g.run();
        }
        let stayed = if leaver == P0 { P1 } else { P0 };
        let on_stack: Vec<PlayerId> = g.state.stack.iter().filter(|i| i.kind == StackKind::Triggered).map(|i| i.controller).collect();
        assert_eq!(on_stack, vec![stayed, stayed], "{leaver:?} conceded: the triggers on the stack are wrong");
    }
}

/// P0, the active player, concedes while declaring attackers: no attack is declared, no one else declares one, and the turn
/// goes on without them (CR 800.4j).
#[test]
fn an_active_player_who_concedes_declaring_attackers_declares_none() {
    let (mut g, _) = game(&[(&BEAR, P0), (&BEAR, P1)]);
    until(&mut g, |_, r| matches!(r.kind, RequestKind::DeclareAttackers { .. }));
    g.concede(P0);
    assert!(g.state.player(P0).lost);
    assert!(g.state.attackers.is_empty(), "an attack was declared");
    let next = g.pending().cloned().expect("the turn goes on");
    assert!(!matches!(next.kind, RequestKind::DeclareAttackers { .. }), "another player was asked P0's attacks");
    assert_ne!(next.player, P0);
}

/// P0 attacks P1, who concedes while declaring blockers: P1 blocks with nothing, and no one else - P0, the attacker, least of
/// all - declares P1's blocks.
#[test]
fn a_defending_player_who_concedes_declaring_blockers_blocks_with_nothing() {
    let (mut g, ids) = game(&[(&BEAR, P0), (&BEAR, P1)]);
    let attack = until(&mut g, |_, r| matches!(r.kind, RequestKind::DeclareAttackers { .. }));
    g.answer(attack.id, P0, Answer::Attack(vec![(ids[0], Target::Player(P1))])).unwrap();
    let blocks = until(&mut g, |_, r| matches!(r.kind, RequestKind::DeclareBlockers { .. }));
    assert_eq!(blocks.player, P1);
    g.concede(P1);
    assert!(g.state.player(P1).lost);
    assert!(g.state.blocks.is_empty(), "a block was declared");
    let next = g.pending().cloned().expect("the turn goes on");
    assert!(!matches!(next.kind, RequestKind::DeclareBlockers { .. }), "another player was asked P1's blocks: {next:?}");
}
