//! Blood of the Martyr's question, asked once for each creature Pyroclasm would damage, names the damage it is about, and
//! redirecting it - which costs life - is not the default answer.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn each_question_names_its_creature_and_redirecting_is_not_the_default() {
    let source = include_str!("../../../scenarios/blood-of-the-martyr-says-which-creature-each-question-is-about-in-the-browser.cantrip")
        .split("Bot1.Accept\n").next().unwrap().to_owned()
        + "[Handover Bot1.Accept]\n";
    let game = run(&GameScript::parse(&source, "blood of the martyr prompt").unwrap()).map_err(|f| f.report).unwrap().game;
    let RequestKind::May { purpose: Purpose::AskDamage(question, about), .. } = &game.pending().unwrap().kind else { panic!("expected the redirection question") };
    assert_eq!(question.ask, "have that damage dealt to you instead?");
    assert!(!question.yes_is_default, "redirecting costs life");
    assert_eq!((game.name(about.source), about.amount), ("Pyroclasm", 2));
    let Target::Object(creature) = about.to else { panic!("the damage is to a creature: {:?}", about.to) };
    assert_eq!(game.name(creature), "Ornithopter");
    assert_eq!(game.state.obj(creature).controller, PlayerId(1), "the opponent's Ornithopter is asked about first");
}
