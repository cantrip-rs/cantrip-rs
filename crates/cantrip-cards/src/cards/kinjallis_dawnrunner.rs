use cantrip_api::*;
use cantrip_mechanics::{explore, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, it explores." (CR 701.44a, 603.6a). The creature explores even if it has left the battlefield
/// by the time the ability resolves (CR 701.44c: its last known information says who controlled it).
fn explores(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    explore(g, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, explores, "When this creature enters, it explores.")],
    ..oracle::KINJALLIS_DAWNRUNNER
};
