use cantrip_api::*;
use cantrip_mechanics::devotion;
use cantrip_oracle as oracle;

/// "Skyreaping deals damage to each creature with flying equal to your devotion to green. (Each {G} in the mana costs of permanents you control counts toward your devotion to green.)" (CR 608.2, 120.3).
fn skyreaping_deals_damage_creature_flying(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_0 = {
        let q: &dyn Query = g;
        i64::from(devotion::devotion(q, item.controller, Color::Green)) as u16
    };
    let recipients_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && q.chars(id).keywords.has(Keyword::Flying)
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .map(Target::Object)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, amount_0))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], skyreaping_deals_damage_creature_flying),
    ..oracle::SKYREAPING
};
