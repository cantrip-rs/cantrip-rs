use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL_THAT_TARGETS_A_CREATURE_YOU_CONTROL;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on Colleen Wing. Scry 1." (CR 122.1, 701.22a): the counter goes on her if she is still the object that
/// triggered (CR 400.7), then, as a second sentence (CR 608.2c), her controller scries 1 whether or not she is.
fn counter_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    g.end_sentence();
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_CAST_A_SPELL_THAT_TARGETS_A_CREATURE_YOU_CONTROL,
        counter_and_scry,
        "Whenever you cast a spell that targets a creature you control, put a +1/+1 counter on Colleen Wing. Scry 1.",
    )],
    ..oracle::COLLEEN_WING_STREET_SAMURAI
};
