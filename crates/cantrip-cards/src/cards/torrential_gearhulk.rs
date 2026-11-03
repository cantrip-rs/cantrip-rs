use cantrip_api::*;
use cantrip_mechanics::{choose, effects, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, you may cast target instant card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead." (CR 115.1).
fn cast_target_instant_card_graveyard_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if (q.state().obj(id).zone.kind() == ZoneKind::Graveyard)
        && ! q.state().obj(id).token && ((q.chars(id).is(CardType::Instant) && (q.state()
        .obj(id).zone.kind() == ZoneKind::Graveyard && asker.controller == q.state()
        .obj(id).owner)))
    )
}

/// "When this creature enters, you may cast target instant card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead." (CR 608.2).
fn cast_target_instant_card_graveyard(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let mut pool_0: Vec<ObjectId> = {
        let q: &dyn Query = g;
        t_0.get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let mut left_0: u32 = 1;
    let mut played_0: Vec<ObjectId> = Vec::new();
    while left_0 > 0 {
        let mut open_0: Vec<ObjectId> = Vec::new();
        for &c in &pool_0 {
            let ways = g.cast_ways_during_resolution_by(c, item.controller);
            if ways.iter().any(|_| true) {
                open_0.push(c);
            }
        }
        let picked_0 = if open_0.is_empty() {
            None
        } else if effects::ask_may(g, item.controller, open_0[0], Purpose::CastFree)? {
            Some(open_0[0])
        } else {
            None
        };
        let Some(card_0) = picked_0 else { break };
        let ways_0: Vec<CastWay> = g
            .cast_ways_during_resolution_by(card_0, item.controller)
            .into_iter()
            .collect();
        let done_0 = {
            let way = choose::cast_way(g, item.controller, card_0, ways_0)?
                .map(|w| w.way);
            g.cast_during_resolution_as(
                card_0,
                CastTerms {
                    instead: Some(ManaCost::ZERO),
                    way,
                    caster: Some(item.controller),
                    exile_instead: ExileInstead::Spell,
                    ..CastTerms::default()
                },
            )?
        };
        if done_0 {
            left_0 -= 1;
            played_0.push(g.state().obj(card_0).became.unwrap_or(card_0));
        } else {
            break;
        }
        pool_0.retain(|&c| c != card_0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::THIS_ENTERS,
                cast_target_instant_card_graveyard,
                "When this creature enters, you may cast target instant card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.",
            )
            .targets(&[target(cast_target_instant_card_graveyard_target_0)]),
    ],
    ..oracle::TORRENTIAL_GEARHULK
};
