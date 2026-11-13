use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], library::search_for_a_card_to_hand), ..oracle::DEMONIC_TUTOR };
