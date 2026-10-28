use cantrip_api::*;
use cantrip_mechanics::affinity;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "If you have fewer than four cards in hand" (CR 603.4): an intervening "if", checked as the ability triggers and again as it
/// resolves.
fn fewer_than_four_cards_in_hand(q: &dyn Query, asker: &Asker) -> bool {
    q.state().zone(Zone::hand(asker.controller)).len() < 4
}

/// "Draw cards equal to the difference": four less the cards in hand now, as it resolves (CR 608.2c).
fn draw_the_difference(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let in_hand = g.state().zone(Zone::hand(item.controller)).len();
    g.draw(item.controller, 4u32.saturating_sub(in_hand as u32));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_artifacts],
    abilities: &[triggered(THIS_ENTERS, draw_the_difference, "When Krang enters, if you have fewer than four cards in hand, draw cards equal to the difference.")
        .condition(fewer_than_four_cards_in_hand)],
    statics: &[static_ability(Layer::PtModify, cantrip_mechanics::statics::itself, cantrip_mechanics::statics::plus_power_per_other_artifact_you_control, "Krang gets +1/+0 for each other artifact you control.")],
    ..oracle::KRANG_MASTER_MIND
};
