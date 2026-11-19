use cantrip_api::*;
use cantrip_mechanics::{card_types, discover, triggers, untapped_you_control};
use cantrip_oracle as oracle;

const TAP_TWO: Question = Question::new("TapPermanent", "tap two untapped artifacts and/or creatures you control to discover 3?", "", "Tap two", "Don't tap");

/// "You may tap two untapped artifacts and/or creatures you control. If you do, discover 3" (CR 118.12, 701.57a): a player who
/// has fewer than two such permanents isn't asked (CR 609.3); the two are chosen as the payment is made, and discovering follows.
/// Any untapped artifact or creature of yours counts (`untapped_you_control`): one that came under your control this turn, and
/// Caparocti itself while it is untapped (an attacker with vigilance).
fn may_tap_two_to_discover_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, Cost::tap_untapped(2, untapped_you_control::<{ card_types(&[CardType::Artifact, CardType::Creature]) }>), &TAP_TWO)? {
        discover::discover(g, item.controller, 3)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        may_tap_two_to_discover_3,
        "Whenever Caparocti Sunborn attacks, you may tap two untapped artifacts and/or creatures you control. If you do, discover 3.",
    )],
    ..oracle::CAPAROCTI_SUNBORN
};
