//! The protocol presents one replacement's mana alternatives to the player tapping the land.
use cantrip_engine::*;
use cantrip_host::{handles::HandleSpace, prompt::prompt, protocol::PromptPayload};
use cantrip_script::{Advance, GameScript, Scripted};

fn prompts(source: &str) -> Vec<(PlayerId, String, Vec<String>)> {
    let mut game = Scripted::deal(GameScript::parse(source, "grouped-mana-host").unwrap()).map_err(|f| f.report).unwrap();
    let mut found = Vec::new();
    loop {
        match game.advance(&[PlayerId(0)]).map_err(|f| f.report).unwrap() {
            Advance::Ask(expected) => {
                if matches!(&expected.request.kind, RequestKind::ChooseModes { modes, .. } if modes.iter().any(|m| matches!(m, Label::ManaType(_)))) {
                    let PromptPayload::ChooseModesPromptMessage(p) = prompt(game.game(), &expected.request, &mut HandleSpace::new()).unwrap().0 else { panic!("mana modes") };
                    found.push((expected.request.player, p.heading, p.modes));
                }
                game.answered(expected.intent).map_err(|f| f.report).unwrap();
            }
            Advance::Check(_) => {},
            Advance::Done => break,
        }
    }
    game.verdict().map_err(|f| f.report).unwrap();
    found
}

#[test]
fn dual_land_asks_once_and_single_type_does_not_ask() {
    for source in [include_str!("../../../scenarios/naked-singularity-tundra-r.cantrip"), include_str!("../../../scenarios/naked-singularity-tundra-g.cantrip")] {
        let found = prompts(source);
        assert_eq!(found, [(PlayerId(0), "Choose which type of mana to add".into(), vec!["{R}".into(), "{G}".into()])]);
    }
    assert!(prompts(include_str!("../../../scenarios/naked-singularity-island-mana.cantrip")).is_empty());
}
