use cantrip_api::*;
use cantrip_mechanics::{hand, targets};
use cantrip_oracle as oracle;

/// "You may put a creature card from your hand onto the battlefield." (CR 608.2d)
fn put_creature_from_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::may_put_onto_battlefield(g, item.controller, item.object, |c| c.is(CardType::Creature))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(3, targets::filters::another_creature),
        put_creature_from_hand,
        "Sacrifice three other creatures: You may put a creature card from your hand onto the battlefield. Activate only as a sorcery and only once each turn.",
    )
    .sorcery_speed()
    .once_each_turn()],
    ..oracle::AKUL_THE_UNREPENTANT
};
