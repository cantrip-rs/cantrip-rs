use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Each opponent discards a card and loses 2 life. You draw a card and gain 2 life." (CR 701.9, 121.1, 119.3, 608.2c): two
/// sentences, done in turn; the second is done whether or not any opponent discarded.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_discards_a_card_and_loses_life::<2>(g, item)?;
    g.end_sentence();
    effects::draw_a_card_and_gain_life::<2>(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effect,
        "When this creature enters, each opponent discards a card and loses 2 life. You draw a card and gain 2 life.",
    )],
    // As an additional cost to cast this spell, sacrifice a creature (CR 118.8, 601.2h): exactly one.
    additional_cost: Some(Cost::sacrifice(1, targets::filters::creature)),
    ..oracle::ARBITER_OF_WOE
};
