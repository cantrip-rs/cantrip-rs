use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Earthquake deals X damage to each creature without flying and each player." (CR 608.2, 120.3).
fn earthquake_deals_x_damage_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_0 = i64::from(item.x) as u16;
    let recipients_0 = {
        let q: &dyn Query = g;
        {
            let mut all: Vec<Target> = Vec::new();
            for t in q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).is(CardType::Creature)
                        && !(q.chars(id).keywords.has(Keyword::Flying))
                })
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .map(Target::Object)
                .collect::<Vec<Target>>()
            {
                if !all.contains(&t) {
                    all.push(t);
                }
            }
            for t in q
                .state()
                .apnap()
                .collect::<Vec<PlayerId>>()
                .into_iter()
                .map(Target::Player)
                .collect::<Vec<Target>>()
            {
                if !all.contains(&t) {
                    all.push(t);
                }
            }
            all
        }
    };
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, amount_0))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], earthquake_deals_x_damage_creature),
    ..oracle::EARTHQUAKE
};
