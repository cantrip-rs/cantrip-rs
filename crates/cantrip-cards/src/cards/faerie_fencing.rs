use cantrip_api::*;
use cantrip_mechanics::{history, lands, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -X/-X until end of turn. That creature gets an additional -3/-3 until end of turn if you
/// controlled a Faerie as you cast this spell." Whether you did is what the board was as the spell became cast (CR
/// 601.2i), kept in its cast record: a Faerie lost since still counts, one gained since does not (the ruling), and a copy,
/// which was not cast (CR 707.10), never has it.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = item.x as i32;
    pump(g, item.object, creature, -x, -x);
    // A sentence ends (CR 608.2c): the state triggers judge what it did before the next (`Game::end_sentence`).
    g.end_sentence();
    if g.history().fact_as_cast(item.object) != 0 {
        pump(g, item.object, creature, -3, -3);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    cast_fact: Some(history::controlled_a_as_cast::<{ lands::sub(Subtype::Faerie) }>),
    ..oracle::FAERIE_FENCING
};
