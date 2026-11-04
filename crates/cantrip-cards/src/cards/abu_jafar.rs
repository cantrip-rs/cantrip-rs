use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::triggers::THIS_DIES;

/// "When this creature dies, destroy all creatures blocking or blocked by it. They can't be regenerated." (CR 608.2, 701.8a).
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
                    && (q.creatures_blocking(item.object).contains(&id)
                        || q.creatures_blocked_by(item.object).contains(&id))
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all_no_regen(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_DIES,
            destroy_all_creatures_blocking_blocked,
            "When this creature dies, destroy all creatures blocking or blocked by it. They can't be regenerated.",
        ),
    ],
    ..oracle::ABU_JAFAR
};
