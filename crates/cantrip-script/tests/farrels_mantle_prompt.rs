use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn damage_choice_is_not_the_default() {
    let source = include_str!("../../../scenarios/farrels-mantle-unblocked-damage.cantrip")
        .split("Bot1.Accept").next().unwrap().to_owned() + "[Handover Bot1.Accept]\n";
    let script = GameScript::parse(&source, "farrels-mantle-prompt").unwrap();
    let game = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report)).game;
    let RequestKind::May { purpose: Purpose::Ask(question), .. } = &game.pending().unwrap().kind else {
        panic!("expected the damage choice");
    };
    assert_eq!(question.ask, "have the attacking creature deal damage to the target?");
    assert_eq!(question.detail, "If you do, it assigns no combat damage this turn.");
    assert_eq!(question.yes, "Deal damage");
    assert_eq!(question.no, "Decline");
    assert!(!question.yes_is_default);
}
