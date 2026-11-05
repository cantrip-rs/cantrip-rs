use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature enters with a +1/+1 counter on it for each other creature and/or artifact you control" (CR 614.1c, 122.6):
/// counted as it enters, among the permanents of the player it enters under. It is not on the battlefield yet, so it never
/// counts itself, and a permanent entering with it is not counted either (its ruling). A permanent that is both a creature and
/// an artifact is one permanent, and counts once.
fn enters_with_counters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let counted = g
        .permanents_of(entering.controller)
        .into_iter()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Creature) || chars.is(CardType::Artifact)
        })
        .count();
    entering.counters[CounterKind::PlusOne as usize] += i64::try_from(counted).expect("battlefield count fits i64");
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_counters),
    ..oracle::LUXKNIGHT_BREACHER
};
