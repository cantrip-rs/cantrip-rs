use cantrip_api::*;
use cantrip_mechanics::kicker::was_kicked;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target artifact with mana value X or less": the X it was kicked with, read from the creature (CR 702.33a).
fn artifact_with_value_at_most_x(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let x = q.obj(asker.source).x as u32;
    matches!(t, Target::Object(id) if is_live(q, id) && q.is_on_battlefield(id) && q.chars(id).is(CardType::Artifact) && q.mana_value(id) <= x)
}

/// Gain control of the artifact (CR 108.4, no duration); an Equipment is then attached to this creature.
fn take_and_attach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(artifact) = targets::object(g, item, 0) else { return Ok(()) };
    g.gain_control(item.object, artifact, item.controller, Duration::Indefinite);
    if g.chars(artifact).has_subtype(Subtype::Equipment) && is_live(g, item.object) && g.is_on_battlefield(item.object) {
        g.attach(artifact, item.object);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "Kicker {X}. X can't be 0." (CR 107.3a): announcing 0 is not kicking it, so a kicked one has X 1 or more.
    kicker: kicker(mana("{X}")),
    x_cant_be_zero: true,
    abilities: &[triggered(THIS_ENTERS, take_and_attach, "When this creature enters, if it was kicked, gain control of target artifact with mana value X or less. If that artifact is an Equipment, attach it to this creature.")
        .targets(&[target(artifact_with_value_at_most_x)])
        .condition(was_kicked)],
    ..oracle::THIEVING_SKYDIVER
};
