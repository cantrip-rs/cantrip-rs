use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn psychic_allergy_names_its_sacrifice_choice() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus
Mox Pearl
Mox Sapphire
Psychic Allergy
Island x22
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Island.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
[T3 Bot1.Main1]
Bot1.Hand.Island.Play
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Psychic Allergy.Cast
Bot1.Colors(Green)
[Handover Bot1.Decline]
"#, "psychic-allergy-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let request = game.pending().expect("payment question");
    let RequestKind::UnlessPays { question, options, .. } = &request.kind else { panic!("expected payment question") };
    assert_eq!(question.ask, "sacrifice two Islands, or destroy {subject}?");
    assert_eq!(question.yes, "Sacrifice two Islands");
    assert!(options.iter().any(|o| o.kind == ActionKind::PayCost));
    assert!(options.iter().any(|o| o.kind == ActionKind::DeclineCost));
}

#[test]
fn psychic_allergy_payment_question_tracks_changed_land_word() {
    let template = r#"
[Bot1 Deck]
Black Lotus
Mox Pearl
Mox Sapphire
Mox Sapphire
Psychic Allergy
Magical Hack
Mountain
Mountain
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Mountain.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Sapphire.Cast
[T3 Bot1.Main1]
Bot1.Hand.Mountain.Play
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire#1.Activate
Bot1.Hand.Psychic Allergy.Cast
Bot1.Colors(Green)
Bot1.Battlefield.Mox Sapphire#2.Activate
Bot1.Hand.Magical Hack.Cast(Psychic Allergy)
Bot1.Pass
Bot2.Pass
Bot1.Modes(Island)
Bot1.Modes(Mountain)
[Handover Bot1.Decline]
"#;
    for land in ["Mountain", "Plains", "Swamp", "Forest"] {
    let input = template.replace("Mountain", land);
    let script = GameScript::parse(&input, "psychic-allergy-changed-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let request = game.pending().expect("payment question");
    let RequestKind::UnlessPays { question, options, .. } = &request.kind else { panic!("expected payment question") };
    assert_eq!(question.ask, format!("sacrifice two {}, or destroy {{subject}}?", if land == "Plains" { "Plains".to_owned() } else { format!("{land}s") }));
    assert_eq!(question.yes, format!("Sacrifice two {}", if land == "Plains" { "Plains".to_owned() } else { format!("{land}s") }));
    assert!(options.iter().any(|o| o.kind == ActionKind::PayCost));
    assert!(options.iter().any(|o| o.kind == ActionKind::DeclineCost));
    }
}
