use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, you may destroy target artifact or enchantment." (CR 115.1).
fn destroy_target_artifact_enchantment_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Enchantment)))
    )
}

/// "When this creature enters, you may destroy target artifact or enchantment." (CR 608.2, 701.8a).
fn destroy_target_artifact_enchantment(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_3: Vec<ObjectId> = Vec::new();
    let mut hoisted_2: Vec<ObjectId> = Vec::new();
    let t_0 = vec![g.state().target(item, 0)];
    if effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        let objects_1 = t_0
            .get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .into_iter()
            .collect::<Vec<_>>();
        let destroyed_1 = g.destroy_all(&objects_1);
        g.end_sentence();
        hoisted_2.extend(destroyed_1.clone());
        hoisted_3
            .extend({
                let q: &dyn Query = g;
                objects_1
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
            });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::THIS_ENTERS,
                destroy_target_artifact_enchantment,
                "When this creature enters, you may destroy target artifact or enchantment.",
            )
            .targets(&[target(destroy_target_artifact_enchantment_target_0)]),
    ],
    ..oracle::RECLAMATION_SAGE
};
