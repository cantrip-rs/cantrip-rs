use cantrip_api::*;
use cantrip_mechanics::{hand::choose_from_revealed_hand_of_kind, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand and you choose a nonland card from it. Exile that card." Looking at the hand is
/// part of the ability, so the choice is asked even with nothing eligible. The card is remembered as exiled with this
/// creature (CR 607.2a) and is not tied to its staying on the battlefield: the second ability brings it back.
fn reveal_and_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    let chosen = choose_from_revealed_hand_of_kind(g, item, opponent, Purpose::Exile, "nonland", |q, c| !q.chars(c).is(CardType::Land))?;
    if !chosen.is_empty() {
        g.exile_with(&chosen, g.linked(item), false);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, reveal_and_exile, "When this creature enters, target opponent reveals their hand and you choose a nonland card from it. Exile that card.")
            .targets(&[targets::OPPONENT]),
        triggered(cantrip_mechanics::triggers::THIS_LEAVES, cantrip_mechanics::effects::return_exiled_to_hand, "When this creature leaves the battlefield, return the exiled card to its owner's hand."),
    ],
    ..oracle::TIDEHOLLOW_SCULLER
};
