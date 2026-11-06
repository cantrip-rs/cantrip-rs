use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Look at a card at random in target player's hand": the look is private and moves nothing, but it is recorded in the
/// history (`Event::Looked`) as the player's. "You draw a card at the beginning of the next turn's upkeep": a delayed trigger of anyone's next
/// upkeep, set up even when the hand was empty or the target is gone (CR 609.3).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0) {
        if let Some(player) = item.target(0).player() {
            let hand = g.state().zone(Zone::hand(player)).to_vec();
            if !hand.is_empty() {
                let card = hand[g.choose_at_random(hand.len())];
                g.look_at(item.controller, &[card]);
            }
        }
        let this = g.card_id(&CARD);
        g.delayed_trigger(
            DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None },
            item.object,
            item.controller,
            AbilityRef::delayed(this, 0),
            Target::None,
        );
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP.and(SACRIFICE_THIS), effect, "{T}, Sacrifice Urza's Bauble: Look at a card at random in target player's hand. You draw a card at the beginning of the next turn's upkeep.")
        .targets(&[targets::PLAYER])],
    delayed: &[delayed_trigger(|g, item| effects::draw(g, item, 1), "Draw a card.")],
    ..oracle::URZAS_BAUBLE
};
