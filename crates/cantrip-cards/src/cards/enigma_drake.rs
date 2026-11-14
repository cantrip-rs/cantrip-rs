use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "Enigma Drake's power is equal to the number of instant and sorcery cards in your graveyard" (CR 604.3, layer 7a): a
/// characteristic-defining ability, counted whenever characteristics are computed.
fn power_is_instant_and_sorcery_cards_in_your_graveyard(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.power = Some(statics::instant_and_sorcery_cards_in_your_graveyard(q, source) as i32);
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_is_instant_and_sorcery_cards_in_your_graveyard,
        "Enigma Drake's power is equal to the number of instant and sorcery cards in your graveyard.",
    )
    .lost_pt(0, 4)],
    ..oracle::ENIGMA_DRAKE
};
