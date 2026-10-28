use cantrip_api::*;
use cantrip_mechanics::echo;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[echo!(discard_cards(1), echo::DISCARD_A_CARD, "Echo—Discard a card. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)")],
    ..oracle::RAKDOS_HEADLINER
};
