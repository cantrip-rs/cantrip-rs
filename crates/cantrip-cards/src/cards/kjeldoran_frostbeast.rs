use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At end of combat, destroy all creatures blocking or blocked by this creature." (CR 608.2, 701.8a).
fn destroy_all_creatures_blocking_blocked(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && (q
                        .state()
                        .blocks
                        .iter()
                        .any(|b| b.blocker == id && b.attacker == item.object)
                        || q
                            .state()
                            .blocks
                            .iter()
                            .any(|b| b.attacker == id && b.blocker == item.object))
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::END_OF_COMBAT,
            destroy_all_creatures_blocking_blocked,
            "At end of combat, destroy all creatures blocking or blocked by this creature.",
        ),
    ],
    ..oracle::KJELDORAN_FROSTBEAST
};
