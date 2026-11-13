use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn optional_damage_question_explains_the_consequence() {
    let prefix = include_str!("../../../scenarios/gaze-of-pain-declined.cantrip")
        .split("Bot1.Decline").next().unwrap();
    let text = format!("{prefix}[Handover Bot1.Accept]\n");
    let script = GameScript::parse(&text, "gaze-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let RequestKind::May { source, purpose: Purpose::Ask(question) } = &game.pending().unwrap().kind else { panic!("expected optional damage question") };
    assert_eq!(game.cards.get(game.state.obj(*source).card).name, "Grizzly Bears");
    assert_eq!(question.verb, "GazeOfPain");
    assert_eq!(question.ask, "have the attacking creature deal damage equal to its power to the target?");
    assert_eq!(question.detail, "If you do, it assigns no combat damage this turn.");
    assert_eq!(question.yes, "Deal damage");
    assert_eq!(question.no, "Don't deal damage");
    assert!(!question.yes_is_default);
}

#[test]
fn opponents_unblocked_creature_does_not_trigger_gaze() {
    let prefix = include_str!("../../../scenarios/gaze-of-pain-opponents-creature.cantrip")
        .split("[T4 Bot2.Main2]").next().unwrap();
    let text = format!("{prefix}[Handover Bot1.Pass]\n");
    let script = GameScript::parse(&text, "gaze-opponent").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    assert!(game.state.stack.is_empty(), "an opponent's creature must not trigger Gaze of Pain");
}
