use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target opponent mills three cards. Exile up to one noncreature, nonland card from that player's graveyard. For as long as
/// you control Hama, you may cast the exiled card during your turn by waterbending {X} rather than paying its mana cost, where
/// X is its mana value." (CR 701.17a, 608.2d, 611.2a, 701.67a, 118.9): the cards are milled first, so one of them may be the
/// card exiled; the ability's controller chooses it, among the graveyard's cards that are neither creatures nor lands, and may
/// choose none. The exiled card's mana value is read once it is in exile, an {X} in its cost then counting as 0 (CR 202.3e), and
/// is the waterbend cost the permission asks instead of the mana cost. It lasts for as long as the ability's controller
/// controls Hama (CR 611.2b): not at all if they do not as it resolves (CR 113.8), and for good once they stop (CR 400.7).
fn mill_and_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    g.mill(opponent, 3);
    g.end_sentence();
    let eligible = {
        let q: &dyn Query = g;
        q.state().zone(Zone::graveyard(opponent)).iter().copied().filter(|&id| !q.chars(id).is(CardType::Creature) && !q.chars(id).is(CardType::Land)).collect::<Vec<_>>()
    };
    let chosen = choose::select(g, item.controller, item.object, Purpose::Exile, eligible, Selection::of(select::up_to(1), &[]))?;
    let exiled = g.exile_all(&chosen);
    g.end_sentence();
    if let Some(&card) = exiled.first() {
        let cost = generic(g.mana_value(card));
        g.allow_play(
            card,
            item.controller,
            PlayGrant { instead: Some(cost), waterbend: true, when: Some(PlayCondition::DuringYourTurn), expiry: Expiry::WhileControls(item.object), ..PlayGrant::CAST },
        );
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        mill_and_exile,
        "When Hama enters, target opponent mills three cards. Exile up to one noncreature, nonland card from that player's graveyard. For as long as you control Hama, you may cast the exiled card during your turn by waterbending {X} rather than paying its mana cost, where X is its mana value.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::HAMA_THE_BLOODBENDER
};
