use cantrip_api::*;
use cantrip_mechanics::{attach, regenerate, statics, targets};
use cantrip_oracle as oracle;

/// "{R}{R}{R}: Regenerate enchanted creature." (CR 608.2h, 701.19a).
fn regenerate_enchanted_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = g.obj(item.object).attached_to else { return Ok(()) };
    instruction(g, |g| regenerate::permanent(g, item.object, creature));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{R}{R}{R}"),
            regenerate_enchanted_creature,
            "{R}{R}{R}: Regenerate enchanted creature.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            attach::attached_object,
            statics::plus_pt_saturating::<1, 0>,
            "Enchanted creature gets +1/+0.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::THE_BRUTE
};
