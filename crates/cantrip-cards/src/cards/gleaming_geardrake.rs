use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOU_SACRIFICE_AN_ARTIFACT};
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a)
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, investigate, "When this creature enters, investigate."),
        triggered(
            YOU_SACRIFICE_AN_ARTIFACT,
            effects::plus_one_counter_on_this,
            "Whenever you sacrifice an artifact, put a +1/+1 counter on this creature.",
        )
        .on_sacrifice(),
    ],
    ..oracle::GLEAMING_GEARDRAKE
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
