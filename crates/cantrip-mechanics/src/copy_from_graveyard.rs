//! The ability eternalize (CR 702.129a) and embalm (CR 702.128a) share: "[Cost], Exile this card from your graveyard: Create
//! a token that's a copy of it, except ... Activate only as a sorcery." The two differ only in the exceptions, which the
//! caller's `make_token` makes.

use cantrip_api::*;

/// The ability, `make_token` its effect: it makes the token from the card as it was in the graveyard, whose copiable values
/// are read from the object that was exiled to pay (CR 608.2h).
pub(crate) const fn ability(cost: ManaCost, make_token: Effect, text: &'static str) -> AbilityDef {
    cantrip_api::ability(Cost::mana(cost).and(EXILE_THIS), make_token, text).sorcery_speed().from(ZoneKind::Graveyard)
}
