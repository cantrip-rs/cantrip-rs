use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, look at the top four cards of your library. You may reveal an artifact or Pirate card from among
/// them and put it into your hand. Put the rest on the bottom of your library in a random order." (CR 401.4, 701.20a): the Pirate
/// type is read through the words of the Crewmate's own text (CR 612).
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let pirate = g.words(item.object).subtype(Subtype::Pirate);
    look_at_top(g, item.controller, item.object, 4, (0, 1), true, |c| c.is(CardType::Artifact) || c.has_subtype(pirate), Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        look,
        "When this creature enters, look at the top four cards of your library. You may reveal an artifact or Pirate card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::STAUNCH_CREWMATE
};
