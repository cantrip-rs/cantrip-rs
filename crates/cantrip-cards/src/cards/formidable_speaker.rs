use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets;
use cantrip_mechanics::library::{search_to_hand, Find};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// When this creature enters, you may discard a card. If you do, search your library for a creature card, reveal it,
/// put it into your hand, then shuffle. An empty hand asks nothing; choosing no card declines.
fn discard_and_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    if !effects::may_discard_a_card(g, item)? {
        return Ok(());
    }
    search_to_hand(g, player, item.object, Find::ONE.revealed(), |q, c| q.chars(c).is(CardType::Creature))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, discard_and_search, "When this creature enters, you may discard a card. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle."),
        ability(cost("{1}, {T}"), effects::untap_targets, "{1}, {T}: Untap another target permanent.")
            .targets(&[targets::OTHER_PERMANENT]),
    ],
    ..oracle::FORMIDABLE_SPEAKER
};
