use cantrip_api::*;
use cantrip_api::select::{set_part, LargestFn};
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::triggers::{deals_combat_damage_to_player, this_enters};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const EXCHANGE: Question = Question::new("Exchange", "exchange control of those creatures?", "Chosen as the ability resolves", "Exchange", "Don't");

/// "Whenever Kitsune enters or deals combat damage to a player" (CR 603.6a, 510.2).
fn enters_or_deals_combat_damage_to_a_player(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    this_enters(q, source, event).or_else(|| deals_combat_damage_to_player(q, source, event))
}

/// "Two other target creatures": any creature on the battlefield but Kitsune itself.
fn other_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && t != Target::Object(asker.source)
}

/// "Controlled by different players" (CR 601.2c): no two of the creatures chosen are under one player's control. Judged
/// again as the ability resolves (CR 608.2b).
fn different_controllers(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let controller = |c: ObjectId| q.controller(c);
    let distinct = |cards: &[ObjectId]| cards.iter().enumerate().all(|(i, &a)| cards[..i].iter().all(|&b| controller(a) != controller(b)));
    let addable = Bits::of(choice.candidates.len(), |i| {
        let with: Vec<ObjectId> = choice.selected.iter().copied().chain([choice.candidates[i]]).collect();
        !choice.selected.contains(&choice.candidates[i]) && distinct(&with)
    });
    Verdict { complete: distinct(choice.selected), addable }
}

/// A largest set controlled by different players: the first creature of each controller.
fn largest_different_controllers(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    cantrip_api::select::largest_distinct(choice, |c| q.controller(c))
}

/// "You may exchange control of two other target creatures controlled by different players" (CR 701.12a-b): the choice is
/// made as the ability resolves, and if either creature is no longer a legal target none of it happens (the ruling).
fn exchange(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Some(first), Some(second)) = (targets::object(g, item, 0), targets::object(g, item, 1)) {
        if ask_may(g, item.controller, item.object, Purpose::Ask(&EXCHANGE))? {
            g.exchange_control(item.object, first, second);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::DAMAGE.0), enters_or_deals_combat_damage_to_a_player),
        exchange,
        "Whenever Kitsune enters or deals combat damage to a player, you may exchange control of two other target creatures controlled by different players.",
    )
    .targets(&[target(other_creature).between(2, 2).together(set_part(&(different_controllers as SelectFn), &(largest_different_controllers as LargestFn), 0, "controlled by different players"))])],
    ..oracle::KITSUNE_DRAGONS_DAUGHTER
};
