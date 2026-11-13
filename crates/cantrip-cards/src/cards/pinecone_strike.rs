use cantrip_api::*;
use cantrip_mechanics::replacements::EXILE_INSTEAD_OF_DYING;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// Destroy target artifact token (CR 111.1, 701.8): an artifact permanent that is a token.
fn destroy_artifact_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

fn artifact_token(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::ARTIFACT.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).token)
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 608.2c).
static MODAL: ModalDef = modes(&[
    mode(effects::damage_target_exile_if_it_would_die::<3>, "Pinecone Strike deals 3 damage to target creature. If that creature would die this turn, exile it instead.")
        .targets(&[targets::CREATURE]),
    mode(destroy_artifact_token, "Destroy target artifact token.")
        .targets(&[target(artifact_token)]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature would die this turn, exile it instead.")],
    ..oracle::PINECONE_STRIKE
};
