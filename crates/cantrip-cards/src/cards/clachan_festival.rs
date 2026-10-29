use cantrip_api::*;
use cantrip_mechanics::tokens::{self, KITHKIN};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this enchantment enters, create two 1/1 green and white Kithkin creature tokens."
fn two_kithkin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &KITHKIN, 2)?;
    Ok(())
}

/// "{4}{W}: Create a 1/1 green and white Kithkin creature token."
fn one_kithkin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &KITHKIN, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&KITHKIN];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, two_kithkin, "When this enchantment enters, create two 1/1 green and white Kithkin creature tokens."),
        ability(cost("{4}{W}"), one_kithkin, "{4}{W}: Create a 1/1 green and white Kithkin creature token."),
    ],
    ..oracle::CLACHAN_FESTIVAL
};
