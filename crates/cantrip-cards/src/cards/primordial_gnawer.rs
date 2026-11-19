use cantrip_api::*;
use cantrip_mechanics::{discover, triggers};
use cantrip_oracle as oracle;

/// "Discover 3" (CR 701.57a): the controller of the ability discovers, which is its controller as the creature died (CR 603.10a).
fn discover_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discover::discover(g, item.controller, 3)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, discover_three, "When this creature dies, discover 3.")],
    ..oracle::PRIMORDIAL_GNAWER
};
