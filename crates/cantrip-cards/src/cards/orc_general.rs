use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}, Sacrifice another Orc or Goblin: Other Orc creatures get +1/+1 until end of turn." (CR 608.2, 611.2a).
fn other_orc_creatures_get_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(item.object == id)
                    && (q
                        .chars(id)
                        .has_subtype(q.words(item.object).subtype(Subtype::Orc))
                        && q.chars(id).is(CardType::Creature))
            })
            .collect::<Vec<ObjectId>>()
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(1, 1),
                    Duration::EndOfTurn,
                );
            }
        },
    );
    Ok(())
}

/// "{T}, Sacrifice another Orc or Goblin: Other Orc creatures get +1/+1 until end of turn." (CR 118.1).
fn other_orc_creatures_get_1_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    !(asker.source == id)
        && (q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Orc))
            || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Goblin)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP.and(sacrifice(1, other_orc_creatures_get_1_2)),
            other_orc_creatures_get_1,
            "{T}, Sacrifice another Orc or Goblin: Other Orc creatures get +1/+1 until end of turn.",
        ),
    ],
    ..oracle::ORC_GENERAL
};
