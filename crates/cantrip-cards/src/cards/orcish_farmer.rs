use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Target land becomes a Swamp until its controller's next untap step (CR 305.7, 611.2a).
fn swamp(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    let land_type = g.words(item.object).subtype(Subtype::Swamp);
    g.add_effect(item.object, land, EffectKind::SetLandType(land_type), Duration::UntilItsControllersNextUntapStep);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, swamp, "{T}: Target land becomes a Swamp until its controller's next untap step.").targets(&[targets::LAND])],
    ..oracle::ORCISH_FARMER
};
