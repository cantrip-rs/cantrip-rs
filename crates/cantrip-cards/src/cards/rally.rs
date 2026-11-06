use cantrip_api::*;
use cantrip_mechanics::pump;
use cantrip_oracle as oracle;

/// "Blocking creatures get +1/+1 until end of turn." (CR 608.2, 611.2a).
fn blocking_creatures_get_1_1(
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
                q.is_blocking(id)
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                pump(g, item.object, object, 1, 1);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], blocking_creatures_get_1_1),
    ..oracle::RALLY
};
