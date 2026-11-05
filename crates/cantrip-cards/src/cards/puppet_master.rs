use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// Record both the enchanted creature and the graveyard object it became (CR 603.10a, 400.7).
fn when_enchanted_creature_dies(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let (old, new) = triggers::enchanted_creature_died(q, asker.source, event)?;
    Some(Fired { subject: Target::Object(old), data: [new.0, old.0] })
}

/// Return the dead card, then offer to recover the Aura only if that card reached its owner's hand.
fn return_card_owners_hand_if(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = g.found_all_after_move(ObjectId(item.data[1]));
    let moved = g.return_to_hand(&cards);
    g.end_sentence();
    if moved.iter().any(|&id| g.obj(id).zone == Zone::hand(g.obj(id).owner)) {
        let paid = g.may_pay(item.controller, item.object, item.object, cost("{U}{U}{U}"), &PAY_QUESTION)?;
        g.end_sentence();
        if paid {
            if let Some(aura) = g.found_aura_after_enchanted_left(item.object, ObjectId(item.data[1])) {
                g.return_to_hand(&[aura]);
            }
        }
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, when_enchanted_creature_dies),
        return_card_owners_hand_if,
        "When enchanted creature dies, return that card to its owner's hand. If that card is returned to its owner's hand this way, you may pay {U}{U}{U}. If you do, return this card to its owner's hand.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::PUPPET_MASTER
};
