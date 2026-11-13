use cantrip_api::*;
use cantrip_mechanics::flashback;
use cantrip_mechanics::library::search_for_an_instant_or_flash_card_to_hand;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_for_an_instant_or_flash_card_to_hand),
    ways: &[flashback::way(cost("{5}{B}"), "Flashback {5}{B}")],
    ..oracle::MYSTICAL_TEACHINGS
};
