//! The grant applies to red legends, not nonlegendary red creatures.
use cantrip_script::{GameScript, run};

#[test]
fn only_legendary_red_creatures_receive_the_named_ability() {
    let script = GameScript::parse(include_str!("../../../scenarios/mountain-stronghold-grants-bands-to-red-legends.cantrip"), "mountain stronghold grant").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let has_grant = |name: &str| {
        let id = game.state.battlefield().iter().copied()
            .find(|&id| game.chars(id).name == name).unwrap();
        game.chars(id).abilities().any(|a| game.cards.ability(a).text() == "Bands with other legendary creatures")
    };
    assert!(has_grant("Ragavan, Nimble Pilferer"));
    assert!(!has_grant("Monastery Swiftspear"));
    assert!(!has_grant("Isamaru, Hound of Konda"));
}
