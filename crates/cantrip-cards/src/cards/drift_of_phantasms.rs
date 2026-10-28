use cantrip_api::*;
use cantrip_mechanics::transmute::transmute;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[transmute(
        mana("{1}{U}{U}"),
        "Transmute {1}{U}{U} ({1}{U}{U}, Discard this card: Search your library for a card with the same mana value as this card, reveal it, put it into your hand, then shuffle. Transmute only as a sorcery.)",
    )],
    ..oracle::DRIFT_OF_PHANTASMS
};
