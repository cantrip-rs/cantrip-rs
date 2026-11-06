//! Probe Wiitigo's history boundary for two upkeep steps in one turn (CR 500.10).
//! The harness cannot insert an extra upkeep, so substitute a prior upkeep record after real combat.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn previous_upkeep_in_same_turn_excludes_earlier_combat() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus
Mox Pearl
Mox Sapphire
Mox Jet
Wiitigo
Forest x20
[Bot2 Deck]
Ornithopter
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Wiitigo.Cast
[T2 Bot2.Main1]
Bot2.Hand.Ornithopter.Cast
[T3 Bot1.Main1]
Bot1.Attack(Wiitigo > Bot2)
Bot2.Block(Ornithopter > Wiitigo)
[T5 Bot1.Upkeep]
[Handover Bot1.Pass]
"#, "wiitigo-extra-upkeep-history").unwrap();
    let mut game = run(&script).map_err(|f| f.report).expect("upkeep handover").game;
    assert!(!game.state.stack.is_empty(), "upkeep trigger must still be on the stack: {:?}", game.state.turn);
    let pos = game.log.iter().rposition(|r| r.turn == 5 && matches!(r.event, Event::StepBegan { step: Step::Upkeep })).expect("current upkeep");
    let prior = game.log[pos];
    let wiitigo = game.state.battlefield().iter().copied().find(|id| game.cards.get(game.state.obj(*id).card).name == "Wiitigo").unwrap();
    // Model Trap Runner making the attacker blocked just before the extra upkeep.
    game.log[pos - 2] = Record { serial: (pos - 2) as u32, step: Step::EndOfCombat, event: Event::BecameBlocked { attacker: wiitigo }, ..prior };
    game.log[pos - 1] = Record { serial: (pos - 1) as u32, ..prior };
    for _ in 0..20 {
        let request = game.pending().cloned().expect("priority");
        match request.kind {
            RequestKind::Priority { .. } => game.answer(request.id, request.player, Answer::Pass).unwrap(),
            other => panic!("unexpected request: {other:?}"),
        }
        game.run();
        if game.state.stack.is_empty() {
            assert_eq!(game.state.obj(wiitigo).counters(CounterKind::PlusOne), 4, "combat before the earlier upkeep must not count");
            return;
        }
    }
    assert_eq!(game.state.obj(wiitigo).counters(CounterKind::PlusOne), 4, "combat before the earlier upkeep must not count");
}
