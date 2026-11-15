use cantrip_api::*;
use cantrip_mechanics::{effects, hand};
use cantrip_oracle as oracle;

/// "Draw two cards. If the discarded card wasn't a land card, this spell deals 2 damage to each opponent." (CR 608.2c, 601.2h,
/// 120.3): the card the additional cost discarded, a card in the graveyard now but with the types it had in hand
/// (`hand::discarded_chars`; one whose characteristics are undefined, CR 701.9c, is no land). The damage is
/// dealt whatever was drawn.
fn draw_two_then_damage_if_the_discarded_card_was_not_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 2)?;
    g.end_sentence();
    let not_a_land = g
        .history()
        .spell_cost_objects(item.object, CostOrigin::PrintedAdditional, 0, CostPart::Discard(0))
        .next()
        .and_then(|discarded| g.obj(discarded).became)
        .is_some_and(|card| !hand::discarded_chars(&*g, card).is_some_and(|chars| chars.is(CardType::Land)));
    if not_a_land {
        effects::damage_each_opponent(g, item, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], draw_two_then_damage_if_the_discarded_card_was_not_a_land),
    // "As an additional cost to cast this spell, discard a card" (CR 601.2b, 118.8): exactly one, paid with the other costs.
    additional_cost: Some(discard_cards(1)),
    ..oracle::GRAB_THE_PRIZE
};
