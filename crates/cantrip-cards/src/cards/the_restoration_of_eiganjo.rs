use cantrip_api::*;
use cantrip_mechanics::{choose, library, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks or blocks, create a 1/1 colorless Spirit creature token." (CR 608.2).
fn create_1_1_colorless_spirit(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&cantrip_mechanics::tokens::SPIRIT_1_1_COLORLESS);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(1));
    let words_0 = g.words(item.object);
    g.create_tokens_worded(&tokens_0, false, None, words_0)?;
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS_OR_BLOCKS,
            create_1_1_colorless_spirit,
            "Whenever this creature attacks or blocks, create a 1/1 colorless Spirit creature token.",
        ),
    ],
    ..oracle::ARCHITECT_OF_RESTORATION
};

/// "I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle." (CR 608.2).
fn i_search_library_basic_plains(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    library::find_to_hand(
        g,
        item.controller,
        item.object,
        select::BASIC_PLAINS_CARD,
        true,
    )?;
    Ok(())
}

/// "I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle. II — You may discard a card. When you do, return target permanent card with mana value 2 or less from your graveyard to the battlefield tapped. III — Exile this Saga, then return it to the battlefield transformed under your control." (CR 115.1).
fn reflexive_2_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard_with_mana_value_at_most(
        q,
        asker,
        t,
        &targets::PERMANENT_CARD_TYPES,
        2,
    )
}

/// "I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle. II — You may discard a card. When you do, return target permanent card with mana value 2 or less from your graveyard to the battlefield tapped. III — Exile this Saga, then return it to the battlefield transformed under your control." (CR 608.2).
fn reflexive_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_3 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    let rows_3: Vec<(ObjectId, PlayerId, bool)> = objects_3
        .iter()
        .map(|&o| (o, item.controller, true))
        .collect();
    g.put_all_onto_battlefield_as(&rows_3)?;
    Ok(())
}

/// "II — You may discard a card. When you do, return target permanent card with mana value 2 or less from your graveyard to the battlefield tapped." (CR 608.2, 701.9a).
fn ii_discard_card_when_do(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    let mut hoisted_4: Vec<ObjectId> = Vec::new();
    {
        let mut cards_0 = Vec::new();
        cards_0
            .extend({
                let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
                choose::select(
                    g,
                    item.controller,
                    item.object,
                    Purpose::Discard,
                    hand,
                    select::Selection::new(select::Count::UpTo(1)),
                )?
            });
        let discarded_0 = g.discard_together(&cards_0);
        if !cards_0.is_empty() {
            if !discarded_0.clone().is_empty() {
                g.reflexive_trigger(
                    item.object,
                    item.controller,
                    AbilityRef::delayed(item.ability.card, 0),
                    Target::None,
                );
            }
            g.end_sentence();
            hoisted_4.extend(discarded_0.clone());
            hoisted_5.extend(discarded_0.clone());
        }
    }
    Ok(())
}

/// "III — Exile this Saga, then return it to the battlefield transformed under your control." (CR 608.2, 406.3).
fn iii_exile_saga_return_battlefield(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_6 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let moved_6 = g.exile_all(&objects_6);
    g.end_sentence();
    let objects_7 = ({
        let q: &dyn Query = g;
        moved_6
            .clone()
            .get(0)
            .copied()
            .into_iter()
            .filter(|&id| q.state().obj(id).zone.kind() == ZoneKind::Exile)
            .collect::<Vec<ObjectId>>()
    })
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    {
        let mut moved: Vec<ObjectId> = Vec::new();
        for &o in &objects_7 {
            let entered = g.put_onto_battlefield_transformed(o, item.controller, false)?;
            if g.is_on_battlefield(entered) {
                moved.push(entered);
            }
        }
    };
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            i_search_library_basic_plains,
            "I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle.",
        ),
        chapter(
            2,
            ii_discard_card_when_do,
            "II — You may discard a card. When you do, return target permanent card with mana value 2 or less from your graveyard to the battlefield tapped.",
        ),
        chapter(
            3,
            iii_exile_saga_return_battlefield,
            "III — Exile this Saga, then return it to the battlefield transformed under your control.",
        ),
    ],
    delayed: &[
        delayed_trigger(
                reflexive_2,
                "When you do, return target permanent card with mana value 2 or less from your graveyard to the battlefield tapped.",
            )
            .targets(&[target(reflexive_2_target_0)]),
    ],
    back: Some(&BACK),
    ..oracle::THE_RESTORATION_OF_EIGANJO
};

pub static EXTRA: &[&CardDef] = &[&cantrip_mechanics::tokens::SPIRIT_1_1_COLORLESS];
