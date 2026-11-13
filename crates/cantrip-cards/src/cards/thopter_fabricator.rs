use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::tokens::{create_a_thopter, THOPTER};
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&THOPTER];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOU_DRAW_YOUR_SECOND_CARD,
            create_a_thopter,
            "Whenever you draw your second card each turn, create a 1/1 colorless Thopter artifact creature token with flying.",
        ),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::THOPTER_FABRICATOR
};
