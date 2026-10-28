//! Eternalize [cost] (CR 702.129a): "[Cost], Exile this card from your graveyard: Create a token that's a copy of it,
//! except it's a 4/4 black Zombie with no mana cost. Activate only as a sorcery." Fanatic of Rhonas prints it.
//!
//! A card with eternalize lists `ability(cost, text)` among its `abilities`.

use cantrip_api::*;

/// "It" is the card as it was in the graveyard; the exceptions become part of the token's own copiable values (CR 707.9b),
/// and a color or size the card defines is left behind (CR 707.9d).
fn make_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let values = g
        .copiable_values(item.object)
        .colored(ColorSet::of(Color::Black))
        .sized(4, 4)
        .costless()
        .with_subtype(Subtype::Zombie);
    g.create_token_copies_of(values, item.controller, 1)?;
    Ok(())
}

pub const fn ability(cost: ManaCost, text: &'static str) -> AbilityDef {
    crate::copy_from_graveyard::ability(cost, make_token, text)
}
