use cantrip_api::*;
use cantrip_mechanics::{attach, regenerate, statics, targets};
use cantrip_oracle as oracle;

/// "Sacrifice this Aura: Regenerate enchanted creature." (CR 608.2h, 701.19a).
fn regenerate_enchanted_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        regenerate::permanent(g, item.object, creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(SACRIFICE_THIS, regenerate_enchanted_creature, "Sacrifice this Aura: Regenerate enchanted creature.")],
    statics: &[static_ability(
        Layer::PtModify,
        attach::attached_object,
        statics::plus_pt_saturating::<1, 1>,
        "Enchanted creature gets +1/+1.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::THRULL_RETAINER
};
