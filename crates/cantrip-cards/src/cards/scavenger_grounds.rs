use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{2}, {T}, Sacrifice a Desert: Exile all graveyards." (CR 608.2, 406.3).
fn exile_all_graveyards(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let objects_1 = {
        let q: &dyn Query = g;
        q.state()
            .apnap()
            .collect::<Vec<PlayerId>>()
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::graveyard(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .collect::<Vec<ObjectId>>()
    };
    g.exile_all(&objects_1);
    Ok(())
}

/// "{2}, {T}, Sacrifice a Desert: Exile all graveyards." (CR 608.2).
fn exile_all_graveyards_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Desert))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            cost("{2}").and(TAP).and(sacrifice(1, exile_all_graveyards_2)),
            exile_all_graveyards,
            "{2}, {T}, Sacrifice a Desert: Exile all graveyards.",
        ),
    ],
    ..oracle::SCAVENGER_GROUNDS
};
