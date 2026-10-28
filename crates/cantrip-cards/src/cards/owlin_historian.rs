use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, surveil 1." (CR 701.25.)
fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

/// Flying is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, surveil_one, "When this creature enters, surveil 1."),
        // "Whenever one or more cards leave your graveyard" (CR 603.2c): the cards that left together trigger it once.
        triggered(triggers::CARDS_LEAVE_YOUR_GRAVEYARD, effects::this_gets::<1, 1>, "Whenever one or more cards leave your graveyard, this creature gets +1/+1 until end of turn.")
            .once_per_batch(),
    ],
    ..oracle::OWLIN_HISTORIAN
};
