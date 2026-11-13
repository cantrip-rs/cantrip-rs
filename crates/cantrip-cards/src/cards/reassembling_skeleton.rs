use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{B}"), effects::return_this_card_to_battlefield_tapped, "{1}{B}: Return this card from your graveyard to the battlefield tapped.")
        .from(ZoneKind::Graveyard)],
    ..oracle::REASSEMBLING_SKELETON
};
