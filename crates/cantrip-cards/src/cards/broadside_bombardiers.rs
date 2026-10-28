use cantrip_api::*;
use cantrip_mechanics::{boast, damage_target};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Broadside Bombardiers deals damage equal to 2 plus the sacrificed permanent's mana value to any target": the
/// mana value it had as it was sacrificed (CR 608.2h), of the permanent this activation's cost sacrificed
/// (`Query::cost_objects`, CR 602.2b).
fn boast_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let value = g.cost_objects(item).first().map_or(0, |&id| g.mana_value(id));
    damage_target(g, item, 0, 2 + value as u16);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[boast::ability(
        "Boast \u{2014} Sacrifice another creature or artifact: Broadside Bombardiers deals damage equal to 2 plus the sacrificed permanent's mana value to any target. (Activate only if this creature attacked this turn and only once each turn.)",
        Cost::sacrifice(1, targets::filters::another_creature_or_artifact),
        &[targets::ANY],
        boast_damage,
    )],
    ..oracle::BROADSIDE_BOMBARDIERS
};
