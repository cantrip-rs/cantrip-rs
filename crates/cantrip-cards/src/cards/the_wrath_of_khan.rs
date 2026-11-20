use cantrip_api::*;
use cantrip_mechanics::triggers::EACH_BEGINNING_OF_COMBAT;
use cantrip_mechanics::{each_opponent_sacrifices, effects, exile_chosen_from_hand, targets};
use cantrip_oracle as oracle;

const MAY_EXILE: Question = Question::new(
    "MayExileCardFromHand",
    "exile a card from your hand?",
    "Chapter II of The Wrath of Khan: if you do, draw a card",
    "Exile a card",
    "Don't exile",
);

/// I: "This Saga deals 4 damage to target creature an opponent controls. At the beginning of the next combat this turn, gain
/// control of that creature until end of turn. Untap it. It gains haste until end of turn." (CR 608.2b, 603.7c): the delayed
/// ability carries the creature itself, so a creature that left and came back is a different object it never gains control
/// of (CR 400.7).
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.damage(item.object, Target::Object(creature), 4);
    g.end_sentence();
    // "The next combat this turn" (CR 603.7b), about the creature (CR 603.7c): the beginning of a step is about no object,
    // so the ability carries the creature to its resolution as its subject.
    let this = item.ability.card;
    g.delayed_trigger(DelayedWhen::NextThisTurn, item.object, item.controller, AbilityRef::delayed(this, 0), Target::Object(creature));
    Ok(())
}

/// The delayed ability's effect: control until end of turn, then untap, then haste, each its own sentence (CR 611.2a).
fn steal_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        effects::gain_control_of_untap_haste(g, item, creature, Duration::EndOfTurn);
    }
    Ok(())
}

/// II: "You may exile a card from your hand. If you do, draw a card." (CR 608.2c): offered only to a player whose hand is not
/// empty, and the card is chosen only once they have said yes, which they may refuse.
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.state().zone(Zone::hand(item.controller)).is_empty() {
        return Ok(());
    }
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&MAY_EXILE))?
        && !exile_chosen_from_hand(g, item.controller, item.object, 1)?.is_empty()
    {
        g.draw(item.controller, 1);
    }
    Ok(())
}

/// III: "Each opponent sacrifices a nontoken permanent of their choice." (CR 701.21a, 101.4): every opponent chooses on the
/// board as it is now, and the permanents are sacrificed together.
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_sacrifices(g, item.controller, item.object, 1, |q, id| !q.obj(id).token)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            chapter_one,
            "I \u{2014} This Saga deals 4 damage to target creature an opponent controls. At the beginning of the next combat this turn, gain control of that creature until end of turn. Untap it. It gains haste until end of turn.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(2, chapter_two, "II \u{2014} You may exile a card from your hand. If you do, draw a card."),
        chapter(3, chapter_three, "III \u{2014} Each opponent sacrifices a nontoken permanent of their choice."),
    ],
    delayed: &[triggered(
        EACH_BEGINNING_OF_COMBAT,
        steal_it,
        "At the beginning of the next combat this turn, gain control of that creature until end of turn. Untap it. It gains haste until end of turn.",
    )],
    ..oracle::THE_WRATH_OF_KHAN
};
