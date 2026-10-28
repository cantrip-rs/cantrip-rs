use cantrip_api::*;
use cantrip_mechanics::replacements::{EXILE_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_mechanics::{damage_target, destroy_target, targets};
use cantrip_oracle as oracle;

/// Pinecone Strike deals 3 damage to target creature. If that creature would die this turn, exile it instead. The
/// replacement is the spell's own effect on that creature (CR 611.2c, 614.1a), made whether or not the damage is dealt, and
/// before it.
fn damage_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        exile_if_it_would_die(g, item.object, 0, creature);
        damage_target(g, item, 0, 3);
    }
    Ok(())
}

/// Destroy target artifact token (CR 111.1, 701.8): an artifact permanent that is a token.
fn destroy_artifact_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

fn artifact_token(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::ARTIFACT.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).token)
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(damage_creature, "Pinecone Strike deals 3 damage to target creature. If that creature would die this turn, exile it instead.")
        .targets(&[targets::CREATURE]),
    mode(destroy_artifact_token, "Destroy target artifact token.")
        .targets(&[target(artifact_token)]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING],
    ..oracle::PINECONE_STRIKE
};
