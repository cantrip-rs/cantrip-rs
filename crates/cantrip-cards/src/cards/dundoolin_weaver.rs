use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "If you control three or more creatures" (CR 603.4): checked as the ability would trigger, with the Weaver itself among
/// the creatures, and again as it resolves, off the board as it stands: the creatures need not be the same ones.
fn control_three_or_more_creatures(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().filter(|&id| q.chars(id).is(CardType::Creature)).count() >= 3
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::return_first_target_to_hand,
        "When this creature enters, if you control three or more creatures, return target permanent card from your graveyard to your hand.",
    )
    .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD])
    .condition(control_three_or_more_creatures)],
    ..oracle::DUNDOOLIN_WEAVER
};
