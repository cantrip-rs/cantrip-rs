use cantrip_api::*;
use cantrip_mechanics::statics::{plus_pt, your_creatures};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever you gain life, put a growth counter on this enchantment" (CR 119.3, 122.1): one trigger for each life-gain event,
/// however much life it was.
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Growth, 1);
    Ok(())
}

/// "As long as there are five or more growth counters on this enchantment, creatures you control get +3/+3" (CR 611.3a): the
/// counters are asked again whenever characteristics are computed, so it turns on with the fifth counter.
fn your_creatures_with_five_growth_counters(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    your_creatures(q, source, affected, so_far) && q.state().obj(source).counters(CounterKind::Growth) >= 5
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_GAIN_LIFE, grow, "Whenever you gain life, put a growth counter on this enchantment.")],
    statics: &[static_ability(
        Layer::PtModify,
        your_creatures_with_five_growth_counters,
        plus_pt::<3, 3>,
        "As long as there are five or more growth counters on this enchantment, creatures you control get +3/+3.",
    )],
    ..oracle::COMFORTING_COUNSEL
};
