use cantrip_api::*;
use cantrip_mechanics::{choose, explore, select, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of combat on your turn, this creature explores. Then you may have it become a copy of another creature
/// you control until end of turn." (CR 701.44, 707.2, 611.2a): it explores first, so the counter is on it when it becomes the
/// copy, and the copy keeps the counter (CR 707.4 same object). The creature is chosen as the ability resolves, not targeted
/// (CR 608.2d), among your other creatures; the "may" is the choice of none. It becomes the creature's copiable values with no
/// exception, so it has none of its own text until end of turn, this ability included. If it has left the battlefield by now
/// there is nothing to become a copy.
fn explore_then_become_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    explore(g, item.object)?;
    if !g.is_on_battlefield(item.object) {
        return Ok(());
    }
    let others = g.permanents_of(item.controller);
    let Some(&creature) = choose::select(g, item.controller, item.object, Purpose::CopyWhich, others, select::another_creature_permanents(select::up_to(1)))?.first() else {
        return Ok(());
    };
    let values = g.copiable_values(creature);
    g.become_copy(item.object, item.object, values, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::BEGINNING_OF_YOUR_COMBAT, explore_then_become_copy, "At the beginning of combat on your turn, this creature explores. Then you may have it become a copy of another creature you control until end of turn.")],
    ..oracle::DEEPFATHOM_ECHO
};
