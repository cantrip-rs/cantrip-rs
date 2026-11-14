use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}: This creature deals 1 damage to any target and 1 damage to any target of an opponent's choice." (CR 115.1).
fn target_chooser_0(q: &dyn Query, asker: &Asker) -> Vec<PlayerId> {
    q.state().opponents(asker.controller).collect::<Vec<PlayerId>>()
}

/// "{T}: This creature deals 1 damage to any target and 1 damage to any target of an opponent's choice." (CR 608.2, 120.3).
fn creature_deals_1_damage_any(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = announced(g, item, 0, true);
    let t_1 = announced(g, item, 1, true);
    let mut batch_0: Vec<(ObjectId, Target, u32)> = Vec::new();
    for t in t_0.clone() {
        batch_0.push((item.object, t, 1u32));
    }
    for t in t_1.clone() {
        batch_0.push((item.object, t, 1u32));
    }
    g.damage_together(&batch_0);
    Ok(())
}

/// "{T}: This creature deals 1 damage to any target and 1 damage to any target of an opponent's choice." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                creature_deals_1_damage_any,
                "{T}: This creature deals 1 damage to any target and 1 damage to any target of an opponent's choice.",
            )
            .targets(&[targets::ANY, targets::ANY.chosen_by_one_of(target_chooser_0)]),
    ],
    ..oracle::CUOMBAJJ_WITCHES
};
