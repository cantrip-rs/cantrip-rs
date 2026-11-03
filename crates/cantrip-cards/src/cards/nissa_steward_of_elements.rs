use cantrip_api::*;
use cantrip_mechanics::{effects, scry, top_of_library};
use cantrip_oracle as oracle;

/// "+2: Scry 2." (CR 608.2).
fn scry_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    scry(g, item.controller, item.object, 2)?;
    Ok(())
}

/// "0: Look at the top card of your library. If it's a land card or a creature card with mana value less than or equal to the number of loyalty counters on Nissa, you may put that card onto the battlefield." (CR 608.2, 701.5a).
fn look_top_card_library_if(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    let objects_1 = {
        let q: &dyn Query = g;
        Some(item.controller)
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::library(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .take(1)
            .collect::<Vec<ObjectId>>()
    };
    g.look_at(item.controller, &objects_1);
    g.end_sentence();
    let looked_2 = top_of_library(g, item.controller, 1);
    let offered_2 = {
        let q: &dyn Query = g;
        looked_2
            .iter()
            .copied()
            .any(|id| {
                q.chars(id).is(CardType::Land)
                    || (q.chars(id).is(CardType::Creature)
                        && (i64::from(q.mana_value(id)))
                            <= (Some(item.object)
                                .into_iter()
                                .map(|o: ObjectId| i64::from(
                                    q.state().obj(o).counters(CounterKind::Loyalty),
                                ))
                                .sum::<i64>()))
            })
    };
    if offered_2 && effects::ask_may(g, item.controller, item.object, Purpose::Optional)?
    {
        let top_3 = top_of_library(g, item.controller, 1);
        let matching_3: Vec<ObjectId> = {
            let q: &dyn Query = g;
            top_3
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).is(CardType::Land)
                        || (q.chars(id).is(CardType::Creature)
                            && (i64::from(q.mana_value(id)))
                                <= (Some(item.object)
                                    .into_iter()
                                    .map(|o: ObjectId| i64::from(
                                        q.state().obj(o).counters(CounterKind::Loyalty),
                                    ))
                                    .sum::<i64>()))
                })
                .collect()
        };
        let objects_4 = matching_3
            .clone()
            .into_iter()
            .filter(|&o: &ObjectId| !g.state().obj(o).token)
            .collect::<Vec<ObjectId>>();
        let moved_4 = {
            let rows_4: Vec<(ObjectId, PlayerId, bool)> = objects_4
                .iter()
                .map(|&o| (o, g.state().obj(o).owner, false))
                .collect();
            let came_4 = g.put_all_onto_battlefield_as(&rows_4)?;
            came_4
                .into_iter()
                .filter(|&o| g.state().obj(o).zone.is(ZoneKind::Battlefield))
                .collect::<Vec<ObjectId>>()
        };
        g.end_sentence();
        hoisted_5.extend(moved_4.clone());
        hoisted_6
            .extend({
                let q: &dyn Query = g;
                objects_4
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
            });
    }
    Ok(())
}

/// "−6: Untap up to two target lands you control. They become 5/5 Elemental creatures with flying and haste until end of turn. They're still lands." (CR 115.1).
fn untap_up_two_target_lands_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Land) && asker.controller == q.controller(id)))
    )
}

/// "−6: Untap up to two target lands you control. They become 5/5 Elemental creatures with flying and haste until end of turn. They're still lands." (CR 608.2, 611.2a, 701.26a).
fn untap_up_two_target_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = announced(g, item, 0, true);
    let objects_7 = {
        let q: &dyn Query = g;
        t_0.iter()
            .filter_map(|t| t.object())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let turned_7: Vec<ObjectId> = objects_7
        .iter()
        .copied()
        .filter(|&o| g.state().obj(o).tapped != false)
        .collect();
    instruction(
        g,
        |g| {
            for &object in &turned_7 {
                g.untap(object);
            }
        },
    );
    g.end_sentence();
    let objects_8 = {
        let q: &dyn Query = g;
        t_0.iter()
            .filter_map(|t| t.object())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_8_1_0 = {
        let q: &dyn Query = g;
        EffectKind::AddSubtype(q.words(item.object).subtype(Subtype::Elemental))
    };
    let kind_8_2_0 = EffectKind::AddTypes(TypeSet::of(&[CardType::Creature]));
    let kind_8_3_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying]));
    let kind_8_4_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste]));
    instruction(
        g,
        |g| {
            for &object in &objects_8 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::SetPt(5, 5),
                    Duration::EndOfTurn,
                );
                g.add_effect(item.object, object, kind_8_1_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_8_2_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_8_3_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_8_4_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

/// "−6: Untap up to two target lands you control. They become 5/5 Elemental creatures with flying and haste until end of turn. They're still lands." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

/// "+2: Scry 2. 0: Look at the top card of your library. If it's a land card or a creature card with mana value less than or equal to the number of loyalty counters on Nissa, you may put that card onto the battlefield. −6: Untap up to two target lands you control. They become 5/5 Elemental creatures with flying and haste until end of turn. They're still lands." (CR 608.2, 701.5a).
fn enters_with_x_loyalty(
    _: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.counters[CounterKind::Loyalty as usize] += entering.x as i16;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(2), scry_2, "+2: Scry 2.").sorcery_speed(),
        ability(
                loyalty(0),
                look_top_card_library_if,
                "0: Look at the top card of your library. If it's a land card or a creature card with mana value less than or equal to the number of loyalty counters on Nissa, you may put that card onto the battlefield.",
            )
            .sorcery_speed(),
        ability(
                loyalty(-6),
                untap_up_two_target_lands,
                "−6: Untap up to two target lands you control. They become 5/5 Elemental creatures with flying and haste until end of turn. They're still lands.",
            )
            .targets(&[target(untap_up_two_target_lands_target_0).between(0, 2)])
            .sorcery_speed(),
    ],
    enters: Some(enters_with_x_loyalty),
    ..oracle::NISSA_STEWARD_OF_ELEMENTS
};
