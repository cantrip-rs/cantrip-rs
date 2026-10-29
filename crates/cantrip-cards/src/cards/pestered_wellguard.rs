use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_oracle as oracle;

/// "Create a 1/1 blue and black Faerie creature token with flying."
fn faerie(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::FAERIE_BLUE_BLACK, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_BECOMES_TAPPED, faerie, "Whenever this creature becomes tapped, create a 1/1 blue and black Faerie creature token with flying.")],
    ..oracle::PESTERED_WELLGUARD
};

pub static EXTRA: &[&CardDef] = &[&tokens::FAERIE_BLUE_BLACK];
