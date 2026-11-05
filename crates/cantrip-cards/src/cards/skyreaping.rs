use cantrip_api::*;
use cantrip_mechanics::{devotion, targets};
use cantrip_oracle as oracle;

/// "Skyreaping deals damage to each creature with flying equal to your devotion to green." (CR 120.3, 702.9): the devotion is
/// counted once, as the spell resolves, and all the damage is dealt at once.
fn damage_each_flier(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = devotion::devotion(g, item.controller, Color::Green) as u16;
    let batch: Vec<(ObjectId, Target, i64)> =
        g.state().battlefield().iter().copied().filter(|&id| targets::has_flying_creature(g, id)).map(|id| (item.object, Target::Object(id), i64::from(amount))).collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], damage_each_flier),
    ..oracle::SKYREAPING
};
