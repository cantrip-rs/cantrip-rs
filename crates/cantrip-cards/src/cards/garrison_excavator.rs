use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 2/2 red and white Spirit creature token."
fn spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT_2_2_RED_WHITE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CARDS_LEAVE_YOUR_GRAVEYARD,
        spirit,
        "Whenever one or more cards leave your graveyard, create a 2/2 red and white Spirit creature token.",
    )
    .once_per_batch()],
    ..oracle::GARRISON_EXCAVATOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIRIT_2_2_RED_WHITE];
