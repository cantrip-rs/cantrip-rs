use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Exile all white creatures. For each creature exiled this way, its controller draws a card." (CR 701.13a, 608.2h, 121.2c).
fn exile_all_white_creatures_creature(
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
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
    };
    g.exile_all(&objects_0);
    g.end_sentence();
    let players_1: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().apnap().collect::<Vec<PlayerId>>()
    };
    for bound_0 in players_1.iter().copied() {
        let amount_2 = {
            let q: &dyn Query = g;
            (objects_0
                .iter()
                .copied()
                .filter(|&o| q.state().obj(o).became.is_some())
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| {
                    q.chars(id).is(CardType::Creature) && bound_0 == q.controller(id)
                })
                .collect::<Vec<ObjectId>>()
                .len() as i64) as u32
        };
        let players_2 = Some(bound_0).into_iter().collect::<Vec<PlayerId>>();
        for player in players_2 {
            g.draw(player, amount_2);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], exile_all_white_creatures_creature),
    ..oracle::MARTYRS_CRY
};
