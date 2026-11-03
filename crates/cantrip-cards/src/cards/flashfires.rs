use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Destroy all Plains." (CR 608.2, 701.8a).
fn destroy_all_plains(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Plains))
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], destroy_all_plains),
    ..oracle::FLASHFIRES
};
