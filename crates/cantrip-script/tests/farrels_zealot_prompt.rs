use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn self_target_damage_is_not_the_default_choice() {
    let source = include_str!("../../../scenarios/farrels-zealot-self-target.cantrip")
        .split("Bot1.Decline").next().unwrap().to_owned() + "[Handover Bot1.Accept]\n";
    let script = GameScript::parse(&source, "farrels-zealot-prompt").unwrap();
    let game = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report)).game;
    let RequestKind::May { purpose: Purpose::Ask(question), .. } = &game.pending().unwrap().kind else {
        panic!("expected the damage choice");
    };
    assert_eq!(question.ask, "have this creature deal 3 damage to the target creature?");
    assert_eq!(question.detail, "If you do, it assigns no combat damage this turn");
    assert_eq!(question.yes, "Deal damage");
    assert_eq!(question.no, "Don't");
    assert!(!question.yes_is_default);
}
