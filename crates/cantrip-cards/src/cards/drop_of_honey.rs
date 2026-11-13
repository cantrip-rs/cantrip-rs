use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, triggers};
use cantrip_oracle as oracle;

/// A creature can be chosen only if it has the least power and can be destroyed (CR 101.2).
fn least_destroyable(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let least = q.state().battlefield().iter().copied()
        .filter(|&id| q.chars(id).is(CardType::Creature))
        .filter_map(|id| q.chars(id).power)
        .min();
    Verdict::each(choice, |id| {
        q.chars(id).is(CardType::Creature)
            && q.chars(id).power.is_some_and(|power| Some(power) == least)
            && q.can_destroy(id)
    })
}

/// "At the beginning of your upkeep, destroy the creature with the least power. It can't be regenerated. If two or more creatures are tied for least power, you choose one of them." (CR 608.2, 701.8a).
fn destroy_creature_least_power_cant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let battlefield = g.state().battlefield().to_vec();
    let creatures = choose::select(
        g,
        item.controller,
        item.object,
        Purpose::Destroy,
        battlefield,
        select::Selection::new(select::as_many_as_can(1))
            .with(select::part(&(least_destroyable as SelectFn), 0, "creature with the least power")),
    )?;
    g.destroy_all_no_regen(&creatures);
    Ok(())
}

/// "When there are no creatures on the battlefield, sacrifice this enchantment." (CR 603.8).
fn sacrifice_enchantment_state(q: &dyn Query, _: &Asker) -> bool {
    (q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| q.chars(id).is(CardType::Creature))
        .collect::<Vec<ObjectId>>()
        .len() as i64) == 0
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            destroy_creature_least_power_cant,
            "At the beginning of your upkeep, destroy the creature with the least power. It can't be regenerated. If two or more creatures are tied for least power, you choose one of them.",
        ),
        state_triggered(
            sacrifice_enchantment_state,
            effects::sacrifice_self,
            "When there are no creatures on the battlefield, sacrifice this enchantment.",
        ),
    ],
    ..oracle::DROP_OF_HONEY
};
