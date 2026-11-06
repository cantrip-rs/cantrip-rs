//! The resolution choice describes the two actions, including a changed color word.
use cantrip_engine::{label_text, Label, RequestKind};
use cantrip_script::{run, GameScript};

fn choice(color: Option<&str>) -> Vec<String> {
    let response = color.map_or_else(String::new, |color| format!("Bot1.Battlefield.Mox Sapphire.Activate\nBot1.Hand.Sleight of Mind.Cast(Stack.Essence Filter)\nBot1.Pass\nBot2.Pass\nBot1.Colors(White)\nBot1.Colors({color})\nBot1.Pass\nBot2.Pass\n"));
    let source = format!("[Bot1 Deck]\nEssence Filter\nBlack Lotus\nMox Sapphire\nSleight of Mind\nIsland x20\n[Bot2 Deck]\nIsland x20\n[Actions]\nBot1.Hand.Mox Sapphire.Cast\nBot1.Pass\nBot1.Hand.Black Lotus.Cast\nBot1.Pass\nBot1.Battlefield.Black Lotus.Activate(Color: Green)\nBot1.Hand.Essence Filter.Cast\n{response}[Handover Bot1.Modes]\n[Expect]\n");
    let script = GameScript::parse(&source, "choice").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let request = result.game.pending().expect("choice pending");
    let RequestKind::ChooseModes { modes, eligible, count, at_most, .. } = &request.kind else { panic!("expected choice") };
    assert_eq!((*count, *at_most), (1, 0));
    assert_eq!(eligible, &[0, 1]);
    assert!(modes.iter().all(|mode| matches!(mode, Label::Card(_, _) | Label::WordedCard(_, _, _))), "action choices must use card action labels");
    modes.iter().map(|mode| label_text(mode).to_string()).collect()
}

#[test]
fn choice_describes_actions_and_tracks_changed_color() {
    assert_eq!(choice(None), ["Destroy all enchantments", "Destroy all nonwhite enchantments"]);
    for (color, label) in [("Blue", "nonblue"), ("Black", "nonblack"), ("Red", "nonred"), ("Green", "nongreen")] {
        assert_eq!(choice(Some(color)), ["Destroy all enchantments".to_string(), format!("Destroy all {label} enchantments")]);
    }
}
