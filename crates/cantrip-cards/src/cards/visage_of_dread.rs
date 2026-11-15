use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, THIS_ENTERS_OR_ATTACKS};
use cantrip_mechanics::hand::discard_chosen_artifact_or_creature_from_revealed_hand;
use cantrip_mechanics::{craft, effects, targets};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You choose an artifact or creature card from it. That player discards that card."
/// The whole hand is revealed (CR 701.20a), and nothing is asked when it holds no artifact or creature card.
fn reveal_and_take(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(opponent) = item.target(0) else { return Ok(()) };
    discard_chosen_artifact_or_creature_from_revealed_hand(g, item, opponent)?;
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS_OR_ATTACKS, effects::may_mill_two, "Whenever this creature enters or attacks, you may mill two cards.")],
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
