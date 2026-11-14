use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, exile a creature you control": exiled as the cost is paid, with the spell and so
/// with the creature it becomes (CR 607.2q). Its enters trigger exiles with it too (CR 607.2a), and when it leaves the
/// battlefield, every card exiled with it - the creature that paid and the one it exiled - goes to its owner's hand.
pub static CARD: CardDef = CardDef {
    additional_cost: Some(Cost::exile_permanents(1, targets::filters::creature)),
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::exile_first_target_with_this, "When this creature enters, exile target creature an opponent controls.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            triggers::THIS_LEAVES,
            effects::return_exiled_to_hand,
            "When this creature leaves the battlefield, put each card exiled with it into its owner's hand.",
        ),
    ],
    ..oracle::FEAR_OF_ABDUCTION
};
