use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Destroy all white permanents." (CR 608.2, 701.8a).
fn destroy_all_white_permanents(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).colors.has(q.words(item.object).color(Color::White))
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], destroy_all_white_permanents),
    ..oracle::ANARCHY
};
