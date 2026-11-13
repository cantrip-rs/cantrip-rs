use cantrip_engine::*;
use cantrip_script::{run, GameScript};

#[test]
fn upkeep_payment_names_the_artifact_and_consequence() {
    let source = include_str!("../../../scenarios/curse-artifact-opponent-sacrifices.cantrip");
    let actions = source.split("Bot2.PayCost").next().unwrap();
    let text = format!("{actions}\n[Handover Bot2.Accept]\n");
    let game = run(&GameScript::parse(&text, "curse-artifact-payment").unwrap()).unwrap().game;
    let request = game.pending().unwrap();
    assert_eq!(request.player, PlayerId(1));
    let RequestKind::UnlessPays { source, subject, question, .. } = &request.kind else { panic!("expected sacrifice choice") };
    assert_eq!(game.name(*source), "Ornithopter");
    assert_eq!(game.name(*subject), "Ornithopter");
    assert_eq!(question.ask, "Curse Artifact: sacrifice {source}?");
    assert_eq!(question.detail, "If you don't, the Aura deals 2 damage to you.");
    assert_eq!(question.yes, "Sacrifice {subject}");
    assert_eq!(question.no, "Don't sacrifice");
}
