use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn payment_explains_both_consequences() {
    let source = include_str!("../../../scenarios/power-sink-declined-taps-lands.cantrip")
        .split("Bot2.Decline").next().unwrap().to_owned() + "[Handover Bot2.Accept]\n";
    let game = run(&GameScript::parse(&source, "power sink prompt").unwrap()).map_err(|f| f.report).unwrap().game;
    let RequestKind::UnlessPays { question, total, subject, source, .. } = &game.pending().unwrap().kind else { panic!("expected payment choice") };
    assert_eq!(total.generic, 2);
    assert_ne!(subject, source);
    assert!(game.state.stack.iter().any(|item| item.object == *subject && item.kind == StackKind::Spell));
    assert_eq!(question.ask, "pay {cost}?");
    assert_eq!(question.detail, "If you decline, {subject} is countered if able; tap all lands with mana abilities you control and lose all unspent mana");
}

#[test]
fn uncounterable_spell_has_truthful_payment_question() {
    let source = include_str!("../../../scenarios/power-sink-uncounterable.cantrip")
        .split("Bot1.Decline").next().unwrap().to_owned() + "[Handover Bot1.Accept]\n";
    let game = run(&GameScript::parse(&source, "uncounterable prompt").unwrap()).map_err(|f| f.report).unwrap().game;
    let RequestKind::UnlessPays { question, total, subject, source, .. } = &game.pending().unwrap().kind else { panic!("expected payment choice") };
    assert_eq!(total.generic, 2);
    let sink = game.state.stack.iter().find(|item| item.object == *source).unwrap();
    assert_eq!(game.state.target(sink, 0), Target::Object(*subject));
    assert_eq!(question.ask, "pay {cost}?");
    assert_eq!(question.detail, "If you decline, {subject} is countered if able; tap all lands with mana abilities you control and lose all unspent mana");
}
