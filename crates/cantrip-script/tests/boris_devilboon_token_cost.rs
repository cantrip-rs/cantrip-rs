//! A Minor Demon has no mana cost, rather than a mana cost of zero (CR 111.3, 202.1a).
use cantrip_script::{GameScript, run};

#[test]
fn minor_demon_has_no_mana_cost() {
    let text = include_str!("../../../scenarios/boris-devilboon-creates-minor-demon.cantrip");
    let script = GameScript::parse(text, "boris-token-cost").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let token = game.state.battlefield().iter().copied()
        .find(|&id| game.cards.get(game.state.obj(id).card).name == "Minor Demon")
        .expect("Boris created a Minor Demon");
    assert!(!game.cards.get(game.state.obj(token).card).has_cost, "the token has no mana cost");
}
