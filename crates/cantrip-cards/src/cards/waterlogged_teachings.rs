use cantrip_api::*;
use cantrip_mechanics::library::search_for_an_instant_or_flash_card_to_hand;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static BACK: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Blue, Color::Black]), "{T}: Add {U} or {B}.")],
    enters: Some(mana::enters_tapped),
    ..oracle::INUNDATED_ARCHIVE
};

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_for_an_instant_or_flash_card_to_hand),
    back: Some(&BACK),
    ..oracle::WATERLOGGED_TEACHINGS
};
