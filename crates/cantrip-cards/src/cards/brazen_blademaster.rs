use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{artifacts_you_control, effects};
use cantrip_oracle as oracle;

/// "...while you control two or more artifacts" (CR 109.4, 603.2): asked as the attack is declared, as part of the trigger event, and never
/// again as the ability resolves - this is no intervening "if" (CR 603.4) - so an artifact that is gone by then changes nothing.
fn you_control_two_or_more_artifacts(q: &dyn Query, asker: &Asker) -> bool {
    artifacts_you_control(q, asker.controller) >= 2
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::this_gets::<2, 1>,
        "Whenever this creature attacks while you control two or more artifacts, it gets +2/+1 until end of turn.",
    )
    .only_if(you_control_two_or_more_artifacts)],
    ..oracle::BRAZEN_BLADEMASTER
};
