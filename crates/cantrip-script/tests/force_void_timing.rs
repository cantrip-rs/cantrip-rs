//! Force Void selects the next turn's upkeep, not the next upkeep (CR 603.7).
use cantrip_engine::*;
use cantrip_script::{run, GameScript};

#[test]
fn force_void_schedules_the_next_turns_upkeep() {
    let source = include_str!("../../../scenarios/force-void-counters-and-draws.cantrip");
    let before_next_turn = source.split("[T3 Bot1.Main1]").next().unwrap();
    let script = GameScript::parse(before_next_turn, "force-void-timing").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    assert_eq!(result.game.state.delayed.len(), 1);
    assert_eq!(result.game.cards.ability(result.game.state.delayed[0].ability).text(), "Draw a card.");
    assert!(matches!(result.game.state.delayed[0].when,
        DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
}

#[test]
fn force_void_payment_question_describes_the_counter() {
    let source = include_str!("../../../scenarios/force-void-own-creature-paid.cantrip");
    let prefix = source.split("Bot1.Battlefield.Mox Jet.Activate").next().unwrap();
    let script = GameScript::parse(&format!("{prefix}[Handover Bot1.Decline]\n"), "force-void-payment").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    match &result.game.pending().unwrap().kind {
        RequestKind::UnlessPays { question, options, .. } => {
            assert_eq!(question.ask, "pay {cost}, or {subject} is countered?");
            assert_eq!(question.yes, "Pay {cost}");
            assert_eq!(question.no, "Let it be countered");
            assert!(options.iter().any(|o| o.kind == ActionKind::DeclineCost));
        }
        _ => panic!("expected an unless-payment prompt"),
    }
}
