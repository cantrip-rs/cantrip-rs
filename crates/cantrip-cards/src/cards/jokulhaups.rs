use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Destroy all artifacts, creatures, and lands. They can't be regenerated." (CR 608.2, 701.8a).
fn destroy_all_artifacts_creatures_lands(
    g: &mut dyn Game,
    _: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature)
                    || q.chars(id).is(CardType::Land)
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all_no_regen(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], destroy_all_artifacts_creatures_lands),
    ..oracle::JOKULHAUPS
};
