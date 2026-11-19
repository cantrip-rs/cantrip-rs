use cantrip_api::*;
use cantrip_mechanics::{explore, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, it explores X times." X is the one it was cast with (CR 107.3m); it explores once for each, one after the
/// other, each a whole explore (CR 701.44a) that may add a counter or put a revealed card into the graveyard before the next.
fn explores_x_times(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for _ in 0..g.obj(item.object).x {
        explore(g, item.object)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, explores_x_times, "When this creature enters, it explores X times.")],
    ..oracle::JADELIGHT_SPELUNKER
};
