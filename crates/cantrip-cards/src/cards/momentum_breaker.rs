use cantrip_api::*;
use cantrip_mechanics::start_your_engines::speed;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{each_discards_a_card, each_opponent_sacrifices_choices, targets};
use cantrip_oracle as oracle;

/// "When this enchantment enters, each opponent sacrifices a creature or Vehicle of their choice. Each opponent who can't
/// discards a card." (CR 701.21a, 101.4, 701.9): the opponents choose and sacrifice together; one who sacrificed nothing -
/// no creature or Vehicle, or none an opponent's ability can make them sacrifice (CR 101.2) - then discards, in turn order.
fn edict_or_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let sacrificed = each_opponent_sacrifices_choices(g, item.controller, item.object, 1, targets::creature_or_vehicle)?;
    let cant: Vec<PlayerId> = g.state().apnap().filter(|&p| p != item.controller && !sacrificed.iter().any(|&(who, _)| who == p)).collect();
    each_discards_a_card(g, &cant, item.object)?;
    Ok(())
}

/// "You gain life equal to your speed" (CR 702.179f): 0 for a player with no speed.
fn gain_life_equal_to_speed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let life = i32::from(speed(g.state(), item.controller));
    g.gain_life(item.controller,(life) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[
        triggered(THIS_ENTERS, edict_or_discard, "When this enchantment enters, each opponent sacrifices a creature or Vehicle of their choice. Each opponent who can't discards a card."),
        ability(cost("{2}").and(SACRIFICE_THIS), gain_life_equal_to_speed, "{2}, Sacrifice this enchantment: You gain life equal to your speed."),
    ],
    ..oracle::MOMENTUM_BREAKER
};
