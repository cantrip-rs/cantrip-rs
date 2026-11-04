use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, THIS_ENTERS_OR_ATTACKS};
use cantrip_mechanics::hand::discard_chosen_artifact_or_creature_from_revealed_hand;
use cantrip_mechanics::{craft, effects, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Mill", "mill two cards?", "The top two cards of your library go into your graveyard", "Mill two", "Don't mill");

/// "Target opponent reveals their hand. You choose an artifact or creature card from it. That player discards that card."
/// The whole hand is revealed (CR 701.20a), and nothing is asked when it holds no artifact or creature card.
fn reveal_and_take(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(opponent) = item.target(0) else { return Ok(()) };
    discard_chosen_artifact_or_creature_from_revealed_hand(g, item, opponent)?;
    Ok(())
}

/// "Whenever this creature enters or attacks, you may mill two cards." With fewer than two cards in the library the
/// player can't choose to (the card's ruling), so nothing is asked.
fn may_mill_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.state().zone(Zone::library(item.controller)).len() < 2 {
        return Ok(());
    }
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.mill(item.controller, 2);
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS_OR_ATTACKS, may_mill_two, "Whenever this creature enters or attacks, you may mill two cards.")],
    ..oracle::DREAD_OSSEOSAUR
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, reveal_and_take, "When this artifact enters, target opponent reveals their hand. You choose an artifact or creature card from it. That player discards that card.")
            .targets(&[targets::OPPONENT]),
        craft::ability(mana("{5}{B}"), &craft::TWO_CREATURES, "Craft with two creatures {5}{B}"),
    ],
    back: Some(&BACK),
    ..oracle::VISAGE_OF_DREAD
};
