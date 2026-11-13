use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, targets};
use cantrip_oracle as oracle;

/// "{1}{U}: Return target white or black creature card from your graveyard to the battlefield. That creature gains "Cumulative upkeep {2}." If the creature would leave the battlefield, exile it instead of putting it anywhere else." (CR 115.1).
fn return_target_white_black_creature_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    let Target::Object(id) = t else { return false };
    let words = q.words(asker.source);
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature])
        && (q.chars(id).colors.has(words.color(Color::White))
            || q.chars(id).colors.has(words.color(Color::Black)))
}

/// "{1}{U}: Return target white or black creature card from your graveyard to the battlefield. That creature gains "Cumulative upkeep {2}." If the creature would leave the battlefield, exile it instead of putting it anywhere else." (CR 608.2, 611.2a, 701.13a).
fn return_target_white_black_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    let moved_0 = {
        let rows_0: Vec<(ObjectId, PlayerId, bool)> = objects_0
            .iter()
            .map(|&o| (o, item.controller, false))
            .collect();
        let came_0 = g.put_all_onto_battlefield_as(&rows_0)?;
        came_0
            .into_iter()
            .filter(|&o| g.state().obj(o).zone.is(ZoneKind::Battlefield))
            .collect::<Vec<ObjectId>>()
    };
    g.end_sentence();
    let objects_1 = moved_0
        .clone()
        .get(0)
        .copied()
        .into_iter()
        .collect::<Vec<ObjectId>>();
    let kind_1_0_0 = EffectKind::AddAbility(AbilityRef::delayed(g.card_id(&CARD), 0));
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                g.add_effect(item.object, object, kind_1_0_0, Duration::Indefinite);
            }
        },
    );
    g.end_sentence();
    let Some(object) = moved_0.clone().get(0).copied() else { return Ok(()) };
    g.exile_on_leave(object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{U}"),
                return_target_white_black_creature,
                "{1}{U}: Return target white or black creature card from your graveyard to the battlefield. That creature gains \"Cumulative upkeep {2}.\" If the creature would leave the battlefield, exile it instead of putting it anywhere else.",
            )
            .targets(&[target(return_target_white_black_creature_target_0)]),
    ],
    delayed: &[
        cumulative_upkeep!(
            cantrip_mechanics::cumulative_upkeep::Payment::Mana(ManaCost { generic : 2, x
            : 0, symbols : [0, 0, 0, 0, 0, 0], hybrid : [None, None, None, None, None,
            None], twobrid : [0, 0, 0, 0, 0], phyrexian : [0, 0, 0, 0, 0], snow : 0,
            ..ManaCost::ZERO }), "Cumulative upkeep"
        ),
    ],
    ..oracle::DREAMS_OF_THE_DEAD
};
