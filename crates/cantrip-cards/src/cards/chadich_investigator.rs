use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

fn create_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    tokens::create(g, player, &tokens::KLINGON, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, create_token, "When this creature enters, create a 2/1 red Klingon Warrior creature token with haste.")
    ],
    ..oracle::CHADICH_INVESTIGATOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::KLINGON];
