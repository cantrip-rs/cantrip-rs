use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_IF_CAST;
use cantrip_oracle as oracle;

/// "When this creature enters, if you cast it, destroy all nonartifact creatures." The intervening-if condition is
/// checked both when it enters and as the ability resolves (CR 603.4). All qualifying creatures are destroyed together
/// (CR 101.4, 701.8a); artifact creatures are excluded.
fn destroy_nonartifact_creatures(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let doomed: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Creature) && !chars.is(CardType::Artifact)
        })
        .collect();
    g.destroy_all(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_IF_CAST,
        destroy_nonartifact_creatures,
        "When this creature enters, if you cast it, destroy all nonartifact creatures.",
    )],
    ..oracle::CRYSTALLINE_ENTITY
};
