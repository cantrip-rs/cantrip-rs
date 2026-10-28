use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{2}{B}: Return this card from your graveyard to your hand." An activated ability that functions only in its owner's
/// graveyard (CR 113.6k), which `from` says; nothing if the card has left it by the time the ability resolves (CR 400.7).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{B}"), effects::return_this_to_hand, "{2}{B}: Return this card from your graveyard to your hand.")
        .from(ZoneKind::Graveyard)],
    ..oracle::PROJECT_DEATHLOK_SOLDIER
};
