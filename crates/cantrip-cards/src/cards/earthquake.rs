use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Earthquake deals X damage to each creature without flying and each player." One simultaneous damage event includes both
/// players and every creature without flying on the battlefield (CR 608.2c, 120.2).
fn earthquake_deals_x_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = item.x;
    let mut hits: Vec<(ObjectId, Target, i64)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && !g.chars(id).keywords.has(Keyword::Flying))
        .map(|id| (item.object, Target::Object(id), i64::from(x)))
        .collect();
    hits.extend(g.state().apnap().map(|p| (item.object, Target::Player(p), i64::from(x))));
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], earthquake_deals_x_damage),
    ..oracle::EARTHQUAKE
};
