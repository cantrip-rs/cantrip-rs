use cantrip_api::*;
use cantrip_mechanics::{effects, gift, targets};
use cantrip_oracle as oracle;

/// "Return target permanent card from your graveyard to your hand. If the gift was promised, instead return two target
/// permanent cards from your graveyard to your hand." The promise is announced before the targets are chosen (CR
/// 601.2b-c, 702.174m), so the one slot asks for two when it was (`if_promised`); the gift itself is given before this runs
/// (CR 702.174j). Each target still in the graveyard returns together (CR 400.3, 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD.if_promised(2, 2)], effects::return_each_target_to_hand),
    gift: Some(gift::CARD),
    ..oracle::PEERLESS_RECYCLING
};
