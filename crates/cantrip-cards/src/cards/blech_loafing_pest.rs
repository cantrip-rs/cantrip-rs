use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_GAIN_LIFE;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each Pest, Bat, Insect, Snake, and Spider you control": every permanent its controller controls as
/// it resolves with any of the five subtypes (CR 608.2c, 205.3), each given one counter however many of the types it has, a
/// changeling among them (CR 702.73a), the subtypes read through the words of Blech's text (CR 612.2); all at once (CR 101.4).
fn counters_on_the_pests_and_company(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let kinds = [Subtype::Pest, Subtype::Bat, Subtype::Insect, Subtype::Snake, Subtype::Spider].map(|s| words.subtype(s));
    let puts: Vec<(ObjectId, CounterKind, i16)> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| kinds.iter().any(|&k| g.chars(id).has_subtype(k)))
        .map(|id| (id, CounterKind::PlusOne, 1))
        .collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_GAIN_LIFE,
        counters_on_the_pests_and_company,
        "Whenever you gain life, put a +1/+1 counter on each Pest, Bat, Insect, Snake, and Spider you control.",
    )],
    ..oracle::BLECH_LOAFING_PEST
};
