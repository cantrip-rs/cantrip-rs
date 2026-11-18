use cantrip_api::*;
use cantrip_mechanics::{disguise, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, you lose 3 life." (CR 119.3.)
fn you_lose_three_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, you_lose_three_life, "When this creature enters, you lose 3 life.")],
    ways: &[disguise::way(Cost::mana(mana("{B}{B}")), "Disguise {B}{B}")],
    ..oracle::NIGHTDRINKER_MOROII
};
