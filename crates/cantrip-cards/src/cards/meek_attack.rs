use cantrip_api::*;
use cantrip_mechanics::hand;
use cantrip_oracle as oracle;

/// "You may put a creature card with total power and toughness 5 or less from your hand onto the battlefield. That creature
/// gains haste. At the beginning of the next end step, sacrifice that creature." (CR 608.2d, 611.2a, 603.7): the total is of
/// the card's printed power and toughness in the hand, a creature card with none counting 0 for it. Nobody is asked with no
/// card that small in hand; nothing follows if the card does not stay on the battlefield.
fn put_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let small = |c: &Chars| c.is(CardType::Creature) && c.power.into_iter().chain(c.toughness).map(i32::from).sum::<i32>() <= 5;
    let Some(returned) = hand::may_put_onto_battlefield(g, item.controller, item.object, small)? else { return Ok(()) };
    if !g.is_on_battlefield(returned) {
        return Ok(());
    }
    cantrip_mechanics::effects::gain_haste(g, item.object, returned);
    cantrip_mechanics::delayed::at_next_end_step(g, item, 0, Target::Object(returned));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{R}"), put_creature, "{1}{R}: You may put a creature card with total power and toughness 5 or less from your hand onto the battlefield. That creature gains haste. At the beginning of the next end step, sacrifice that creature.")],
    delayed: &[delayed_trigger(cantrip_mechanics::delayed::sacrifice_subject, "Sacrifice that creature.")],
    ..oracle::MEEK_ATTACK
};
