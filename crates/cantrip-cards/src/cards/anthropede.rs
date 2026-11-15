use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const DISCARD: u8 = 0;
const PAY: u8 = 1;
const NEITHER: u8 = 2;

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");

/// "When this creature enters, you may discard a card or pay {2}. When you do, destroy target Room." (CR 603.12, 608.2c-d,
/// 701.9, 118.1): the controller chooses as the ability resolves whether to discard a card, pay {2} or do neither; a discard with
/// an empty hand is not offered. Only when the discard or the payment really happened is the reflexive ability created.
fn discard_or_pay(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let hand = g.state().zone(Zone::hand(you)).to_vec();
    let mut options = Vec::new();
    if !hand.is_empty() {
        options.push(DISCARD);
    }
    options.push(PAY);
    options.push(NEITHER);
    let this = g.card_id(&CARD);
    let labels = options.iter().map(|&i| Label::Card(this, i)).collect();
    let done = match options[choose::one_of(g, you, item.object, labels)? as usize] {
        DISCARD => {
            let chosen = choose::select(g, you, item.object, Purpose::Discard, hand, select::CARD.taking(select::exactly(1)))?;
            !g.discard_together(&chosen).is_empty()
        }
        PAY => g.may_pay(you, item.object, item.object, cost("{2}"), &PAY_QUESTION)?,
        _ => false,
    };
    if done {
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

fn room_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Room])
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, discard_or_pay, "When this creature enters, you may discard a card or pay {2}. When you do, destroy target Room.")],
    delayed: &[delayed_trigger(effects::destroy_first_target, "When you do, destroy target Room.").targets(&[target(room_filter)])],
    labels: &["Discard a card", "Pay {2}", "Neither"],
    ..oracle::ANTHROPEDE
};
