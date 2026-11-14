use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw a card for each creature you control with a +1/+1 counter on it. Those creatures gain indestructible until end of
/// turn." The creatures are counted as the spell resolves (CR 608.2); the second sentence is its own instruction (CR 608.2c)
/// and gives indestructible to those same creatures (CR 611.2c, 702.12b), the opponent's creatures with counters being
/// neither counted nor given it.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let countered: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.state().obj(id).counters(CounterKind::PlusOne) > 0)
        .collect();
    effects::draw(g, item, countered.len() as u32)?;
    instruction(g, |g| {
        for creature in countered {
            effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::INSPIRING_CALL };
