use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOU_ATTACK;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOU_ATTACK, effects::loot, "Whenever you attack, draw a card, then discard a card."),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::BOOSTED_SLOOP
};
