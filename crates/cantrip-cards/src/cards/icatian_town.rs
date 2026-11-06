use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create four 1/1 white Citizen creature tokens." (CR 701.7a).
fn create_four_1_1_white(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::CITIZEN_1_1_WHITE, 4)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], create_four_1_1_white),
    ..oracle::ICATIAN_TOWN
};

pub static EXTRA: &[&CardDef] = &[&tokens::CITIZEN_1_1_WHITE];
