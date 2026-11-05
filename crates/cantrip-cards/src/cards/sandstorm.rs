use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Sandstorm deals 1 damage to each attacking creature." (CR 608.2, 120.3).
fn sandstorm_deals_1_damage_attacking(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let recipients_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.is_attacking(id)
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .map(Target::Object)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, i64)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, 1i64))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], sandstorm_deals_1_damage_attacking),
    ..oracle::SANDSTORM
};
