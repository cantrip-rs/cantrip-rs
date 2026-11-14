use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Seismic Rupture deals 2 damage to each creature without flying": one damage event for every creature on the battlefield
/// without flying as the spell resolves, all dealt together (CR 608.2c, 120.2).
fn deal_two_to_each_creature_without_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, u32)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && !g.chars(id).keywords.has(Keyword::Flying))
        .map(|id| (item.object, Target::Object(id), 2))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], deal_two_to_each_creature_without_flying),
    ..oracle::SEISMIC_RUPTURE
};
