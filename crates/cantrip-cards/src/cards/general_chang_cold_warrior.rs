use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each Klingon you control." The subtype is read through this ability's words
/// (CR 205.3, 612.2); all the counters are placed together (CR 101.4, 122.1, 608.2c).
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let klingon = g.words(item.object).subtype(Subtype::Klingon);
    effects::counters_on_each_of_subtype_you_control(
        g,
        item,
        klingon,
        CounterKind::PlusOne,
        1,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(sacrifice(1, targets::filters::creature)),
        rally,
        "{1}, Sacrifice a creature: Put a +1/+1 counter on each Klingon you control.",
    )],
    ..oracle::GENERAL_CHANG_COLD_WARRIOR
};
