use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature dies, create a 1/1 colorless Hero creature token." (CR 700.4, 111.4)
fn dies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::HERO_1_1_COLORLESS, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, dies, "When this creature dies, create a 1/1 colorless Hero creature token.")],
    ..oracle::DWARVEN_CASTLE_GUARD
};

pub static EXTRA: &[&CardDef] = &[&tokens::HERO_1_1_COLORLESS];
