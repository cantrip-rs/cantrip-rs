use cantrip_api::*;
use cantrip_mechanics::{connive, triggers, typecycling};
use cantrip_oracle as oracle;

/// "When this creature enters, it connives." (CR 701.50a, 603.6a). It connives even if it has left the battlefield by the
/// time the ability resolves; it then takes no counter (CR 701.50b).
fn connives(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    connive(g, item.object, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, connives, "When this creature enters, it connives."),
        typecycling::basic_landcycling(mana("{2}"), "Basic landcycling {2}"),
    ],
    ..oracle::A_I_M_SCIENTISTS
};
