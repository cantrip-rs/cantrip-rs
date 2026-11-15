use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "X is the number of creatures you control with power 2 or less", counted once as the ability resolves (CR 608.2h, 208.1):
/// Arabella herself counts while her power is 2 or less, and a creature with no power does not.
fn drain_for_small_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
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
    abilities: &[triggered(
        THIS_ATTACKS,
        drain_for_small_creatures,
        "Whenever this creature attacks, it deals X damage to each opponent and you gain X life, where X is the number of creatures you control with power 2 or less.",
    )],
    ..oracle::ARABELLA_ABANDONED_DOLL
};
