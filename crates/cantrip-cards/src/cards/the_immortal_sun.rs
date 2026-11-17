use cantrip_api::*;
use cantrip_mechanics::cost_reduction;
use cantrip_mechanics::statics::CREATURES_YOU_CONTROL_GET_PLUS_ONE;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Players can't activate planeswalkers' loyalty abilities." (CR 602.5, 606.3): a planeswalker permanent's ability with
/// a loyalty cost can't be activated, by either player; anything else - a planeswalker's non-loyalty abilities, and
/// abilities in other zones - still can.
fn planeswalker_loyalty_cant_activate(q: &dyn Query, _: &Asker, _: PlayerId, source: ObjectId, def: &ActivatedDef) -> bool {
    !q.is_on_battlefield(source) || !q.chars(source).is(CardType::Planeswalker) || def.cost.loyalty.is_none()
}

/// "At the beginning of your draw step, draw an additional card." (CR 603.2, 121.1): one extra card beside the
/// turn-based draw.
fn draw_an_additional_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// "Creatures you control get +1/+1." (CR 604.1): the shared anthem, in layer 7c beside Glorious Anthem.
pub static CARD: CardDef = CardDef {
    activation_rules: &[planeswalker_loyalty_cant_activate],
    abilities: &[triggered(
        triggers::YOUR_DRAW_STEP,
        draw_an_additional_card,
        "At the beginning of your draw step, draw an additional card.",
    )],
    cost_changes: &[cost_reduction::spells_you_cast_cost_1_less],
    statics: &[CREATURES_YOU_CONTROL_GET_PLUS_ONE],
    ..oracle::THE_IMMORTAL_SUN
};
