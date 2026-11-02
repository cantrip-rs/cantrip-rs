use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Exile all nonland permanents that aren't legendary." (CR 608.2, 406.3).
fn exile_all_nonland_permanents_arent(
    g: &mut dyn Game,
    _: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(q.chars(id).is(CardType::Land)) && !(q.chars(id).supertypes.legendary)
            })
            .collect::<Vec<ObjectId>>()
    };
    g.exile_all(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], exile_all_nonland_permanents_arent),
    ..oracle::URZAS_RUINOUS_BLAST
};
