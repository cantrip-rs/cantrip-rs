use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{X}, {T}: This creature deals half X damage, rounded down, to any target, and half X damage, rounded up, to you." (CR 608.2, 120.3).
fn creature_deals_half_x_damage(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let mut batch_0: Vec<(ObjectId, Target, u32)> = Vec::new();
    let amount_0_0 = item.x / 2;
    for t in t_0.clone() {
        batch_0.push((item.object, t, amount_0_0));
    }
    let amount_0_1 = item.x / 2 + item.x % 2;
    for t in Some(item.controller)
        .into_iter()
        .map(Target::Player)
        .collect::<Vec<Target>>()
    {
        batch_0.push((item.object, t, amount_0_1));
    }
    g.damage_together(&batch_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{X}").and(TAP),
                creature_deals_half_x_damage,
                "{X}, {T}: This creature deals half X damage, rounded down, to any target, and half X damage, rounded up, to you.",
            )
            .targets(&[targets::ANY]),
    ],
    ..oracle::BANSHEE
};
