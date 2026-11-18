use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "If this creature is suspected, put a +1/+1 counter on it. Otherwise, suspect it." (CR 701.60a, 701.60d): whether it is
/// suspected is judged as the ability resolves; one that has left the battlefield is neither suspected nor suspectable (CR 400.7).
fn counter_if_suspected_otherwise_suspect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.is_on_battlefield(item.object) {
        return Ok(());
    }
    if g.is_suspected(item.object) {
        effects::plus_one_counter_on_this(g, item)
    } else {
        g.suspect(item.object);
        Ok(())
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{B}"),
        counter_if_suspected_otherwise_suspect,
        "{2}{B}: If this creature is suspected, put a +1/+1 counter on it. Otherwise, suspect it.",
    )],
    ..oracle::REPEAT_OFFENDER
};
