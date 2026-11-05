use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each artifact creature and/or Vehicle you control" (CR 122.1, 608.2, 101.4): every artifact creature
/// and every Vehicle (crewed into a creature or not, CR 301.7) its controller controls as it resolves, all at once.
fn counter_on_each_artifact_creature_and_or_vehicle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let puts: Vec<(ObjectId, CounterKind, i64)> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| {
            let c = g.chars(id);
            (c.is(CardType::Artifact) && c.is(CardType::Creature)) || c.has_subtype(Subtype::Vehicle)
        })
        .map(|id| (id, CounterKind::PlusOne, 1))
        .collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, counter_on_each_artifact_creature_and_or_vehicle, "{T}: Put a +1/+1 counter on each artifact creature and/or Vehicle you control."),
        ability(
            cost("{2}").and(remove_kind_counters_among(CounterKind::PlusOne, 2, targets::filters::artifact)),
            |g, item| effects::draw(g, item, 1),
            "{2}, Remove two +1/+1 counters from among artifacts you control: Draw a card.",
        ),
    ],
    ..oracle::IRON_SPIDER_STARK_UPGRADE
};
