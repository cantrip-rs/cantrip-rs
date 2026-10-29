use cantrip_api::*;
use cantrip_mechanics::{beam_me_up, tokens};
use cantrip_mechanics::tokens::KLINGON;
use cantrip_oracle as oracle;

fn create_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &KLINGON, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], create_token),
    ways: &[beam_me_up::way(mana("{2}{R}"), "Beam me up {2}{R}")],
    ..oracle::BOARDING_ACTION
};

pub static EXTRA: &[&CardDef] = &[&KLINGON];
