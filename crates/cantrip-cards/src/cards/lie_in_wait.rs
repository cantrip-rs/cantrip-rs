use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to your hand. ~ deals damage equal to that card's power to target
/// creature." (CR 608.2b, 120.3, 208.1) The power is the card's as it sits in the graveyard, read before it moves, per its
/// ruling. A card that is no longer a legal target is not returned and gives no information, so no damage is dealt (CR 608.2b);
/// a creature target that is gone still lets the card return. A power of 0 or less is no damage (CR 120.8).
fn return_and_deal_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = targets::object(g, item, 0);
    let victim = targets::object(g, item, 1);
    let power = card.map(|c| g.chars(c).power.unwrap_or(0).max(0) as u32);
    if let Some(card) = card {
        g.return_to_hand(&[card]);
    }
    if let (Some(victim), Some(power)) = (victim, power) {
        g.damage(item.object, Target::Object(victim), power);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD, targets::CREATURE], return_and_deal_its_power),
    ..oracle::LIE_IN_WAIT
};
