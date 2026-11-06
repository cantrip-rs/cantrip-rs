use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Tap all blue creatures." (CR 608.2, 701.26a).
fn tap_all_blue_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).colors.has(q.words(item.object).color(Color::Blue))
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
    };
    g.tap_together(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], tap_all_blue_creatures),
    ..oracle::RIPTIDE
};
