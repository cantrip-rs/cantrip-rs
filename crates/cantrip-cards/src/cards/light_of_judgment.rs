use cantrip_api::*;
use cantrip_mechanics::{attach, choose, select, targets};
use cantrip_oracle as oracle;

/// "Light of Judgment deals 6 damage to target creature. Destroy up to one Equipment attached to that creature." (CR 608.2c, 701.8,
/// 301.5): the damage first, then, as it resolves and not as it is cast (the ruling, CR 608.2d), you choose up to one of the Equipment
/// attached to the creature, whoever controls it, and destroy it. The creature has not yet died of the damage (state-based actions
/// wait, CR 704.3), so its Equipment is still attached. With the creature an illegal target the spell does nothing (CR 608.2b).
fn six_damage_then_destroy_an_equipment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.damage(item.object, Target::Object(creature), 6);
    let equipment = attach::equipment_attached_to(g, creature);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Destroy, equipment, select::cards(select::up_to(1)))?;
    g.destroy_all(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], six_damage_then_destroy_an_equipment), ..oracle::LIGHT_OF_JUDGMENT };
