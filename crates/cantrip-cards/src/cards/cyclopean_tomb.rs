use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets, your_upkeep};
use cantrip_oracle as oracle;

/// "Target non-Swamp land" (CR 115.1, 205.3i): Swamp read through the words of this artifact's text (CR 612).
fn non_swamp_land(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let swamp = q.words(asker.source).subtype(Subtype::Swamp);
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Land) && !q.chars(id).has_subtype(swamp))
}

/// "Put a mire counter on target non-Swamp land. That land is a Swamp for as long as it has a mire counter on it." (CR 122.1,
/// 305.7, 611.2b): the land type replaces the land's others and its abilities, and lasts as long as the counter, this
/// artifact gone or not (the rulings).
fn mire(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(land, CounterKind::Mire, 1);
    let swamp = g.words(item.object).subtype(Subtype::Swamp);
    g.add_effect(item.object, land, EffectKind::SetLandType(swamp), Duration::WhileCounter(CounterKind::Mire));
    Ok(())
}

/// "When this artifact is put into a graveyard from the battlefield, at the beginning of each of your upkeeps for the rest of
/// the game, ..." (CR 603.7): a delayed triggered ability whose source is this artifact as it last existed.
fn each_upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::EachStep { step: Step::Upkeep, on_turn_of: Some(item.controller) },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

/// The lands a mire counter was put onto with `tomb` but that a mire counter has not been removed from with it, oldest first:
/// the counter records whose cause is an ability of `tomb` resolving, its activated one and its delayed one alike, whether
/// `tomb` is still there or not. A land that has left the battlefield is another object (CR 400.7), and a permanent that
/// is no longer a land is not "a land".
fn mired_with(q: &dyn Query, tomb: ObjectId) -> Vec<ObjectId> {
    let with_tomb = |r: &&Record| matches!(r.cause, Cause::Ability { source, .. } if source == tomb);
    let history = q.history();
    let mut lands: Vec<ObjectId> = Vec::new();
    for r in history.select(EventQuery::kind(EventMask::COUNTERS)).filter(with_tomb) {
        if let Event::CountersPut { object, kind: CounterKind::Mire, amount, .. } = r.event
            && amount > 0
            && !lands.contains(&object)
        {
            lands.push(object);
        }
    }
    let removed = |land: ObjectId| {
        history
            .select(EventQuery::kind(EventMask::COUNTERS).object(land))
            .filter(with_tomb)
            .any(|r| matches!(r.event, Event::CountersRemoved { kind: CounterKind::Mire, amount, .. } if amount > 0))
    };
    lands.retain(|&land| q.is_on_battlefield(land) && q.chars(land).is(CardType::Land) && !removed(land));
    lands
}

/// "Remove all mire counters from a land that a mire counter was put onto with this artifact but that a mire counter has not
/// been removed from with this artifact." (CR 122.1, 608.2d): the player chooses one such land as the ability resolves, and
/// every mire counter comes off it, wherever it came from (the rulings). Nothing is asked when no land qualifies. A land that
/// qualifies has had no counter removed with this artifact, so one whose counters all went another way still qualifies, and
/// choosing it removes none.
fn unmire(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands = mired_with(g, item.object);
    let Some(&land) = choose::select(g, item.controller, item.object, Purpose::Choose, lands, select::land_cards(select::exactly(1)))?.first() else {
        return Ok(());
    };
    let n = g.obj(land).counters(CounterKind::Mire);
    if n > 0 {
        g.remove_counters(land, CounterKind::Mire, n);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}").and(TAP),
            mire,
            "{2}, {T}: Put a mire counter on target non-Swamp land. That land is a Swamp for as long as it has a mire counter on it. Activate only during your upkeep.",
        )
        .targets(&[target(non_swamp_land)])
        .condition(your_upkeep),
        triggered(
            cantrip_mechanics::triggers::THIS_DIES,
            each_upkeep,
            "When this artifact is put into a graveyard from the battlefield, at the beginning of each of your upkeeps for the rest of the game, remove all mire counters from a land that a mire counter was put onto with this artifact but that a mire counter has not been removed from with this artifact.",
        ),
    ],
    delayed: &[delayed_trigger(
        unmire,
        "At the beginning of each of your upkeeps for the rest of the game, remove all mire counters from a land that a mire counter was put onto with this artifact but that a mire counter has not been removed from with this artifact.",
    )],
    ..oracle::CYCLOPEAN_TOMB
};
