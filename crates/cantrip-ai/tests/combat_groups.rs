//! Restrictions on how many creatures a whole declaration has (CR 506.5, 508.1c, 509.1b; `cantrip_state::CombatGroup`) in
//! the AI player's terms: Caverns of Despair's "No more than two creatures can attack each combat" and "No more than two
//! creatures can block each combat", Errantry's "Enchanted creature can only attack alone", Orcish Conscripts' "can't
//! attack unless at least two other creatures attack" and its block twin. Every walk through the factoring reaches a
//! declaration the engine accepts, and every set of creatures the engine lets attack (or block) is reached by one.

use std::collections::BTreeSet;

use cantrip_ai::{Answering, Knowledge, observe_indexed};
use cantrip_engine::*;
use cantrip_obs::factor::{FactoredAnswer, Machine};
use cantrip_obs::{AskKind, InfoMode, Ref};
use cantrip_script::GameScript;

/// The participants of every answer a walk can reach, each walk's answer checked with the engine.
fn walked(game: &Game, request: &Request, answering: &Answering) -> BTreeSet<Vec<Ref>> {
    let mut reached = BTreeSet::new();
    let mut open = vec![Machine::new(&answering.ask, &[])];
    while let Some(m) = open.pop() {
        let Some(step) = m.step() else {
            let answer = m.answer().unwrap().clone();
            if let Err(r) = answering.engine(game, request, &answer) {
                panic!("walked {answer:?}: {} ({:?})", r.why, r.answer);
            }
            let (FactoredAnswer::Attack(pairs) | FactoredAnswer::Block(pairs)) = &answer else { panic!("{answer:?}") };
            let mut who: Vec<Ref> = pairs.iter().map(|p| p.0).collect();
            who.sort();
            who.dedup();
            reached.insert(who);
            continue;
        };
        let legal: Vec<usize> = step.legal().collect();
        assert!(!legal.is_empty(), "a step with nothing legal: {step:?}");
        for i in legal {
            let mut next = m.clone();
            next.choose(i);
            open.push(next);
        }
    }
    reached
}

/// Every set of the eligible creatures the engine accepts declared attacking (the first defender) or blocking (the first
/// attacker each may block).
fn legal(game: &Game, request: &Request, answering: &Answering) -> BTreeSet<Vec<Ref>> {
    let (eligible, pick): (Vec<Ref>, Box<dyn Fn(Vec<(Ref, Ref)>) -> FactoredAnswer>) = match &answering.ask.kind {
        AskKind::DeclareAttackers { eligible, .. } => (eligible.clone(), Box::new(FactoredAnswer::Attack)),
        AskKind::DeclareBlockers { eligible, .. } => (eligible.clone(), Box::new(FactoredAnswer::Block)),
        other => panic!("not a declaration: {other:?}"),
    };
    let to = |c: Ref| match &answering.ask.kind {
        AskKind::DeclareAttackers { defenders, .. } => defenders[0],
        AskKind::DeclareBlockers { allowed, .. } => allowed.iter().find(|p| p.0 == c).expect("a creature that may block").1,
        _ => unreachable!(),
    };
    let mut sets = BTreeSet::new();
    for bits in 0..1u32 << eligible.len() {
        let who: Vec<Ref> = eligible.iter().enumerate().filter(|(i, _)| bits >> i & 1 == 1).map(|p| *p.1).collect();
        if answering.engine(game, request, &pick(who.iter().map(|&c| (c, to(c))).collect())).is_ok() {
            let mut sorted = who.clone();
            sorted.sort();
            sets.insert(sorted);
        }
    }
    sets
}

/// The script's handed-over declaration, walked every way; the sizes of the sets of creatures the engine allows.
fn check(name: &str, script: &str) -> Vec<usize> {
    let script = GameScript::parse(script, name).unwrap();
    let result = cantrip_script::run(&script).unwrap_or_else(|f| panic!("{}\n{}", f.message, f.report));
    assert!(result.handed_over, "{name}: handed over");
    let game = result.game;
    let request = game.pending().cloned().expect("a declaration");
    let knowledge = Knowledge::of(&game, request.player);
    let (obs, index) = observe_indexed(&game, &knowledge, request.player, InfoMode::Full).expect("an observation");
    let answering = Answering::new(&game, &request, &obs, &index);
    let legal = legal(&game, &request, &answering);
    assert_eq!(walked(&game, &request, &answering), legal, "{name}: the walks reach exactly the legal declarations");
    let mut sizes: Vec<usize> = legal.iter().map(Vec::len).collect();
    sizes.sort();
    sizes
}

const CAVERNS: &str = "\
[Bot1 Deck]
Caverns of Despair
Black Lotus
Mox Ruby
Ornithopter x3
Mountain x20
[Bot2 Deck]
Ornithopter x3
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Ornithopter.Cast
Bot1.Hand.Ornithopter.Cast
Bot1.Hand.Ornithopter.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Caverns of Despair.Cast
[T2 Bot2.Main1]
Bot2.Hand.Ornithopter.Cast
Bot2.Hand.Ornithopter.Cast
Bot2.Hand.Ornithopter.Cast
";

#[test]
fn caverns_of_despair_walks_reach_every_two_attackers_and_never_three() {
    let script = format!("{CAVERNS}[T3 Bot1.Main1]\n[Handover Bot1.Attack]\n");
    // No attacker, each one alone, each two together: never all three.
    assert_eq!(check("caverns-attack", &script), [0, 1, 1, 1, 2, 2, 2]);
}

#[test]
fn caverns_of_despair_walks_reach_every_two_blockers_and_never_three() {
    let script = format!("{CAVERNS}[T3 Bot1.Attackers]\nBot1.Attack(Ornithopter#1 > Bot2; Ornithopter#2 > Bot2)\n[Handover Bot2.Block]\n");
    assert_eq!(check("caverns-block", &script), [0, 1, 1, 1, 2, 2, 2]);
}

#[test]
fn errantry_walks_attack_with_the_enchanted_creature_only_alone() {
    let script = "\
[Bot1 Deck]
Mox Ruby
Mox Pearl
Grizzly Bears
Ornithopter
Errantry
Forest
Forest x10
[Bot2 Deck]
Plains x20
[Actions]
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Ornithopter.Cast
Bot1.Hand.Forest.Play
Bot1.Battlefield.Forest.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Grizzly Bears.Cast
[T3 Bot1.Main1]
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Errantry.Cast(Bot1.Battlefield.Grizzly Bears)
[Handover Bot1.Attack]
";
    // Nobody, the Bears alone, the Ornithopter alone: never both.
    assert_eq!(check("errantry-attack", script), [0, 1, 1]);
}

const CONSCRIPTS: &str = "\
[Bot1 Deck]
Orcish Conscripts
Mox Ruby
Memnite x2
Mountain x20
[Bot2 Deck]
Memnite x3
Island x20
[Actions]
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Orcish Conscripts.Cast
Bot1.Hand.Memnite.Cast
Bot1.Hand.Memnite.Cast
";

#[test]
fn orcish_conscripts_walks_attack_with_it_only_beside_two_others() {
    let script = format!("{CONSCRIPTS}[T3 Bot1.Main1]\n[Handover Bot1.Attack]\n");
    // Any of the Memnites without it, or all three.
    assert_eq!(check("conscripts-attack", &script), [0, 1, 1, 2, 3]);
}

#[test]
fn orcish_conscripts_walks_block_with_it_only_beside_two_others() {
    let script = format!(
        "{CONSCRIPTS}[T2 Bot2.Main1]\nBot2.Hand.Memnite.Cast\nBot2.Hand.Memnite.Cast\nBot2.Hand.Memnite.Cast\n[T4 Bot2.Attackers]\nBot2.Attack(Memnite#1 > Bot1; Memnite#2 > Bot1; Memnite#3 > Bot1)\n[Handover Bot1.Block]\n"
    );
    assert_eq!(check("conscripts-block", &script), [0, 1, 1, 2, 3]);
}
