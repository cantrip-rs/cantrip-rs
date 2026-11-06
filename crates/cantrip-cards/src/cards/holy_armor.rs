use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

/// "{W}: Enchanted creature gets +0/+1 until end of turn." (CR 608.2, 611.2a).
fn enchanted_creature_gets_0_1(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        Some(item.object)
            .and_then(|o: ObjectId| q.state().obj(o).attached_to)
            .into_iter()
            .collect::<Vec<ObjectId>>()
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(0, 1),
                    Duration::EndOfTurn,
                );
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{W}"),
            enchanted_creature_gets_0_1,
            "{W}: Enchanted creature gets +0/+1 until end of turn.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            attach::attached_object,
            statics::plus_pt_saturating::<0, 2>,
            "Enchanted creature gets +0/+2.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::HOLY_ARMOR
};
