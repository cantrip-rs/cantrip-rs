use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets, triggers};
use cantrip_oracle as oracle;

/// Chapter II: "Mill four cards. When you do, return target creature card from your graveyard to your hand." The reflexive
/// trigger exists only if the cards were milled (CR 603.12), and chooses its target after milling.
fn mill_and_reflex(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = g.card_id(&CARD);
    library::mill_then_reflexive(g, item, 4, AbilityRef::delayed(this, 0));
    Ok(())
}

/// "Return target creature card from your graveyard to your hand": a target gone by resolution is left alone (CR 608.2b).
fn return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.return_to_hand(&[card]);
    Ok(())
}

/// Chapter III: "Whenever a creature you control attacks alone this turn" is a delayed triggered ability (CR 603.7b), made
/// when the chapter resolves and lasting the turn; it is the card's second delayed ability.
fn attacks_alone_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = g.card_id(&CARD);
    g.delayed_trigger(DelayedWhen::ThisTurn, item.object, item.controller, AbilityRef::delayed(this, 1), Target::None);
    Ok(())
}

/// "Put three +1/+1 counters on it. It gains trample, lifelink, and indestructible until end of turn": "it" is the lone attacker
/// the trigger is about.
fn counters_and_keywords(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 3);
    // "Gains trample, lifelink, and indestructible" is one instruction.
    g.one_instruction(&mut |g| {
        cantrip_mechanics::trample_until_end_of_turn(g, item.object, creature);
        effects::gain_lifelink_until_end_of_turn(g, item.object, creature);
        effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::destroy_all_creatures, "I \u{2014} Destroy all creatures."),
        chapter(2, mill_and_reflex, "II \u{2014} Mill four cards. When you do, return target creature card from your graveyard to your hand."),
        chapter(
            3,
            attacks_alone_this_turn,
            "III \u{2014} Whenever a creature you control attacks alone this turn, put three +1/+1 counters on it. It gains trample, lifelink, and indestructible until end of turn.",
        ),
    ],
    delayed: &[
        delayed_trigger(return_it, "When you do, return target creature card from your graveyard to your hand.")
            .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
        triggered(
            triggers::CREATURE_YOU_CONTROL_ATTACKS_ALONE,
            counters_and_keywords,
            "Whenever a creature you control attacks alone this turn, put three +1/+1 counters on it. It gains trample, lifelink, and indestructible until end of turn.",
        ),
    ],
    ..oracle::THE_LAST_RONIN
};
