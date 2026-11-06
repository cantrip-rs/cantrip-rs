use cantrip_api::*;
use cantrip_mechanics::{regenerate, targets};
use cantrip_oracle as oracle;

/// "{G}{G}{G}{G}, {T}: Regenerate target creature." (CR 701.19a).
fn regenerate_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    regenerate::target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{G}{G}{G}{G}").and(TAP),
        regenerate_creature,
        "{G}{G}{G}{G}, {T}: Regenerate target creature.",
    ).targets(&[targets::CREATURE])],
    ..oracle::NIALL_SILVAIN
};
