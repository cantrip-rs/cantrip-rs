use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "Whenever Arabella attacks, it deals X damage to each opponent and you gain X life, where X is the number of creatures
/// you control with power 2 or less." X is determined once, as the ability resolves (ruling 2024-09-20; CR 608.2h), from
/// the layered power of each creature the controller controls then; the damage and the life gain use that one value.
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| {
            let c = g.chars(id);
            c.is(CardType::Creature) && c.power.is_some_and(|p| p <= 2)
        })
        .count() as u32;
    effects::damage_each_opponent_and_gain_life(g, item, x)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, drain, "Whenever Arabella attacks, it deals X damage to each opponent and you gain X life, where X is the number of creatures you control with power 2 or less.")],
    ..oracle::ARABELLA_ABANDONED_DOLL
};
