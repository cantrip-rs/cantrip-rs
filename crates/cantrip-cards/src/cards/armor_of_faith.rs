use cantrip_api::*;
use cantrip_mechanics::{attach, pump, statics, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature gets +0/+1 until end of turn" (CR 608.2h, 611.2a): use the current attachment,
/// or its last known attachment if the Aura has left the battlefield.
fn enchanted_creature_gets_0_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        pump(g, item.object, creature, 0, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{W}"), enchanted_creature_gets_0_1, "{W}: Enchanted creature gets +0/+1 until end of turn.")],
    statics: &[static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt_saturating::<1, 1>, "Enchanted creature gets +1/+1.")],
    enchant: Some(targets::CREATURE),
    ..oracle::ARMOR_OF_FAITH
};
