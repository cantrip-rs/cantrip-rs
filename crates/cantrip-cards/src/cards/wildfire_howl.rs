use cantrip_api::*;
use cantrip_mechanics::{gift, targets};
use cantrip_oracle as oracle;

/// "Wildfire Howl deals 2 damage to each creature. If the gift was promised, instead Wildfire Howl deals 1 damage to any target and 2
/// damage to each creature." (CR 702.174, 120.3, 608.2b, 608.2c): the gift (the opponent draws a card) was given before this runs
/// (CR 702.174j). The any-target exists only when the gift was promised (CR 601.2c, `TargetSpec::if_promised`); it is dealt its 1
/// together with the 2 to every creature on the battlefield as the spell resolves (one instruction joined by "and"), so a creature
/// that is the target takes 3 in all. If that only target is no longer legal the spell does not resolve at all (CR 608.2b), so no creature is
/// damaged (ruling 2024-07-26).
fn howl(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hits: Vec<(ObjectId, Target, u32)> =
        g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).map(|id| (item.object, Target::Object(id), 2)).collect();
    if item.gift.is_some() {
        hits.push((item.object, g.state().target(item, 0), 1));
    }
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY.between(0, 0).if_promised(1, 1)], howl),
    gift: Some(gift::CARD),
    ..oracle::WILDFIRE_HOWL
};
