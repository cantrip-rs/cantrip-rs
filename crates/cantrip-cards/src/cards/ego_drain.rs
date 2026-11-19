use cantrip_api::*;
use cantrip_mechanics::{choose, hand::discard_chosen_nonland_from_revealed_hand, lands, select, statics, targets};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card. If you don't control a
/// Faerie, exile a card from your hand." (CR 701.9a, 701.13, 608.2c): the discard is Duress's shape. The Faerie is looked for
/// after it, as the spell resolves, through the words of this card's text (CR 612): any permanent you control with that creature
/// type now. Without one, you must exile a card of your own hand (not a choice to decline, CR 608.2d), none being asked for when the
/// hand is empty.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    discard_chosen_nonland_from_revealed_hand(g, item, opponent)?;
    if statics::you_control_a::<{ lands::sub(Subtype::Faerie) }>(g, &Asker::spell(item.object, item.controller)) {
        return Ok(());
    }
    let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
    let chosen = choose::select(g, item.controller, item.object, Purpose::Exile, hand, select::CARD)?;
    g.exile_all(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], effect),
    ..oracle::EGO_DRAIN
};
