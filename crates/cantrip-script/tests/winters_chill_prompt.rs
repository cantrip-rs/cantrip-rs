use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn payment_names_the_attacker_and_options() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Mox Emerald
Balduvian Bears
Forest x20
[Bot2 Deck]
Snow-Covered Island
Mox Sapphire
Winter's Chill
Island x20
[Actions]
Bot1.Hand.Mox Emerald.Cast
Bot1.Hand.Forest.Play
Bot1.Battlefield.Forest.Activate
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Hand.Balduvian Bears.Cast
[T2 Bot2.Main1]
Bot2.Hand.Snow-Covered Island.Play
Bot2.Hand.Mox Sapphire.Cast
[T3 Bot1.Main1]
Bot1.Attack(Balduvian Bears > Bot2)
Bot2.Battlefield.Snow-Covered Island.Activate
Bot2.Battlefield.Mox Sapphire.Activate
Bot2.Hand.Winter's Chill.Cast(X: 1)
Bot2.Target(Bot1.Battlefield.Balduvian Bears)
Bot2.Pass
[Handover Bot1.Decline]
"#, "winters-chill-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let request = game.pending().expect("payment question");
    let RequestKind::UnlessPays { question, subject, options, .. } = &request.kind else { panic!("expected payment question") };
    assert_eq!(question.ask, "pay {cost} to prevent combat damage to and by {subject}?");
    assert_eq!(game.cards.get(game.state.obj(*subject).card).name, "Balduvian Bears");
    assert!(options.iter().any(|o| o.kind == ActionKind::DeclineCost));
}

#[test]
fn second_payment_names_the_attacker_and_options() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Mox Emerald
Balduvian Bears
Forest x20
[Bot2 Deck]
Snow-Covered Island
Mox Sapphire
Winter's Chill
Island x20
[Actions]
Bot1.Hand.Mox Emerald.Cast
Bot1.Hand.Forest.Play
Bot1.Battlefield.Forest.Activate
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Hand.Balduvian Bears.Cast
[T2 Bot2.Main1]
Bot2.Hand.Snow-Covered Island.Play
Bot2.Hand.Mox Sapphire.Cast
[T3 Bot1.Main1]
Bot1.Attack(Balduvian Bears > Bot2)
Bot2.Battlefield.Snow-Covered Island.Activate
Bot2.Battlefield.Mox Sapphire.Activate
Bot2.Hand.Winter's Chill.Cast(X: 1)
Bot2.Target(Bot1.Battlefield.Balduvian Bears)
Bot2.Pass
Bot1.Decline
[Handover Bot1.Decline]
"#, "winters-chill-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let request = game.pending().expect("payment question");
    let RequestKind::UnlessPays { question, subject, options, .. } = &request.kind else { panic!("expected payment question") };
    assert_eq!(question.ask, "pay {cost} for {subject}?");
    assert_eq!(game.cards.get(game.state.obj(*subject).card).name, "Balduvian Bears");
    assert!(options.iter().any(|o| o.kind == ActionKind::DeclineCost));
}
