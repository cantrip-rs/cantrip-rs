//! The real entry questions and permanent projection name both ordered subtype choices.
use std::collections::HashSet;
use cantrip_engine::*;
use cantrip_host::{handles::HandleSpace, prompt::prompt, protocol::PromptPayload, view::player_view};
use cantrip_script::{Advance, GameScript, Scripted};

#[test]
fn ordered_land_type_prompts_and_public_view_are_typed() {
    let original = include_str!("../../../scenarios/illusionary-terrain-changes-basic-lands-and-upkeep.cantrip");
    let prefix = original.split("[T2 Bot2.Main1]").next().unwrap();
    let text = format!("{prefix}\n[Expect]\nBot1.Battlefield.Snow-Covered Forest.Subtypes = Island\n");
    let mut script = Scripted::deal(GameScript::parse(&text, "illusionary-ordered-choices").unwrap()).map_err(|f| f.report).unwrap();
    let mut captions = Vec::new();
    loop {
        match script.advance(&[PlayerId(0)]).map_err(|f| f.report).unwrap() {
            Advance::Ask(expected) => {
                if matches!(expected.request.kind, RequestKind::ChooseSubtype { .. }) {
                    let PromptPayload::ChooseModesPromptMessage(p) = prompt(script.game(), &expected.request, &mut HandleSpace::new()).unwrap().0 else { panic!("subtype prompt") };
                    captions.push((p.heading, p.modes));
                }
                script.answered(expected.intent).map_err(|f| f.report).unwrap();
            }
            Advance::Check(_) => {},
            Advance::Done => break,
        }
    }
    script.verdict().map_err(|f| f.report).unwrap();
    assert_eq!(captions.len(), 2);
    assert_eq!(captions[0].0, "Choose the first basic land type");
    assert_eq!(captions[1].0, "Choose the second basic land type");
    assert_eq!(captions[0].1.len(), 5);
    assert_eq!(captions[1].1.len(), 4);
    assert!(!captions[1].1.iter().any(|x| x == "Forest"));
    for viewer in [PlayerId(0), PlayerId(1)] {
        let game = script.game();
        let view = player_view(game, viewer, &mut HandleSpace::new(), &["Alice".into(), "Bob".into()], &[false, false], &HashSet::new(), &HashSet::new(), &HashSet::new());
        let terrain = view.objects.iter().find(|o| o.name == "Illusionary Terrain" && o.zone == Some(cantrip_host::protocol::ZoneKindView::Battlefield)).unwrap();
        let serialized = serde_json::to_value(&terrain.choices).unwrap();
        assert_eq!(serialized, serde_json::json!([
            {"Key": "first land type", "Value": "Forest", "Object": 0},
            {"Key": "second land type", "Value": "Island", "Object": 0}
        ]));
    }
}

#[test]
fn existing_single_land_type_is_shown_as_a_land_type() {
    let source = include_str!("../../../scenarios/phantasmal-terrain-land.cantrip");
    let script = GameScript::parse(source, "phantasmal-single-land-type").unwrap();
    let game = cantrip_script::run(&script).map_err(|f| f.report).unwrap().game;
    let view = player_view(&game, PlayerId(0), &mut HandleSpace::new(), &["Alice".into(), "Bob".into()], &[false, false], &HashSet::new(), &HashSet::new(), &HashSet::new());
    let aura = view.objects.iter().find(|o| o.name == "Phantasmal Terrain" && o.zone == Some(cantrip_host::protocol::ZoneKindView::Battlefield)).unwrap();
    let choices: Vec<_> = aura.choices.iter().map(|c| (c.key.as_str(), c.value.as_str())).collect();
    assert_eq!(choices, [("land type", "Mountain")]);
}
