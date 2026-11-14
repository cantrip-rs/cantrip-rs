use cantrip_api::*;
use cantrip_mechanics::{choose, prevention};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "Pay", "pay {cost} so this ability won't prevent {subject}'s combat damage?", "", "Pay {cost}", "Don't pay",
);

/// The white creature sacrificed to activate Heroism, with color words read from its text (CR 612.2).
fn white_creature(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).colors.has(q.words(asker.source).color(Color::White))
}

/// Each attacking red creature's controller chooses whether to pay as this resolves (CR 118.12a).
/// Players choose in APNAP order, and each chooses the order of their own choices (CR 101.4c).
/// The resulting shields are made together about those exact objects for the turn (CR 615.1, 611.2a).
fn prevent_unpaid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let red = g.words(item.object).color(Color::Red);
    let creatures: Vec<(ObjectId, PlayerId)> = g.state().attackers.iter().filter_map(|a| {
        let c = g.chars(a.id);
        (c.is(CardType::Creature) && c.colors.has(red)).then_some((a.id, g.controller(a.id)))
    }).collect();
    let players: Vec<PlayerId> = g.state().apnap().collect();
    let mut unpaid = Vec::new();
    for player in players {
        let mut remaining: Vec<ObjectId> = creatures.iter().filter_map(|&(id, who)| (who == player).then_some(id)).collect();
        while !remaining.is_empty() {
            let creature = if remaining.len() == 1 {
                remaining[0]
            } else {
                let options = remaining.iter().copied().map(Target::Object).collect();
                let Some(Target::Object(chosen)) = choose::one_target(g, player, item.object, options, "Choose the attacking creature to pay for next")? else { return Ok(()) };
                chosen
            };
            remaining.retain(|&id| id != creature);
            if !g.may_pay(player, item.object, creature, cost("{2}{R}"), &PAY)? {
                unpaid.push(creature);
            }
        }
    }
    instruction(g, |g| {
        for creature in unpaid {
            g.add_effect(item.object, creature, EffectKind::Prevention {
                card: item.ability.card, index: 0, to: Target::None, shield: Shield::All,
            }, Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(sacrifice(1, white_creature), prevent_unpaid,
        "Sacrifice a white creature: For each attacking red creature, prevent all combat damage that would be dealt by that creature this turn unless its controller pays {2}{R}.")],
    granted_preventions: &[prevention::COMBAT_DAMAGE_DEALT_BY.text(
        "For each attacking red creature, prevent all combat damage that would be dealt by that creature this turn unless its controller pays {2}{R}.")],
    ..oracle::HEROISM
};
