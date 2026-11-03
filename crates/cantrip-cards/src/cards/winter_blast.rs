use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Tap X target creatures. Winter Blast deals 2 damage to each of those creatures with flying." (CR 608.2, 120.3, 701.26a).
fn tap_x_target_creatures_winter(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::legal_objects(g, item);
    g.tap_together(&objects_0);
    g.end_sentence();
    let recipients_1 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                objects_0.contains(&id)
                    && (q.chars(id).is(CardType::Creature)
                        && q.chars(id).keywords.has(Keyword::Flying))
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .map(Target::Object)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, 2u32))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.counted()], tap_x_target_creatures_winter),
    ..oracle::WINTER_BLAST
};
