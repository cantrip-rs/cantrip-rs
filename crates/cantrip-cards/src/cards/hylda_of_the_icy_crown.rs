use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_TAP_AN_UNTAPPED_CREATURE_AN_OPPONENT_CONTROLS;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to choose a mode?", "", "Pay {cost}", "Don't pay");

/// "Create a 4/4 white and blue Elemental creature token." (CR 111.1, 701.7a)
fn create_elemental(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ELEMENTAL_4_4_WHITE_BLUE, 1)?;
    Ok(())
}

/// "Choose one" (CR 700.2), chosen as the reflexive ability goes on the stack (CR 603.12, 603.3c).
static MODAL: ModalDef = modes(&[
    mode(create_elemental, "Create a 4/4 white and blue Elemental creature token."),
    mode(effects::plus_one_counter_on_each_creature_you_control, "Put a +1/+1 counter on each creature you control."),
    mode(effects::scry_then_draw::<2>, "Scry 2, then draw a card."),
]);

/// "You may pay {1}. When you do, choose one" (CR 118.12, 603.12): each creature tapped is its own trigger (CR 603.2c) and its own
/// payment; declining pays nothing and triggers nothing.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_TAP_AN_UNTAPPED_CREATURE_AN_OPPONENT_CONTROLS,
        |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{1}"), &PAY),
        "Whenever you tap an untapped creature an opponent controls, you may pay {1}. When you do, choose one \u{2014} Create a 4/4 white and blue Elemental creature token; or Put a +1/+1 counter on each creature you control; or Scry 2, then draw a card.",
    )],
    delayed: &[modal_delayed_trigger(
        &MODAL,
        "When you do, choose one \u{2014}\n\u{2022} Create a 4/4 white and blue Elemental creature token.\n\u{2022} Put a +1/+1 counter on each creature you control.\n\u{2022} Scry 2, then draw a card.",
    )],
    ..oracle::HYLDA_OF_THE_ICY_CROWN
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELEMENTAL_4_4_WHITE_BLUE];
