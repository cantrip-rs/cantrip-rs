use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{1}, {T}: Return this creature and target green or blue creature you control to their owner's hand." (CR 115.1).
fn return_creature_target_green_blue_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t)
        && (targets::creature_of_either_color::<{ Color::Green as u8 }, { Color::Blue as u8 }>().filter)(q, asker, t)
}

/// "{1}, {T}: Return this creature and target green or blue creature you control to their owner's hand." (CR 608.2, 400.7).
fn return_creature_target_green_blue(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                item.object == id
                    || t_0.get(0).copied().and_then(|t: Target| t.object()) == Some(id)
            })
            .collect::<Vec<ObjectId>>()
    };
    g.return_to_hand(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}").and(TAP),
                return_creature_target_green_blue,
                "{1}, {T}: Return this creature and target green or blue creature you control to their owner's hand.",
            )
            .targets(&[target(return_creature_target_green_blue_target_0)]),
    ],
    ..oracle::SNOW_HOUND
};
