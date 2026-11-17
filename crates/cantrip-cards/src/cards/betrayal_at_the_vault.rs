use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control deals damage equal to its power to each of two other target creatures" (CR 120.3, 609.7, 608.2b):
/// the creature is the first target and the victims the second and third, each judged as the spell resolves. With the creature no
/// longer a legal target nothing is dealt damage (no source); with one victim gone the other still takes the damage. Its power is
/// read once, as the spell resolves, and the damage to both is dealt together (CR 120.2): one event, by the creature, so its
/// lifelink and deathtouch count for each victim.
fn deal_damage_to_two_others(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let power = fight::damage_equal_to_power(g, creature);
    if power == 0 {
        return Ok(());
    }
    let hits: Vec<(ObjectId, Target, u32)> =
        (1..item.target_count as usize).filter_map(|slot| targets::object(g, item, slot)).map(|victim| (creature, Target::Object(victim), power)).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_OTHER_THAN_THOSE_CHOSEN.between(2, 2)], deal_damage_to_two_others),
    ..oracle::BETRAYAL_AT_THE_VAULT
};
