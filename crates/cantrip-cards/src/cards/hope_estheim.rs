use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "Each opponent mills X cards, where X is the amount of life you gained this turn" (CR 701.17, 101.4): X is read once, from
/// the event history, as the ability resolves, and every opponent mills together.
fn opponents_mill_life_gained(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.history().life_gained(item.controller);
    let mills: Vec<(PlayerId, u64)> = g.state().opponents(item.controller).map(|p| (p, x)).collect();
    g.mill_together_wide(&mills);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        opponents_mill_life_gained,
        "At the beginning of your end step, each opponent mills X cards, where X is the amount of life you gained this turn.",
    )],
    ..oracle::HOPE_ESTHEIM
};
