use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

const SACRIFICE: Question = Question::new(
    "SacrificeTwoOrControllerDraws", "sacrifice two nonland, nontoken permanents? If you don't, its controller draws two cards", "", "Sacrifice them", "Don't sacrifice",
);

/// "Nonland, nontoken permanent" (CR 110.5, 205.2a, 111.1): a permanent that is not a land as the layers have it and is not a token.
fn nonland_nontoken(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    !q.chars(id).is(CardType::Land) && !q.state().obj(id).token
}

/// "Target opponent may sacrifice two nonland, nontoken permanents of their choice. If they don't, you draw two cards" (CR 608.2d, 118.12, 701.21):
/// the opponent chooses whether to sacrifice, and which two, as it resolves (CR 118.3: an opponent with fewer than two such permanents can't sacrifice, is not
/// asked, and so doesn't), and Rakdos's controller draws two cards when they don't. A target that is no longer legal stops the ability (CR 608.2b), so nobody draws.
fn sacrifice_or_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(them) = item.target(0) else { return Ok(()) };
    let sacrificed = g.may_pay(them, item.object, item.object, sacrifice(2, nonland_nontoken), &SACRIFICE)?;
    if !sacrificed {
        g.draw(item.controller, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        sacrifice_or_draw,
        "At the beginning of your end step, target opponent may sacrifice two nonland, nontoken permanents of their choice. If they don't, you draw two cards.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::RAKDOS_PATRON_OF_CHAOS
};
