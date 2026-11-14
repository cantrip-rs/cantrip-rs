use cantrip_api::*;
use cantrip_mechanics::{damage_target, mana, targets};
use cantrip_oracle as oracle;

/// The sacrificed creatures' total power is read as they last existed (CR 608.2h); negative
/// powers contribute to the sum before it is clamped to zero (CR 107.1b). The source itself
/// counts if it was a creature when sacrificed. Keep the first arrivals before dealing damage,
/// so a card that moves again during damage is not found as a new object (CR 400.7j).
fn damage_then_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut sacrificed: Vec<ObjectId> = g.history().ability_cost_objects_of_part(item.serial, CostPart::Variable).collect();
    let source: Vec<ObjectId> = g.history().ability_cost_objects_of_part(item.serial, CostPart::SelfPermanent).collect();
    let mut power: i32 = sacrificed.iter().map(|&id| g.chars(id).power.unwrap_or(0)).sum();
    for &id in &source {
        if g.chars(id).is(CardType::Creature) {
            power += g.chars(id).power.unwrap_or(0);
        }
    }
    sacrificed.extend(source);
    let cards: Vec<ObjectId> = sacrificed.into_iter().flat_map(|id| g.found_all_after_move(id)).filter(|&id| !g.obj(id).token && !g.obj(id).zone.kind().is_hidden()).collect();
    damage_target(g, item, 0, power.max(0) as u32);
    g.end_sentence();
    g.exile_all(&cards);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[ability(
        TAP.and(SACRIFICE_THIS).and(Cost { variable: Some(VariableCost::SacrificeX(targets::filters::creature)), ..Cost::NONE }),
        damage_then_exile,
        "{T}, Sacrifice this artifact and any number of creatures you control: This artifact deals X damage to any target, where X is the total power of the creatures sacrificed this way, then exile this artifact and those creature cards.",
    ).targets(&[targets::ANY])],
    ..oracle::SWORD_OF_THE_AGES
};
