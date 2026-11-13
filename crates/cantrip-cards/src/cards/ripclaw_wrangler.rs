use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::effects::each_opponent_discards_a_card;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, each_opponent_discards_a_card, "When this Vehicle enters, each opponent discards a card."),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::RIPCLAW_WRANGLER
};
