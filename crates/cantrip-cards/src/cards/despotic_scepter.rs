use cantrip_api::*;
use cantrip_mechanics::destroy_target_no_regen;
use cantrip_oracle as oracle;

/// "{T}: Destroy target permanent you own. It can't be regenerated." (CR 115.1).
fn destroy_target_permanent_own_cant_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (asker
        .controller == q.state().obj(id).owner)
    )
}

/// "{T}: Destroy target permanent you own. It can't be regenerated." (CR 608.2, 701.8a).
fn destroy_target_permanent_own_cant(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    destroy_target_no_regen(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                destroy_target_permanent_own_cant,
                "{T}: Destroy target permanent you own. It can't be regenerated.",
            )
            .targets(&[target(destroy_target_permanent_own_cant_target_0)]),
    ],
    ..oracle::DESPOTIC_SCEPTER
};
