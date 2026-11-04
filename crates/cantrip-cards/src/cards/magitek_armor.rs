use cantrip_api::*;
use cantrip_mechanics::{crew::crew, tokens, triggers};
use cantrip_oracle as oracle;

/// "When this Vehicle enters, create a 1/1 colorless Hero creature token" (CR 603.6a, 111.1): the controller of the ability
/// creates it.
fn hero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::HERO_1_1_COLORLESS, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, hero, "When this Vehicle enters, create a 1/1 colorless Hero creature token."),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::MAGITEK_ARMOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::HERO_1_1_COLORLESS];
