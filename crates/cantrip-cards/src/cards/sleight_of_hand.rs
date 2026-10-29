use cantrip_api::*;
use cantrip_mechanics::library::look_at_top_two_take_one_other_on_bottom;
use cantrip_oracle as oracle;

/// "Look at the top two cards of your library. Put one of them into your hand and the other on the bottom of your
/// library." (CR 401.2, 609.3)
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top_two_take_one_other_on_bottom(g, item.controller, item.object)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::SLEIGHT_OF_HAND };
