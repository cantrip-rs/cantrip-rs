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
        loop {
            // A creature that left the game with a player who conceded as another was asked is no longer one to pay for
            // (CR 800.4a, 400.7).
            remaining.retain(|&a| is_live(g, a));
            let attacker = match remaining.as_slice() {
                [] => break,
                [only] => *only,
                _ => {
                    let rule = cantrip_mechanics::select::Selection::new(cantrip_mechanics::select::exactly(1)).called("attacking creature to pay for next");
                    match cantrip_mechanics::choose::select(g, payer, item.object, Purpose::Choose, remaining.clone(), rule)?.first() {
                        Some(&attacker) => attacker,
                        // Nothing was left to choose from: the rest left the game as the question was asked.
                        None => break,
                    }
                }
            };
            remaining.retain(|&a| a != attacker);
            if !g.may_pay(payer, item.object, attacker, cost("{1}"), &PAY_QUESTION)? {
                unpaid.push(attacker);
            }
        }
    }
    g.end_sentence();
    let blockers: Vec<ObjectId> = unpaid.iter().flat_map(|&a| g.creatures_blocking(a))
        .filter(|&b| g.controller(b) == item.controller && g.chars(b).is(CardType::Creature))
        .collect();
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
