use cantrip_api::*;
use cantrip_mechanics::triggers::{you_attack_with_a_creature_of_subtype, THIS_ENTERS_OR_ATTACKS};
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

/// What the payment asks: what paying leads to, the card chosen after (CR 603.12), at most mana value 3 (`amount`, for
/// the bots).
const ASK: Question = Question::new(
    "ReturnPermanentCard",
    "pay {cost} to return a nonland permanent card with mana value 3 or less from your graveyard to the battlefield?",
    "It returns with a finality counter on it  ·  the card is chosen once you have paid",
    "Pay {cost}",
    "Don't pay",
)
.amount(3);

/// "You gain 1 life."
fn gain_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 1)
}

/// "Return [it] from your graveyard to the battlefield with a finality counter on it" (CR 122.6, 614.1c): it enters with the
/// counter, under the control of the ability's controller. A target gone by then leaves the ability nothing to do, and it
/// does not resolve (CR 608.2b).
fn return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for card in targets::legal_objects(g, item) {
        g.put_onto_battlefield_with_counters(card, item.controller, false, &[(CounterKind::Finality, 1)])?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when_asked(EventMask::ATTACKS, you_attack_with_a_creature_of_subtype::<{ lands::sub(Subtype::Bat) }>), gain_one, "Whenever a Bat you control attacks, you gain 1 life."),
        triggered(THIS_ENTERS_OR_ATTACKS, |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{W}{B}").and(life(2)), &ASK), "Whenever Zoraline enters or attacks, you may pay {W}{B} and 2 life. When you do, return target nonland permanent card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it."),
    ],
    delayed: &[delayed_trigger(return_it, "When you do, return target nonland permanent card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it.")
        .targets(&[targets::NONLAND_PERMANENT_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::ZORALINE_COSMOS_CALLER
};
