use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn the_prevention_effect_is_shown_on_its_creature_with_both_directions() {
    let original = include_str!("../../../scenarios/elvish-scout-does-not-prevent-noncombat-damage.cantrip");
    let actions = original.split("[T3 Bot1.Main2]").next().unwrap();
    let text = format!("{actions}\n[Handover Bot1.Pass]\n");
    let script = GameScript::parse(&text, "scout-prevention-metadata").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let creature = game.state.battlefield().iter().copied().find(|id| game.cards.get(game.state.obj(*id).card).name == "Kraken Hatchling").unwrap();
    let effect = game.state.effects.iter().find(|e| e.affected == creature && matches!(e.kind, EffectKind::Prevention { .. })).expect("prevention on the attacking creature");
    assert_eq!(effect.duration, Duration::EndOfTurn);
    let EffectKind::Prevention { card, index, to, shield } = effect.kind else { unreachable!() };
    assert_eq!(to, Target::None);
    assert_eq!(shield, Shield::All);
    let display = &game.cards.get(card).granted_preventions[index as usize];
    assert_eq!(display.damage, "combat damage");
    assert_eq!(display.of_it, Some("to and dealt by it"));
}
