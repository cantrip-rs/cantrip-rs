//! Heroism's payment avoids only its own prevention, not other effects (2004-10-04 ruling).
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn payment_names_the_creature_and_only_this_abilitys_prevention() {
    let source = include_str!("../../../scenarios/heroism-pays-for-each-attacking-red-creature.cantrip");
    let actions = source.split("Bot2.Battlefield.Mox Ruby.Activate\nBot2.Battlefield.Mox Pearl#1.Activate").next().unwrap();
    let text = format!("{actions}\n[Handover Bot2.Accept]\n");
    let game = run(&GameScript::parse(&text, "heroism-payment").unwrap()).unwrap().game;
    let request = game.pending().unwrap();
    assert_eq!(request.player, PlayerId(1));
    let RequestKind::UnlessPays { source, subject, question, .. } = &request.kind else { panic!("expected Heroism payment") };
    assert_eq!(game.name(*source), "Heroism");
    assert_eq!(game.name(*subject), "Warship Scout");
    assert_eq!(question.ask, "pay {cost} so this ability won't prevent {subject}'s combat damage?");
}
