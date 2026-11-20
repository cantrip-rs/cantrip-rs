use cantrip_api::*;
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

/// "This spell costs {X} less to cast, where X is the number of cards in your graveyard that are instant cards, sorcery
/// cards, and/or have an Adventure" (CR 601.2f, 118.7a): each such card counts once, however many of those it is, counted
/// as the cost is determined. A card that has an Adventure counts in the graveyard whether or not it was ever cast as one
/// (its ruling), and a card the graveyard holds counts as the card it is there, so a creature card with an Adventure is not
/// an instant or a sorcery but has one. It is asked only of the creature spell's cost (the card has no abilities, and the
/// Adventure Stoke Genius has no such text of its own).
fn less_per_instant_sorcery_or_adventure_in_your_graveyard(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let cards = q
        .state()
        .zone(Zone::graveyard(player))
        .iter()
        .filter(|&&id| {
            let chars = q.chars(id);
            chars.is(CardType::Instant) || chars.is(CardType::Sorcery) || library::card_has_an_adventure(q, id)
        })
        .count();
    CostDelta::less(cards as u32)
}

/// The Adventure (CR 715): "Discard your hand, then draw two cards."
fn stoke_genius(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::discard_hand_then_draw(g, item, 2)
}

pub static STOKE_GENIUS: CardDef = CardDef { spell: spell(&[], stoke_genius), ..oracle::STOKE_GENIUS };

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_per_instant_sorcery_or_adventure_in_your_graveyard],
    adventure: Some(&STOKE_GENIUS),
    ..oracle::HEARTH_ELEMENTAL
};

pub static EXTRA: &[&CardDef] = &[&STOKE_GENIUS];
