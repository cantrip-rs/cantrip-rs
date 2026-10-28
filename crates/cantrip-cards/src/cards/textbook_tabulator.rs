use cantrip_api::*;
use cantrip_mechanics::{effects, increment, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Surveil 2" (CR 701.25a).
fn surveil_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        increment::TRIGGER,
        triggered(THIS_ENTERS, surveil_two, "When this creature enters, surveil 2."),
    ],
    ..oracle::TEXTBOOK_TABULATOR
};
