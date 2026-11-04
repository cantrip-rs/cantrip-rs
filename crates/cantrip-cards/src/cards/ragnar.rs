use cantrip_api::*;
use cantrip_mechanics::{regenerate, targets};
use cantrip_oracle as oracle;

fn regenerate_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    regenerate::target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{G}{W}{U}, {T}"), regenerate_creature, "{G}{W}{U}, {T}: Regenerate target creature.")
        .targets(&[targets::CREATURE])],
    ..oracle::RAGNAR
};
