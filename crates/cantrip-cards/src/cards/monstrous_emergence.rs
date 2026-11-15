use cantrip_api::*;
use cantrip_mechanics::{behold, damage_target, targets};
use cantrip_oracle as oracle;

/// "Monstrous Emergence deals damage equal to the power of the creature you chose or the card you revealed to target creature"
/// (CR 120.3, 608.2h): the power of what the cost beheld, as it last existed if the creature has left the battlefield by now.
fn damage_equal_to_the_beheld_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g
        .history()
        .spell_cost_objects(item.object, CostOrigin::PrintedAdditional, 0, CostPart::Behold)
        .next()
        .map_or(0, |beheld| g.chars(beheld).power.unwrap_or(0).max(0) as u32);
    damage_target(g, item, 0, power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], damage_equal_to_the_beheld_power),
    // "As an additional cost to cast this spell, choose a creature you control or reveal a creature card from your hand" (CR
    // 701.4a, 601.2b): behold a creature, which is not optional.
    additional_cost: Some(behold::cost(behold::creature, "creature")),
    ..oracle::MONSTROUS_EMERGENCE
};
