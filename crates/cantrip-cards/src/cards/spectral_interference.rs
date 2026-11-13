use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target artifact or creature spell" (CR 115.1a): a spell on the stack with the card type artifact, creature, or both (an
/// artifact creature is one spell, not two).
fn artifact_or_creature_spell(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::SPELL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(artifact_or_creature_spell)], effects::counter_first_target_unless_pays::<4>),
    ..oracle::SPECTRAL_INTERFERENCE
};
