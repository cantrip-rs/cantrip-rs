use cantrip_api::*;
use cantrip_mechanics::tokens::{self, ROBOT_2_2};
use cantrip_mechanics::{effects, triggers::YOU_ATTACK};
use cantrip_oracle as oracle;

/// "Dyadrine enters with a number of +1/+1 counters on it equal to the amount of mana spent to cast it" (CR 614.1c, 601.2h):
/// every mana of the payment, the {X} part, the {G}{W} and any increase of the cost included, and none that was only spent to
/// keep the spell from being countered. Read from the spell it enters from, so a Dyadrine put onto the battlefield without being
/// cast, or a copy of the spell (CR 707.10) or a token copy of the permanent, had nothing spent and enters with none.
fn enters_with_counters_for_the_mana_spent(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] += i64::from(g.obj(entering.object).spent.total() as i16);
    Ok(())
}

const REMOVE: Question = Question::new(
    "RemoveCounters",
    "remove a +1/+1 counter from each of two creatures you control?",
    "If you do, you draw a card and create a 2/2 colorless Robot artifact creature token",
    "Remove the counters",
    "Don't remove them",
);

/// "Whenever you attack, you may remove a +1/+1 counter from each of two creatures you control. If you do, draw a card and
/// create a 2/2 colorless Robot artifact creature token." Two different creatures, one counter from each (CR 122.1): the
/// choice is the removal of two counters among the creatures with one, each pool holding the one counter that may come off it.
/// With fewer than two creatures that have a +1/+1 counter it can't be done, so nothing is asked. Having removed them ("if you do"), the player draws
/// and gets the Robot.
fn attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let pools: Vec<CounterPool> = g
        .permanents_of(you)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.obj(id).counters(CounterKind::PlusOne) > 0)
        .map(|object| CounterPool { object, kind: CounterKind::PlusOne, count: 1 })
        .collect();
    if pools.len() < 2 || !effects::ask_may(g, you, item.object, Purpose::Ask(&REMOVE))? {
        return Ok(());
    }
    // The core has checked the answer against the question: it is the counters taken.
    if let Answer::Counters(taken) = g.ask(you, RequestKind::RemoveCounters { source: item.object, pools, total: 2, up_to: false })? { remove_and_reward(g, you, taken)?; }
    Ok(())
}

/// The counters come off, one from each of the two creatures, and the player draws a card and gets the Robot ("if you do").
fn remove_and_reward(g: &mut dyn Game, you: PlayerId, taken: Vec<(ObjectId, CounterKind, u16)>) -> Result<(), Suspend> {
    for (object, kind, n) in taken {
        g.remove_counters(object, kind, n as i16);
    }
    g.draw(you, 1);
    tokens::create(g, you, &ROBOT_2_2, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_counters_for_the_mana_spent),
    abilities: &[triggered(YOU_ATTACK, attack, "Whenever you attack, you may remove a +1/+1 counter from each of two creatures you control. If you do, draw a card and create a 2/2 colorless Robot artifact creature token.")],
    ..oracle::DYADRINE_SYNTHESIS_AMALGAM
};

pub static EXTRA: &[&CardDef] = &[&ROBOT_2_2];
