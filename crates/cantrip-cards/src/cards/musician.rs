use cantrip_api::*;
use cantrip_mechanics::{targets, triggers, cumulative_upkeep};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, destroy this creature unless you pay {1} for each music counter on it," (CR 608.2, 701.8a, 118.1).
fn granted_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let counters = g.state().obj(item.object).counters(CounterKind::Music) as u32;
    let paid = g.may_pay(item.controller, item.object, item.object, Cost::mana(generic(counters)), &PAY_QUESTION)?;
    g.end_sentence();
    if !paid {
        g.destroy_all(&[item.object]);
    }
    Ok(())
}

/// "{T}: Put a music counter on target creature. If it doesn't have "At the beginning of your upkeep, destroy this creature unless you pay {1} for each music counter on it," it gains that ability." (CR 608.2, 611.2a, 122.1).
fn put_music_counter_target_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::Music, 1);
    g.end_sentence();
    let ability = AbilityRef::delayed(item.ability.card, 0);
    if !g.chars(creature).abilities.contains(&Some(ability)) {
        g.add_effect(item.object, creature, EffectKind::AddAbility(ability), Duration::Indefinite);
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
    abilities: &[
        cumulative_upkeep!(
            cumulative_upkeep::Payment::Mana(mana("{1}")), "Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
        ability(
                TAP,
                put_music_counter_target_creature,
                "{T}: Put a music counter on target creature. If it doesn't have \"At the beginning of your upkeep, destroy this creature unless you pay {1} for each music counter on it,\" it gains that ability.",
            )
            .targets(&[targets::CREATURE]),
    ],
    delayed: &[
        triggered(
            triggers::YOUR_UPKEEP,
            granted_2,
            "At the beginning of your upkeep, destroy this creature unless you pay {1} for each music counter on it,",
        ),
    ],
    ..oracle::MUSICIAN
};
