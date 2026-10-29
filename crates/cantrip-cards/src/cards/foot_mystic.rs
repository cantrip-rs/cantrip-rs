use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{history, tokens};
use cantrip_oracle as oracle;

fn create_a_ninja(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::NINJA, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        create_a_ninja,
        "Disappear \u{2014} When this creature enters, if a permanent left the battlefield under your control this turn, create a 1/1 black Ninja creature token.",
    )
    .condition(history::disappear)],
    ..oracle::FOOT_MYSTIC
};

pub static EXTRA: &[&CardDef] = &[&tokens::NINJA];
