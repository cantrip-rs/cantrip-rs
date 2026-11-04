use cantrip_engine::*;
use cantrip_script::{GameScript, run};

fn at_choice(replacement: &str) -> Game {
    let source = include_str!("../../../scenarios/thelons-chant-counter-and-upkeep-cost.cantrip")
        .split("Bot2.Accept").next().unwrap().to_owned() + replacement;
    let script = GameScript::parse(&source, "thelons-chant-prompt").unwrap();
    run(&script).map_err(|f| f.report).unwrap().game
}

#[test]
fn counter_choice_names_the_action_and_uses_counter_metadata() {
    let game = at_choice("[Handover Bot2.Accept]\n");
    let RequestKind::May { purpose: Purpose::Ask(question), .. } = &game.pending().unwrap().kind else { panic!("expected optional counter question") };
    assert_eq!(question.ask, "put a -1/-1 counter on a creature you control?");
    assert_eq!(question.no, "Take damage");
    let game = at_choice("Bot2.Accept\n[Handover Bot2.Choose]\n");
    let RequestKind::ChooseCards { purpose: Purpose::PutCounters(CounterKind::MinusOne, 1), .. } = &game.pending().unwrap().kind else { panic!("expected -1/-1 counter selection") };
}
