use cantrip_api::*;
use cantrip_mechanics::{discover, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, if you cast it, discover 3." (CR 701.57a). The intervening "if" (CR 603.4) is the trigger itself:
/// it fires only for the cast spell's own entering, and not when the creature is put onto the battlefield some other way.
fn discover_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discover::discover(g, item.controller, 3)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS_IF_CAST, discover_3, "When this creature enters, if you cast it, discover 3.")],
    ..oracle::GEOLOGICAL_APPRAISER
};
