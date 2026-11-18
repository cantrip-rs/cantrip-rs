use cantrip_api::*;
use cantrip_mechanics::effects::this_gets_and_gains_vigilance;
use cantrip_mechanics::tokens::{self, you_investigate};
use cantrip_mechanics::triggers::{THIS_ENTERS, YOU_DRAW_YOUR_SECOND_CARD};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, you_investigate, "When this creature enters, investigate."),
        triggered(
            YOU_DRAW_YOUR_SECOND_CARD,
            this_gets_and_gains_vigilance::<1, 1>,
            "Whenever you draw your second card each turn, this creature gets +1/+1 and gains vigilance until end of turn.",
        ),
    ],
    ..oracle::LOXODON_EAVESDROPPER
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
