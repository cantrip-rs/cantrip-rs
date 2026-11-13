use cantrip_api::*;
use cantrip_mechanics::{choose, class, effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// Level 2: "Whenever you sacrifice a permanent" (CR 701.21a), once the Class is level 2 (CR 716.2a): one trigger for each
/// permanent, Scavenger's Talent itself included (read as it last was, level and all, CR 603.10a).
fn you_sacrifice_a_permanent_at_level_two(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::sacrificed_by(q, me.controller, event)?;
    if class::level(q, me.source) >= 2 { Fired::PLAIN } else { None }
}

/// "At the beginning of your end step", once the Class is level 3 (CR 716.2a).
fn your_end_step_at_level_three(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if class::level(q, me.source) >= 3 { triggers::your_end_step(q, me, event) } else { None }
}

const ASK: Question = Question::new(
    "SacrificeThreeToReturn",
    "sacrifice three other nonland permanents?",
    "If you do, return a creature card from your graveyard to the battlefield with a finality counter on it",
    "Sacrifice three",
    "Don't sacrifice",
);

/// Level 3: "you may sacrifice three other nonland permanents. If you do, return a creature card from your graveyard to the
/// battlefield with a finality counter on it" (CR 701.21a, 122.6, 614.1c). Not offered without three other nonland permanents
/// that can be sacrificed (CR 101.2: one that can't is no option). The three are sacrificed together, and the creature card is chosen afterwards, from the graveyard as it now is:
/// one of the sacrificed creatures may be the card (the ruling). It enters with the finality counter, so it is exiled instead of
/// dying (CR 122.1h).
fn sacrifice_three_to_reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.controller;
    let eligible: Vec<ObjectId> =
        g.permanents_of(me).into_iter().filter(|&p| p != item.object && !g.chars(p).is(CardType::Land) && g.can_sacrifice(me, p)).collect();
    if eligible.len() < 3 || !effects::ask_may(g, me, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let chosen = if eligible.len() == 3 { eligible } else { choose::cards(g, me, item.object, Purpose::Sacrifice, eligible, 3, false)? };
    g.sacrifice(me, &chosen);
    let creatures: Vec<ObjectId> =
        g.state().zone(Zone::graveyard(me)).iter().copied().filter(|&c| g.chars(c).is(CardType::Creature)).collect();
    if let Some(card) = choose::card(g, me, item.object, Purpose::PutOntoBattlefield, creatures)? {
        g.put_onto_battlefield_with_counters(card, me, false, &[(CounterKind::Finality, 1)])?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // Level 1 (CR 603.2c, 700.4, 603.10a): the creatures are judged as they last were on the battlefield; creatures that die
        // together are one batch and trigger it once, and it triggers no more than once in a turn.
        triggered(
            triggers::CREATURE_YOU_CONTROL_DIES,
            tokens::create_a_food_token,
            "Whenever one or more creatures you control die, create a Food token. This ability triggers only once each turn.",
        )
        .once_per_batch()
        .triggers_each_turn(1),
        class::level_two("{1}{B}", "{1}{B}: Level 2"),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, you_sacrifice_a_permanent_at_level_two),
            effects::target_player_mills::<2>,
            "Whenever you sacrifice a permanent, target player mills two cards.",
        )
        .on_sacrifice()
        .targets(&[targets::PLAYER]),
        class::level_three("{2}{B}", "{2}{B}: Level 3"),
        triggered(
            when_asked(EventMask::STEP_BEGAN, your_end_step_at_level_three),
            sacrifice_three_to_reanimate,
            "At the beginning of your end step, you may sacrifice three other nonland permanents. If you do, return a creature card from your graveyard to the battlefield with a finality counter on it.",
        ),
    ],
    ..oracle::SCAVENGERS_TALENT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
