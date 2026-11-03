use cantrip_api::*;
use cantrip_mechanics::{choose, choose_from_hand, effects};
use cantrip_oracle as oracle;

/// "Whenever a source deals damage to Rona" (CR 603.2).
fn when_source_deals_damage_rona(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { source, target, .. } = event else { return None };
    let source = *source;
    let subject = Target::Object(source);
    if matches!(* target, Target::Object(o) if asker.source == o) {
        return Fired::about(subject);
    }
    None
}

/// "Whenever a source deals damage to Rona, that source's controller exiles a card from their hand at random. If it's a land card, you may put it onto the battlefield under your control. Otherwise, you may cast it without paying its mana cost." (CR 608.2, 406.3).
fn sources_controller_exiles_card_their(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_7: Vec<ObjectId> = Vec::new();
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    let mut hoisted_4: Vec<ObjectId> = Vec::new();
    let mut pool_0 = {
        let q: &dyn Query = g;
        item.subject
            .object()
            .map(|o: ObjectId| q.controller(o))
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::hand(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .collect::<Vec<ObjectId>>()
    };
    let mut picked_0: Vec<ObjectId> = Vec::new();
    while picked_0.len() < 1 && !pool_0.is_empty() {
        let at = g.choose_at_random(pool_0.len());
        picked_0.push(pool_0.remove(at));
    }
    let objects_1 = picked_0.clone();
    let moved_1 = g.exile_all(&objects_1);
    g.end_sentence();
    if {
        let q: &dyn Query = g;
        (objects_1
            .iter()
            .copied()
            .filter(|&o| q.state().obj(o).became.is_some())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| q.chars(id).is(CardType::Land))
            .collect::<Vec<ObjectId>>()
            .len() as i64) > 0
    } {
        if effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
            let objects_3 = ({
                let q: &dyn Query = g;
                moved_1
                    .clone()
                    .get(0)
                    .copied()
                    .into_iter()
                    .filter(|&id| {
                        q.state().obj(id).became.is_none()
                            && q.state().obj(id).zone.kind() == ZoneKind::Exile
                    })
                    .collect::<Vec<ObjectId>>()
            })
                .into_iter()
                .filter(|&o: &ObjectId| !g.state().obj(o).token)
                .collect::<Vec<ObjectId>>();
            let moved_3 = {
                let rows_3: Vec<(ObjectId, PlayerId, bool)> = objects_3
                    .iter()
                    .map(|&o| (o, item.controller, false))
                    .collect();
                let came_3 = g.put_all_onto_battlefield_as(&rows_3)?;
                came_3
                    .into_iter()
                    .filter(|&o| g.state().obj(o).zone.is(ZoneKind::Battlefield))
                    .collect::<Vec<ObjectId>>()
            };
            g.end_sentence();
            hoisted_4.extend(moved_3.clone());
            hoisted_5
                .extend({
                    let q: &dyn Query = g;
                    objects_3
                        .iter()
                        .copied()
                        .filter(|&o| q.state().obj(o).became.is_some())
                        .collect::<Vec<ObjectId>>()
                });
        }
        g.end_sentence();
        hoisted_6.extend(hoisted_4.clone());
        hoisted_7.extend(hoisted_5.clone());
    } else {
        let mut pool_8: Vec<ObjectId> = {
            let q: &dyn Query = g;
            moved_1
                .clone()
                .get(0)
                .copied()
                .into_iter()
                .filter(|&o| q.state().obj(o).became.is_none())
                .collect::<Vec<ObjectId>>()
        };
        let mut left_8: u32 = 1;
        let mut played_8: Vec<ObjectId> = Vec::new();
        while left_8 > 0 {
            let mut open_8: Vec<ObjectId> = Vec::new();
            for &c in &pool_8 {
                let ways = g.cast_ways_during_resolution_by(c, item.controller);
                if ways.iter().any(|_| true) {
                    open_8.push(c);
                }
            }
            let picked_8 = if open_8.is_empty() {
                None
            } else if effects::ask_may(
                g,
                item.controller,
                open_8[0],
                Purpose::CastFree,
            )? {
                Some(open_8[0])
            } else {
                None
            };
            let Some(card_8) = picked_8 else { break };
            let ways_8: Vec<CastWay> = g
                .cast_ways_during_resolution_by(card_8, item.controller)
                .into_iter()
                .collect();
            let done_8 = {
                let way = choose::cast_way(g, item.controller, card_8, ways_8)?
                    .map(|w| w.way);
                g.cast_during_resolution_as(
                    card_8,
                    CastTerms {
                        instead: Some(ManaCost::ZERO),
                        way,
                        caster: Some(item.controller),
                        exile_instead: ExileInstead::Never,
                        ..CastTerms::default()
                    },
                )?
            };
            if done_8 {
                left_8 -= 1;
                played_8.push(g.state().obj(card_8).became.unwrap_or(card_8));
            } else {
                break;
            }
            pool_8.retain(|&c| c != card_8);
        }
        g.end_sentence();
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DAMAGE, when_source_deals_damage_rona),
            sources_controller_exiles_card_their,
            "Whenever a source deals damage to Rona, that source's controller exiles a card from their hand at random. If it's a land card, you may put it onto the battlefield under your control. Otherwise, you may cast it without paying its mana cost.",
        ),
    ],
    ..oracle::RONA_TOLARIAN_OBLITERATOR
};

/// "Whenever you cast a legendary spell" (CR 603.2).
fn when_cast_legendary_spell(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if asker.controller == *player && q.chars(id).supertypes.legendary {
        return Fired::about(subject);
    }
    None
}

/// "Whenever you cast a legendary spell, untap Rona." (CR 608.2, 701.26a, 502.3).
fn untap_rona(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let turned_0: Vec<ObjectId> = objects_0
        .iter()
        .copied()
        .filter(|&o| g.state().obj(o).tapped != false)
        .collect();
    instruction(
        g,
        |g| {
            for &object in &turned_0 {
                g.untap(object);
            }
        },
    );
    Ok(())
}

/// "{T}: Draw a card, then discard a card." (CR 608.2, 121.1, 701.9a).
fn draw_card_discard_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.end_sentence();
    let mut cards_2 = Vec::new();
    cards_2.extend(choose_from_hand(g, item.controller, item.object, 1)?);
    g.discard_together(&cards_2);
    Ok(())
}

/// "{5}{B/P}: Transform Rona. Activate only as a sorcery." (CR 608.2).
fn transform_rona_activate_only_as(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if let Some(object) = ({
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    })
        .first()
        .copied()
    {
        g.transform_by(object, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, when_cast_legendary_spell),
            untap_rona,
            "Whenever you cast a legendary spell, untap Rona.",
        ),
        ability(TAP, draw_card_discard_card, "{T}: Draw a card, then discard a card."),
        ability(
                Cost::mana(ManaCost {
                    generic: 5,
                    x: 0,
                    symbols: [0, 0, 0, 0, 0, 0],
                    hybrid: [None, None, None, None, None, None],
                    twobrid: [0, 0, 0, 0, 0],
                    phyrexian: [0, 0, 1, 0, 0],
                }),
                transform_rona_activate_only_as,
                "{5}{B/P}: Transform Rona. Activate only as a sorcery.",
            )
            .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::RONA_HERALD_OF_INVASION
};
