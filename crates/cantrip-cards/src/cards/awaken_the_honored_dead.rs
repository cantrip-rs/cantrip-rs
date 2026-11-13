use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Chapter III: "You may discard a card. When you do, return target creature or land card from your graveyard to your hand."
/// The discard is a choice made as the chapter resolves, not a cost, so nothing is asked of an empty hand; the reflexive
/// ability (CR 603.12) follows a card really discarded, and chooses its target as it goes on the stack, the card just
/// discarded included.
fn discard_for_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::may_discard_a_card(g, item)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "Target creature or land card in your graveyard" (CR 115.1).
fn creature_or_land_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature, CardType::Land])
}

/// "Return target creature or land card from your graveyard to your hand": with the card gone from the graveyard the
/// ability does nothing (CR 608.2b).
fn return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = targets::object(g, item, 0) {
        g.return_to_hand(&[card]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::destroy_first_target, "I \u{2014} Destroy target nonland permanent.").targets(&[targets::NONLAND_PERMANENT]),
        chapter(2, effects::you_mill::<3>, "II \u{2014} Mill three cards."),
        chapter(3, discard_for_return, "III \u{2014} You may discard a card. When you do, return target creature or land card from your graveyard to your hand."),
    ],
    delayed: &[delayed_trigger(return_it, "When you do, return target creature or land card from your graveyard to your hand.")
        .targets(&[target(creature_or_land_card)])],
    ..oracle::AWAKEN_THE_HONORED_DEAD
};
