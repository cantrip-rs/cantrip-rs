//! Choice captions describe the restrictions, including the ability's text changes.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

fn choice(text: &str) -> (Purpose, Count, String, Vec<String>) {
    let script = GameScript::parse(text, "raiding-party-choice").unwrap();
    let result = run(&script).map_err(|f| f.report).unwrap();
    let request = result.game.pending().expect("choice");
    let RequestKind::ChooseCards { purpose, rule, .. } = &request.kind else { panic!("expected card choice") };
    (*purpose, rule.count, rule.kind.to_string(), rule.words(&|_| String::new()))
}

#[test]
fn creature_and_plains_choices_describe_their_restrictions() {
    let scenario = include_str!("../../../scenarios/raiding-party-multicolor-creature-saves-plains.cantrip");
    let prefix = scenario.split("! Bot1.Choose(Bot2.Battlefield.Savannah Lions)").next().unwrap();
    let (purpose, count, kind, words) = choice(&format!("{prefix}[Handover Bot1.Choose]\n"));
    assert!(matches!(purpose, Purpose::Tap));
    assert!(matches!(count, Count::AnyNumber));
    assert_eq!(kind, "untapped white creature");
    assert_eq!(words, ["you control"]);
    let prefix = scenario.split("Bot1.Choose(Bot1.Battlefield.Plains#1;").next().unwrap();
    let (purpose, count, kind, _) = choice(&format!("{prefix}[Handover Bot1.Choose]\n"));
    assert!(matches!(purpose, Purpose::Choose));
    assert!(matches!(count, Count::UpTo(4)));
    assert_eq!(kind, "Plains land");
}

#[test]
fn captions_follow_words_changed_before_activation() {
    let scenario = include_str!("../../../scenarios/raiding-party-colors-locked-when-activated.cantrip");
    let prefix = scenario.split("[T5 Bot1.Main1]").next().unwrap();
    let pre = format!("{prefix}[T5 Bot1.Main1]\nBot1.Battlefield.Island.Activate\nBot1.Hand.Sleight of Mind.Cast(Bot1.Battlefield.Raiding Party)\nBot1.Pass\nBot2.Pass\nBot1.Colors(White)\nBot1.Colors(Blue)\nBot1.Pass\nBot2.Pass\nBot1.Battlefield.Raiding Party.Activate\nBot1.Choose(Orcish Lumberjack)\nBot1.Pass\nBot2.Pass\n");
    for (color, expected) in [("Blue", "untapped blue creature"), ("Black", "untapped black creature"), ("Red", "untapped red creature"), ("Green", "untapped green creature")] {
        let changed = pre.replace("Bot1.Colors(Blue)", &format!("Bot1.Colors({color})"));
        let (_, _, kind, words) = choice(&format!("{changed}[Handover Bot1.Choose]\n"));
        assert_eq!(kind, expected);
        assert_eq!(words, ["you control"]);
    }

    for (land, expected) in [("Island", "Island land"), ("Swamp", "Swamp land"), ("Mountain", "Mountain land"), ("Forest", "Forest land")] {
        let hack = pre.replace("Sleight of Mind", "Magical Hack").replace("Bot1.Colors(White)\nBot1.Colors(Blue)", &format!("Bot1.Modes(Plains)\nBot1.Modes({land})"));
        let (_, count, kind, _) = choice(&format!("{hack}Bot1.Choose(Savannah Lions)\n[Handover Bot1.Choose]\n"));
        assert!(matches!(count, Count::UpTo(2)));
        assert_eq!(kind, expected);
    }
}
