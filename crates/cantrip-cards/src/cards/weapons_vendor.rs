use cantrip_api::*;
use cantrip_mechanics::attach::attach_first_target_to_second;
use cantrip_mechanics::triggers::{BEGINNING_OF_YOUR_COMBAT, THIS_ENTERS};
use cantrip_mechanics::{effects, permanents_with_subtype_you_control, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to attach target Equipment you control to target creature you control?", "", "Pay {cost}", "Don't pay");

/// "If you control an Equipment" (CR 603.4): asked as the ability triggers and again as it resolves.
fn you_control_an_equipment(q: &dyn Query, asker: &Asker) -> bool {
    permanents_with_subtype_you_control(q, asker.controller, Subtype::Equipment) >= 1
}

/// "You may pay {1}. When you do, attach target Equipment you control to target creature you control." (CR 603.12): the targets
/// are chosen only if the payment is made, as the reflexive ability goes on the stack.
fn pay_to_attach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this creature enters, draw a card."),
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            pay_to_attach,
            "At the beginning of combat on your turn, if you control an Equipment, you may pay {1}. When you do, attach target Equipment you control to target creature you control.",
        )
        .condition(you_control_an_equipment),
    ],
    delayed: &[delayed_trigger(attach_first_target_to_second, "When you do, attach target Equipment you control to target creature you control.").targets(&[
        targets::EQUIPMENT_YOU_CONTROL.labelled("Equipment to attach"),
        targets::CREATURE_YOU_CONTROL.labelled("Creature to attach it to"),
    ])],
    ..oracle::WEAPONS_VENDOR
};
