use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
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

/// "Whenever a Bat you control attacks" (CR 508.1, 603.2): each attacking creature, as it is declared, that is a Bat
/// (the type as this card's text reads it, CR 612) under this permanent's controller's control. Zoraline is one herself.
fn a_bat_you_control_attacks(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. }
            if q.controller(*attacker) == q.controller(source) && q.chars(*attacker).has_subtype(q.words(source).subtype(Subtype::Bat)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "You gain 1 life."
fn gain_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::gain_life(g, item, 1)
}

/// "You may pay {W}{B} and 2 life. When you do, ..." (CR 118.12, 603.12): one payment of the mana and the life together,
/// which a player with less than 2 life can't make (CR 119.4). Paying is what triggers the reflexive ability, whose target
/// is chosen as it is put on the stack; with none, it is removed (CR 603.3d) and the payment stays paid.
fn pay(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{W}{B}").and(life(2)), &ASK)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
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
        triggered(when(EventMask::ATTACKS, a_bat_you_control_attacks), gain_one, "Whenever a Bat you control attacks, you gain 1 life."),
        triggered(THIS_ENTERS_OR_ATTACKS, pay, "Whenever Zoraline enters or attacks, you may pay {W}{B} and 2 life. When you do, return target nonland permanent card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it."),
    ],
    delayed: &[delayed_trigger(return_it, "When you do, return target nonland permanent card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it.")
        .targets(&[targets::NONLAND_PERMANENT_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::ZORALINE_COSMOS_CALLER
};
