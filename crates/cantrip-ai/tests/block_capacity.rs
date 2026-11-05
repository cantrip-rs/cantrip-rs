//! A declaration allowing one blocker against two attackers remains representable to a policy.
use cantrip_ai::{Answering, Knowledge, observe_indexed};
use cantrip_engine::*;
use cantrip_obs::factor::{FactoredAnswer, Machine, Tok};
use cantrip_obs::{AskKind, InfoMode};
use cantrip_script::GameScript;

#[test]
fn observation_and_factorization_keep_a_blockers_extra_allowance() {
    let script = GameScript::parse("[Bot1 Deck]\nOrnithopter x2\nIsland x20\n[Bot2 Deck]\nOrnithopter\nIsland x20\n[Actions]\nBot1.Hand.Ornithopter.Cast\nBot1.Hand.Ornithopter.Cast\n[T2 Bot2.Main1]\nBot2.Hand.Ornithopter.Cast\n[T3 Bot1.Attackers]\nBot1.Attack(Ornithopter#1 > Bot2; Ornithopter#2 > Bot2)\n[Handover Bot2.Block]\n", "block-capacity").unwrap();
    let game = cantrip_script::run(&script).unwrap_or_else(|f| panic!("{f:?}")).game;
    let mut request = game.pending().cloned().expect("blocker question");
    let RequestKind::DeclareBlockers { eligible, attackers, requirements, .. } = &mut request.kind else { panic!("not blockers") };
    assert_eq!((eligible.len(), attackers.len()), (1, 2));
    requirements.capacities = vec![(eligible[0], 2)];
    let player = request.player;
    let (obs, index) = observe_indexed(&game, &Knowledge::of(&game, player), player, InfoMode::Full).unwrap();
    let answering = Answering::new(&game, &request, &obs, &index);
    let AskKind::DeclareBlockers { eligible, attackers, max_attackers_per_blocker, .. } = &answering.ask.kind else { panic!("not blockers ask") };
    assert_eq!(max_attackers_per_blocker, &[2]);
    let mut machine = Machine::new(&answering.ask, &[]);
    for attacker in attackers {
        let step = machine.step().unwrap();
        let index = step.legal().find(|&i| step.tokens[i].kind == Tok::Block && step.tokens[i].a == eligible[0] && step.tokens[i].b == *attacker).expect("block offered");
        machine.choose(index);
    }
    let factored = machine.answer().unwrap();
    assert!(matches!(factored, FactoredAnswer::Block(blocks) if blocks.len() == 2));
    assert!(matches!(answering.engine(&game, &request, factored), Ok(Answer::Block(blocks)) if blocks.len() == 2));
}
