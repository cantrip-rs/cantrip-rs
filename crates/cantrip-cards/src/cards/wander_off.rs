use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Exile target creature" (CR 701.13a): nothing if it is no longer a legal target as this resolves (CR 608.2b).
fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::exile_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], exile_it),
    ..oracle::WANDER_OFF
};
