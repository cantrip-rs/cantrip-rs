use cantrip_api::*;
use cantrip_mechanics::{library, pump, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target creature an opponent controls gets -X/-X until end of turn, where X is the greatest mana value among cards in your
/// graveyard" (CR 202.3, 608.2h): X is read once, as the ability resolves, from the cards in its controller's graveyard (the
/// ones just milled included), a land's being 0, and is a fixed amount from then on (CR 611.2a).
fn target_gets_minus_x_minus_x_for_greatest_mana_value_in_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = g.state().zone(Zone::graveyard(item.controller)).to_vec().into_iter().map(|card| g.mana_value(card)).fold(0, u32::max);
    let x = i32::try_from(x).unwrap_or(i32::MAX);
    pump(g, item.object, creature, -x, -x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        library::mill_then_reflexive_trigger::<3>,
        "When this creature enters, mill three cards. When you do, target creature an opponent controls gets -X/-X until end of turn, where X is the greatest mana value among cards in your graveyard.",
    )],
    delayed: &[delayed_trigger(
        target_gets_minus_x_minus_x_for_greatest_mana_value_in_your_graveyard,
        "When you do, target creature an opponent controls gets -X/-X until end of turn, where X is the greatest mana value among cards in your graveyard.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::WICKS_PATROL
};
