use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target artifact or tapped creature" (CR 115.1, 608.2b): an artifact, or a creature that is tapped, when chosen and again as
/// this resolves. An untapped artifact creature is an artifact, so it is a legal target; an untapped creature that is no
/// artifact is not.
fn artifact_or_tapped_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::artifact_or_creature_filter(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Artifact) || q.obj(id).tapped)
}

/// "Destroy target artifact or tapped creature. You gain 3 life." (CR 701.8, 119.3, 608.2b): the life is gained whether or not the
/// target was destroyed (indestructible, regenerated); with its only target illegal the spell does nothing at all.
pub static CARD: CardDef = CardDef {
    spell: spell(&[target(artifact_or_tapped_creature)], effects::destroy_first_target_and_gain_life::<3>),
    ..oracle::RADIANT_STRIKE
};
