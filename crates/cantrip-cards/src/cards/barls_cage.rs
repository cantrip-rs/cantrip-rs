use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Target creature doesn't untap during its controller's next untap step (CR 608.2b, 611.2a).
fn keep_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::DoesntUntap, Duration::DuringItsControllersNextUntapStep);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}"),
        keep_tapped,
        "{3}: Target creature doesn't untap during its controller's next untap step.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::BARLS_CAGE
};
