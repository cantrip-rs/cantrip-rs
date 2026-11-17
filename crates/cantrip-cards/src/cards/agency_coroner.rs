use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Draw a card. If the sacrificed creature was suspected, draw two cards instead." (CR 701.60b, 608.2h, 121.1): the creature chosen for
/// the cost is read as it last was on the battlefield, so its designation counts although it is already in a graveyard.
fn draw_a_card_or_two_if_the_sacrificed_creature_was_suspected(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let suspected = g.cost_objects(item).first().is_some_and(|&creature| g.was_suspected(creature));
    g.draw(item.controller, if suspected { 2 } else { 1 });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{B}").and(sacrifice(1, targets::filters::another_creature)),
        draw_a_card_or_two_if_the_sacrificed_creature_was_suspected,
        "{2}{B}, Sacrifice another creature: Draw a card. If the sacrificed creature was suspected, draw two cards instead.",
    )],
    ..oracle::AGENCY_CORONER
};
