use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "Exile target creature. That creature's controller reveals cards from the top of their library until they reveal a creature card. That player puts that card onto the battlefield, then shuffles the rest into their library." (CR 608.2, 406.3, 701.24a).
fn exile_target_creature_creatures_controller(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .collect::<Vec<_>>();
    g.exile_all(&objects_0);
    g.end_sentence();
    let owner_1 = {
        let q: &dyn Query = g;
        let Some(named_1) = t_0
            .get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .map(|o: ObjectId| q.controller(o)) else { return Ok(()) };
        named_1
    };
    let (found_1, rest_1) = library::reveal_until_n(
        g,
        owner_1,
        1,
        &|q: &dyn Query, id: ObjectId| q.chars(id).is(CardType::Creature),
    );
    let shown_1 = found_1.len() + rest_1.len();
    let objects_1: Vec<ObjectId> = g
        .state()
        .zone(Zone::library(owner_1))
        .iter()
        .copied()
        .take(shown_1)
        .collect();
    let player_2 = {
        let q: &dyn Query = g;
        let Some(named_1) = t_0
            .get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .map(|o: ObjectId| q.controller(o)) else { return Ok(()) };
        named_1
    };
    let objects_2 = ({
        let q: &dyn Query = g;
        objects_1
            .clone()
            .into_iter()
            .filter(|&id| q.chars(id).is(CardType::Creature))
            .collect::<Vec<ObjectId>>()
    })
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    {
        let rows_2: Vec<(ObjectId, PlayerId, bool, PlayerId)> = objects_2
            .iter()
            .map(|&o| (o, player_2, false, player_2))
            .collect();
        g.put_all_onto_battlefield_by(&rows_2, &[])?;
    };
    for player in {
        let q: &dyn Query = g;
        t_0.get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .map(|o: ObjectId| q.controller(o))
            .into_iter()
            .collect::<Vec<PlayerId>>()
    } {
        g.shuffle(player);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE],
        exile_target_creature_creatures_controller,
    ),
    ..oracle::TRANSMOGRIFY
};
