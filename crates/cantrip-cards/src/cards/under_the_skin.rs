use cantrip_api::*;
use cantrip_mechanics::manifest_dread::manifest_dread;
use cantrip_mechanics::select::{up_to, Selection, PERMANENT_CARD};
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// "You may return a permanent card from your graveyard to your hand" (CR 110.4a): up to one, chosen on resolution.
const PERMANENT_CARD_UP_TO_ONE: Selection = Selection::of(up_to(1), &[PERMANENT_CARD]).called("permanent");

/// "Manifest dread. You may return a permanent card from your graveyard to your hand." The card the manifest dread put into the graveyard
/// is already there when the choice is made (CR 608.2c).
fn manifest_dread_then_return_a_permanent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    manifest_dread(g, item.controller, item.object)?;
    g.end_sentence();
    library::return_a_card_from_your_graveyard(g, item, PERMANENT_CARD_UP_TO_ONE)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], manifest_dread_then_return_a_permanent), ..oracle::UNDER_THE_SKIN };
