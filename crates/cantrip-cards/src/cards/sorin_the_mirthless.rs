use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "+1: Look at the top card of your library. You may reveal that card and put it into your hand. If you do, you lose life equal to its mana value." (CR 608.2, 119.3, 400.7).
fn look_top_card_library_reveal(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_7: Vec<ObjectId> = Vec::new();
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let mut hoisted_3: Vec<ObjectId> = Vec::new();
    let objects_0 = {
        let q: &dyn Query = g;
        Some(item.controller)
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::library(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .take(1)
            .collect::<Vec<ObjectId>>()
    };
    g.look_at(item.controller, &objects_0);
    g.end_sentence();
    let paid_1 = !({
        let q: &dyn Query = g;
        Some(item.controller)
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::library(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .take(1)
            .collect::<Vec<ObjectId>>()
    })
        .is_empty()
        && effects::ask_may(g, item.controller, item.object, Purpose::Optional)?;
    if paid_1 {
        let objects_2 = {
            let q: &dyn Query = g;
            Some(item.controller)
                .into_iter()
                .flat_map(|p: PlayerId| q.state().zone(Zone::library(p)).to_vec())
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .take(1)
                .collect::<Vec<ObjectId>>()
        };
        let mut owners_2: Vec<PlayerId> = objects_2
            .iter()
            .map(|o| g.state().obj(*o).owner)
            .collect();
        owners_2.dedup();
        for owner in owners_2 {
            let cards: Vec<ObjectId> = objects_2
                .iter()
                .copied()
                .filter(|o| g.state().obj(*o).owner == owner)
                .collect();
            g.reveal(owner, &cards);
        }
        g.end_sentence();
        hoisted_3.extend(objects_2.clone());
    }
    g.end_sentence();
    if (paid_1 as i64) != 0 {
        let objects_4 = {
            let q: &dyn Query = g;
            Some(item.controller)
                .into_iter()
                .flat_map(|p: PlayerId| q.state().zone(Zone::library(p)).to_vec())
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .take(1)
                .collect::<Vec<ObjectId>>()
        };
        let moved_4 = g.return_to_hand_revealed(&objects_4);
        g.end_sentence();
        let amount_5 = {
            let q: &dyn Query = g;
            objects_4
                .iter()
                .copied()
                .filter(|&o| q.state().obj(o).became.is_some())
                .collect::<Vec<ObjectId>>()
                .get(0)
                .copied()
                .map_or(0, |o: ObjectId| i64::from(q.mana_value(o))) as i32
        };
        g.lose_life(item.controller, amount_5);
        g.end_sentence();
        hoisted_6.extend(moved_4.clone());
        hoisted_7
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

pub static VAMPIRE: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet(4)),
    power: Some(2),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Lifelink]),
    ..token!(Vampire)
};

/// "−2: Create a 2/3 black Vampire creature token with flying and lifelink." (CR 608.2, 111.1).
fn create_2_3_black_vampire(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_8 = Vec::new();
    let def_8_0 = g.card_id(&VAMPIRE);
    tokens_8.extend(std::iter::repeat((def_8_0, item.controller)).take(1));
    let words_8 = g.words(item.object);
    g.create_tokens_worded(&tokens_8, false, None, words_8)?;
    Ok(())
}

/// "−7: Sorin deals 13 damage to any target. You gain 13 life." (CR 608.2, 120.3, 119.3).
fn sorin_deals_13_damage_any(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let recipients_10 = t_0.clone();
    let batch: Vec<(ObjectId, Target, u16)> = recipients_10
        .iter()
        .map(|t| (item.object, *t, 13u16))
        .collect();
    g.damage_together(&batch);
    g.end_sentence();
    g.gain_life(item.controller, 13);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                loyalty(1),
                look_top_card_library_reveal,
                "+1: Look at the top card of your library. You may reveal that card and put it into your hand. If you do, you lose life equal to its mana value.",
            )
            .sorcery_speed(),
        ability(
                loyalty(-2),
                create_2_3_black_vampire,
                "−2: Create a 2/3 black Vampire creature token with flying and lifelink.",
            )
            .sorcery_speed(),
        ability(
                loyalty(-7),
                sorin_deals_13_damage_any,
                "−7: Sorin deals 13 damage to any target. You gain 13 life.",
            )
            .targets(&[targets::ANY])
            .sorcery_speed(),
    ],
    ..oracle::SORIN_THE_MIRTHLESS
};

pub static EXTRA: &[&CardDef] = &[&VAMPIRE];
