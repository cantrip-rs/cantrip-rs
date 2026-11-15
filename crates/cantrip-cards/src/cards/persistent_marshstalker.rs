use cantrip_api::*;
use cantrip_mechanics::{hand, lands, statics, triggers, you_have_threshold};
use cantrip_oracle as oracle;

static RETURN: Question = Question::new(
    "ReturnToBattlefieldAttacking",
    "pay {cost} to return {subject} from your graveyard to the battlefield tapped and attacking?",
    "",
    "Pay {cost}",
    "Leave it",
);

/// "You may pay {2}{B}. If you do, return this card from your graveyard to the battlefield tapped and attacking" (CR 118.12,
/// 508.4): the payment is chosen as the ability resolves, and the card returns as a new object under its owner's control, attacking
/// what its controller chooses or just tapped when there is nothing to attack. Once the card is no longer the object that
/// triggered (CR 400.7) nothing is asked and nothing returns.
fn pay_to_return_tapped_and_attacking(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_live(g, item.object) && g.may_pay(item.controller, item.object, item.object, cost("{2}{B}"), &RETURN)? {
        hand::put_tapped_and_attacking(g, item.object, item.controller, item.object)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::itself,
        statics::plus_per_other_subtype_you_control::<{ lands::sub(Subtype::Rat) }, 1, 0>,
        "This creature gets +1/+0 for each other Rat you control.",
    )],
    abilities: &[triggered(
        when_asked(EventMask::ATTACKS, triggers::you_attack_with_a_creature_of_subtype::<{ lands::sub(Subtype::Rat) }>),
        pay_to_return_tapped_and_attacking,
        "Threshold — Whenever you attack with one or more Rats, if there are seven or more cards in your graveyard, you may pay {2}{B}. If you do, return this card from your graveyard to the battlefield tapped and attacking.",
    )
    .once_per_batch()
    .condition(you_have_threshold)
    // It returns this card from the graveyard, so it functions only there (CR 113.6m).
    .functions_in(IN_A_GRAVEYARD)],
    ..oracle::PERSISTENT_MARSHSTALKER
};
