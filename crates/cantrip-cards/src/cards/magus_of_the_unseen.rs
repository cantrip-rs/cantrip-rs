use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// The delayed ability taps the artifact when its controller loses it (CR 603.7, 701.26a).
fn tap_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(artifact) = item.subject.object() {
        g.tap(artifact);
    }
    Ok(())
}

/// Untap the artifact, gain control, grant haste and watch it (CR 608.2c, 611.2a).
fn take_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(artifact) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        g.untap(artifact);
        g.gain_control(item.object, artifact, item.controller, Duration::EndOfTurn);
    });
    g.end_sentence();
    cantrip_mechanics::haste_until_end_of_turn(g, item.object, artifact);
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::WhenEvent,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::Object(artifact),
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{U}").and(TAP),
        take_artifact,
        "{1}{U}, {T}: Untap target artifact an opponent controls and gain control of it until end of turn. It gains haste until end of turn. When you lose control of the artifact, tap it.",
    ).targets(&[targets::ARTIFACT_AN_OPPONENT_CONTROLS])],
    delayed: &[triggered(
        triggers::YOU_LOSE_CONTROL_OF_THIS,
        tap_artifact,
        "When you lose control of the artifact, tap it.",
    )],
    ..oracle::MAGUS_OF_THE_UNSEEN
};
