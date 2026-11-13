use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::each_opponent_loses_life_you_gain_life::<3>, "When this Vehicle enters, each opponent loses 3 life and you gain 3 life."),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::DEBRIS_BEETLE
};
