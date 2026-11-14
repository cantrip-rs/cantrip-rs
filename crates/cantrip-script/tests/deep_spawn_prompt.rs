use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn upkeep_choice_describes_milling_and_sacrifice() {
    let source = include_str!("../../../scenarios/deep-spawn-mills-two-to-remain.cantrip")
        .split("Bot1.Accept").next().unwrap().to_owned() + "[Handover Bot1.Accept]\n";
    let script = GameScript::parse(&source, "deep-spawn-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let RequestKind::May { purpose: Purpose::Ask(question), .. } = &game.pending().unwrap().kind else {
        panic!("expected the mill choice");
    };
    assert_eq!(question.verb, "Mill");
    assert_eq!(question.ask, "mill two cards?");
    assert_eq!(question.detail, "If you don't, sacrifice this creature");
    assert_eq!(question.yes, "Mill two");
    assert_eq!(question.no, "Don't mill");
}
