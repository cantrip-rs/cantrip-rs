use cantrip_api::*;
use cantrip_mechanics::{effects::gain_three_life, library::look_at_top_take_one_rest_to_graveyard};
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard. You gain 3
/// life." (CR 701.20e, 400.3, 119.3): `look_at_top_take_one_rest_to_graveyard`, then the life, which is gained even from a library of
/// fewer than four cards.
fn commune(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 4)?;
    gain_three_life(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], commune), ..oracle::COMMUNE_WITH_EVIL };
