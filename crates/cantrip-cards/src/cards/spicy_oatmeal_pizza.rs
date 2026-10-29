use cantrip_api::*;
use cantrip_mechanics::effects::gain_three_life;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this artifact enters, it deals 4 damage to any target and 3 damage to you": one damage event with two recipients
/// (CR 120.3), the Pizza the source of both. With its one target gone the ability does not resolve (CR 608.2b), and so does
/// not hurt its controller either.
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = g.state().target(item, 0);
    g.damage_together(&[(item.object, target, 4), (item.object, Target::Player(item.controller), 3)]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, enters, "When this artifact enters, it deals 4 damage to any target and 3 damage to you.").targets(&[targets::ANY]),
        ability(cost("{2}, {T}").and(SACRIFICE_THIS), gain_three_life, "{2}, {T}, Sacrifice this artifact: You gain 3 life."),
    ],
    ..oracle::SPICY_OATMEAL_PIZZA
};
