use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const SUSPECT_IT: Question = Question::new("SuspectIt", "suspect it?", "", "Suspect it", "Leave it");

/// "If it's not suspected" (CR 603.4): this creature is not suspected, as it attacks and again as the ability resolves. A creature
/// already suspected is unchanged by suspecting it again (CR 701.60d), so the intervening "if" is what stops the question.
fn this_is_not_suspected(q: &dyn Query, asker: &Asker) -> bool {
    !q.is_suspected(asker.source)
}

/// "You may suspect it" (CR 701.60a, 608.2d): asked of the ability's controller as it resolves, and only while this is still the
/// creature that attacked (CR 400.7).
fn may_suspect_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may_this_card(g, item, Purpose::Ask(&SUSPECT_IT))? {
        g.suspect(item.object);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        may_suspect_this,
        "Whenever this creature attacks, if it's not suspected, you may suspect it. (A suspected creature has menace and can't block.)",
    )
    .condition(this_is_not_suspected)],
    ..oracle::RUBBLEBELT_BRAGGART
};
