use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::{self, MUTAGEN};
use cantrip_oracle as oracle;

/// "Target artifact, enchantment, or creature with flying" (CR 115.1, 702.9): flying as the layers give it now.
fn artifact_enchantment_or_flying_creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && {
        let c = q.chars(id);
        c.is(CardType::Artifact) || c.is(CardType::Enchantment) || targets::has_flying_creature(q, id)
    })
}

/// "Destroy up to one target ... Create a Mutagen token." (CR 701.8a): two sentences; the token comes whether or not anything
/// was targeted, or destroyed.
fn destroy_then_mutagen(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(target) = targets::object(g, item, 0) {
        g.destroy(target);
    }
    g.end_sentence();
    tokens::create_mutagen(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(artifact_enchantment_or_flying_creature).optional()], destroy_then_mutagen),
    ..oracle::MUTANT_CHAIN_REACTION
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
