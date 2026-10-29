use cantrip_api::*;
use cantrip_mechanics::{choose, library};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, you may collect evidence 6" (CR 701.59a, 601.2b): announced like any
/// optional additional cost; the spell has `paid` set when it was paid.
const OPTIONAL_COST: OptionalCost = OptionalCost::new(Cost { collect_evidence: 6, ..Cost::NONE });

const LABELS: &[&str] = &["Cast without collecting evidence", "Collect evidence 6"];

/// "Destroy all creatures. If evidence was collected, exile a card from an opponent's graveyard. Then search its
/// owner's graveyard, hand, and library for any number of cards with that name and exile them. That player shuffles,
/// then draws a card for each card exiled from their hand this way."
///
/// The creatures go together (they may be regenerated), and the ones of the opponents land in their graveyards first,
/// so they may be the card exiled. You choose the card from among all your opponents' graveyards, and you do the
/// searching, in the order the text names the zones; each zone is a search for any number (CR 701.23), so cards of the
/// name may be left. The card exiled is still the object whose name is asked after it has moved (CR 400.7), and the
/// found cards are exiled together. With no card to exile, "its owner" is nobody and nothing more happens.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let creatures: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    g.destroy_all(&creatures);
    if item.paid == 0 {
        return Ok(());
    }
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    let graveyards: Vec<ObjectId> = opponents.iter().flat_map(|&p| g.state().zone(Zone::graveyard(p)).to_vec()).collect();
    let Some(chosen) = choose::card(g, you, item.object, Purpose::Exile, graveyards)? else { return Ok(()) };
    let owner = g.obj(chosen).owner;
    let names = library::object_names(g, chosen);
    g.exile(chosen);
    library::exile_same_name_from_graveyard_hand_and_library(g, you, owner, item.object, &names)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    optional_cost: Some(OPTIONAL_COST),
    labels: LABELS,
    ..oracle::DEADLY_COVER_UP
};
