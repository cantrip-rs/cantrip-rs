use cantrip_api::*;
use cantrip_mechanics::{tokens, typecycling};
use cantrip_oracle as oracle;

/// "Create two 3/2 white Hero creature tokens with vigilance." (CR 111.2, 101.4): both at once, under the spell's controller.
fn two_heroes(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::HERO_3_2_WHITE_VIGILANCE, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], two_heroes),
    abilities: &[typecycling::basic_landcycling(mana("{2}"), "Basic landcycling {2}")],
    ..oracle::BOROUGH_BACKUP
};

pub static EXTRA: &[&CardDef] = &[&tokens::HERO_3_2_WHITE_VIGILANCE];
