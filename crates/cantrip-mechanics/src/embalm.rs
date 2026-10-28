//! Embalm [cost] (CR 702.128a): "[Cost], Exile this card from your graveyard: Create a token that's a copy of this card,
//! except it's white, it has no mana cost, and it's a Zombie in addition to its other types. Activate only as a
//! sorcery." Aven Initiate, Honored Hydra, Oketra's Attendant, Sacred Cat, Tah-Crop Skirmisher and Unwavering Initiate
//! print it. It is eternalize (`eternalize`) with other exceptions, so both go through `copy_from_graveyard`.
//!
//! A card with embalm lists `ability(cost, text)` among its `abilities`.

use cantrip_api::*;

/// The token is a copy of the card as it was in the graveyard (CR 608.2h), white, costless and a Zombie in addition to its
/// other types, its size left as the card prints it.
fn make_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let values = g.copiable_values(item.object).colored(ColorSet::of(Color::White)).costless().with_subtype(Subtype::Zombie);
    g.create_token_copies_of(values, item.controller, 1)?;
    Ok(())
}

pub const fn ability(cost: ManaCost, text: &'static str) -> AbilityDef {
    crate::copy_from_graveyard::ability(cost, make_token, text)
}
