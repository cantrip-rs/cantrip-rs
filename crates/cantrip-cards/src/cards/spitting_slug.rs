use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Whenever this creature blocks (CR 509.3a).
fn blocks(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Blocks { blocker, declared: true, .. } if *blocker == source => Fired::PLAIN,
        _ => None,
    }
}

/// Whenever this creature becomes blocked (CR 509.1h).
fn becomes_blocked(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::BecameBlocked { attacker } if *attacker == source => Fired::PLAIN,
        _ => None,
    }
}

/// Pay {1}{G} for first strike, or the creatures blocking or blocked by this creature gain it (CR 611.2a).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{1}{G}"), &PAY_QUESTION)?;
    g.end_sentence();
    if paid {
        effects::this_gains_first_strike(g, item)?;
    } else {
        let creatures: Vec<ObjectId> = g.state().blocks.iter().filter_map(|b| {
            if b.attacker == item.object { Some(b.blocker) }
            else if b.blocker == item.object { Some(b.attacker) }
            else { None }
        }).filter(|&id| g.chars(id).is(CardType::Creature)).collect();
        instruction(g, |g| {
            for id in creatures {
                effects::gain_until_end_of_turn(g, item.object, id, Keyword::FirstStrike);
            }
        });
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");
const TEXT: &str = "Whenever this creature blocks or becomes blocked, you may pay {1}{G}. If you do, this creature gains first strike until end of turn. Otherwise, each creature blocking or blocked by this creature gains first strike until end of turn.";
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when(EventMask::BLOCKS, blocks), resolve, TEXT),
        triggered(when(EventMask::BECAME_BLOCKED, becomes_blocked), resolve, TEXT),
    ],
    ..oracle::SPITTING_SLUG
};
