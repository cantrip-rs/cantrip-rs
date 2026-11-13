use cantrip_api::*;
use cantrip_mechanics::{effects, targets, targets::filters, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Discard", "discard one or more land cards?", "When you do, return up to that many nonland permanent cards from your graveyard to your hand", "Discard lands", "Don't");

/// You may discard one or more land cards, together (CR 701.9): a cost paid as the ability resolves (CR 118.12), its number
/// the payer's to choose as they pay it, and one that names land cards, so none can be put into a hidden zone unrevealed
/// (CR 701.9c). When you do (CR 603.12), the reflexive ability triggers counting them: "that many" is the number of land
/// cards discarded. A player with no land card in hand is not asked.
fn discard_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let lands = Cost { variable: Some(VariableCost::DiscardOneOrMore(filters::land)), ..Cost::NONE };
    if let Some(discarded) = g.may_pay_counted(you, item.object, item.object, lands, &ASK)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger_counted(item.object, you, AbilityRef::delayed(this, 0), Target::None, discarded);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, discard_lands, "When this creature enters, you may discard one or more land cards. When you do, return up to that many target nonland permanent cards from your graveyard to your hand.")],
    delayed: &[delayed_trigger(effects::return_each_target_to_hand, "When you do, return up to that many target nonland permanent cards from your graveyard to your hand.")
        .targets(&[targets::NONLAND_PERMANENT_CARD_IN_YOUR_GRAVEYARD.between(0, TargetSpec::COUNTED)])],
    ..oracle::LOAMCRAFTER_FAUN
};
