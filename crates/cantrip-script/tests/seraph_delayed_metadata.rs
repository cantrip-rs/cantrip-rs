//! Seraph's two delayed abilities describe the effect each will perform.
use cantrip_engine::*;
use cantrip_script::{run, GameScript};

#[test]
fn seraph_delayed_abilities_describe_their_own_effects() {
    let source = include_str!("../../../scenarios/seraph-leaves-after-return.cantrip");
    let before_return = source.split("[T3 Bot1.End]").next().unwrap();
    let script = GameScript::parse(&format!("{before_return}[T3 Bot1.Main2]\n"), "seraph-before-return").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let delayed = result.game.state.delayed.iter().find(|d| matches!(d.when,
        DelayedWhen::Step { step: Step::End, .. })).expect("return waits for end step");
    assert_eq!(result.game.cards.ability(delayed.ability).text(),
        "Put that card onto the battlefield under your control at the beginning of the next end step.");

    let after_return = source.split("[T5 Bot1.Main1]").next().unwrap();
    let script = GameScript::parse(after_return, "seraph-after-return").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let delayed = result.game.state.delayed.iter().find(|d| matches!(d.when,
        DelayedWhen::WhenEvent)).expect("sacrifice waits for loss of control");
    assert_eq!(result.game.cards.ability(delayed.ability).text(),
        "Sacrifice the creature when you lose control of this creature.");
}
