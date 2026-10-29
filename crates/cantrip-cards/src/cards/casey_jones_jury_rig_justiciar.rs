use cantrip_api::*;
use cantrip_mechanics::library::{self, Rest};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library. You may reveal an artifact card from among them and put it into your hand.
/// Put the rest on the bottom of your library in a random order." (CR 401.4).
fn look_for_an_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::look_at_top(g, item.controller, item.object, 4, (0, 1), true, |chars| chars.is(CardType::Artifact), Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        look_for_an_artifact,
        "When Casey Jones enters, look at the top four cards of your library. You may reveal an artifact card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::CASEY_JONES_JURY_RIG_JUSTICIAR
};
