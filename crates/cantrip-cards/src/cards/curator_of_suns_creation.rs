use cantrip_api::*;
use cantrip_mechanics::discover;
use cantrip_oracle as oracle;

/// "Discover again for the same value" (CR 701.57a): the discover value is the one the triggering discover had, carried by the
/// trigger as `data[0]` (`discover::you_discover`), whatever made it. The discover this ability does triggers it again, but it
/// triggers only once each turn.
fn discover_again(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discover::discover(g, item.controller, item.data[0])?;
    Ok(())
}

/// "Whenever you discover, discover again for the same value. This ability triggers only once each turn." (CR 701.57a, 603.2)
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        discover::YOU_DISCOVER,
        discover_again,
        "Whenever you discover, discover again for the same value. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::CURATOR_OF_SUNS_CREATION
};
