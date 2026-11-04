use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}: Tap target land. It doesn't untap during its controller's untap step for as long as this creature remains tapped." (CR 608.2, 611.2a, 701.26a).
fn tap_target_land_doesnt_untap(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(land);
    g.end_sentence();
    if is_live(g, land) {
        g.add_effect(item.object, land, EffectKind::DoesntUntap, Duration::WhileSourceTapped);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                tap_target_land_doesnt_untap,
                "{T}: Tap target land. It doesn't untap during its controller's untap step for as long as this creature remains tapped.",
            )
            .targets(&[targets::LAND]),
    ],
    may_not_untap: true,
    ..oracle::MOLE_WORMS
};
