use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], effects::draw_two_cards_and_lose_2_life), ..oracle::NIGHTS_WHISPER };
