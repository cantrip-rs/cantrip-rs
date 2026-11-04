use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{U}{U}: For each attacking creature without flying, its controller may pay {1}. If that player doesn't, creatures you control blocking that creature gain first strike until end of turn." (CR 608.2, 611.2a, 118.1).
fn attacking_creature_without_flying_controller(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let attackers: Vec<ObjectId> = g.state().attackers.iter().map(|a| a.id)
        .filter(|&id| g.chars(id).is(CardType::Creature) && !g.chars(id).keywords.has(Keyword::Flying))
        .collect();
    let mut unpaid = Vec::new();
    for payer in g.state().apnap().collect::<Vec<_>>() {
        let mut remaining: Vec<_> = attackers.iter().copied().filter(|&a| g.controller(a) == payer).collect();
        while !remaining.is_empty() {
            let attacker = if remaining.len() == 1 {
                remaining[0]
            } else {
                cantrip_mechanics::choose::select(g, payer, item.object, Purpose::Choose, remaining.clone(),
                    cantrip_mechanics::select::Selection::new(cantrip_mechanics::select::exactly(1)).called("attacking creature to pay for next"))?[0]
            };
            remaining.retain(|&a| a != attacker);
            if !g.may_pay(payer, item.object, attacker, cost("{1}"), &PAY_QUESTION)? {
                unpaid.push(attacker);
            }
        }
    }
    g.end_sentence();
    let blockers: Vec<ObjectId> = g.state().blocks.iter()
        .filter(|b| unpaid.contains(&b.attacker) && g.controller(b.blocker) == item.controller
            && g.chars(b.blocker).is(CardType::Creature))
        .map(|b| b.blocker).collect();
    instruction(g, |g| {
        for &blocker in &blockers {
            cantrip_mechanics::effects::gain_until_end_of_turn(g, item.object, blocker, Keyword::FirstStrike);
        }
    });
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost} for {subject}?",
    "If you don't, creatures blocking it controlled by Tidal Flats' controller gain first strike",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{U}{U}"),
            attacking_creature_without_flying_controller,
            "{U}{U}: For each attacking creature without flying, its controller may pay {1}. If that player doesn't, creatures you control blocking that creature gain first strike until end of turn.",
        ),
    ],
    ..oracle::TIDAL_FLATS
};
