use cantrip_engine::*;
use cantrip_script::{run, GameScript};

#[test]
fn payment_question_identifies_the_spell_and_caster() {
    let source = include_str!("../../../scenarios/nether-void-counters-unless-caster-pays-three.cantrip");
    let prefix = source.split("! Bot1.PayCost").next().unwrap();
    let script = GameScript::parse(&format!("{prefix}[Handover Bot1.Decline]\n"), "nether-void-prompt").unwrap();
    let game = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report)).game;
    let request = game.pending().unwrap();
    assert_eq!(request.player, PlayerId(0));
    let RequestKind::UnlessPays { source, subject, total, question, options, .. } = &request.kind else {
        panic!("expected payment question");
    };
    assert_eq!(game.card(game.obj(*source).card).name, "Nether Void");
    assert_eq!(game.card(game.obj(*subject).card).name, "Memnite");
    assert_eq!(*total, mana("{3}"));
    assert_eq!(question.ask, "pay {cost}, or {subject} is countered?");
    assert!(options.iter().any(|o| o.kind == ActionKind::DeclineCost));
}
