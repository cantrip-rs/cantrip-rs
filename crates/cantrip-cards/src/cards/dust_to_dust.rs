use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Exile the legal targets together (CR 608.2b, 701.13a).
fn exile_artifacts(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.exile_all(&targets::legal_objects(g, item));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT.between(2, 2)], exile_artifacts),
    ..oracle::DUST_TO_DUST
};
