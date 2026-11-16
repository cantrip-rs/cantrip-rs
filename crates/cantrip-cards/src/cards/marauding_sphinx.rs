use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Flying and vigilance are core keywords read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(triggers::YOU_COMMIT_A_CRIME, |g, item| effects::surveil(g, item, 2), "Whenever you commit a crime, surveil 2. This ability triggers only once each turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)")
            .triggers_each_turn(1),
    ],
    ..oracle::MARAUDING_SPHINX
};
