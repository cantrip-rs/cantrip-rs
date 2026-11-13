use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target attacking creature has base power 0 until end of turn" (CR 613.4b, 611.2a).
fn set_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, target, EffectKind::SetPower(0), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, set_power, "{T}: Target attacking creature has base power 0 until end of turn.")
        .targets(&[targets::ATTACKING_CREATURE])],
    ..oracle::SINGING_TREE
};
