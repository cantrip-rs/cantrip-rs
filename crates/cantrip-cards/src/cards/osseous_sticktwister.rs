use cantrip_api::*;
use cantrip_mechanics::targets::filters::nonland;
use cantrip_mechanics::select::{self, exactly, Selection};
use cantrip_mechanics::{choose, delirium, fight::damage_equal_to_power, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

/// The choices of each opponent, as `labels` below.
const SACRIFICE: u8 = 0;
const DISCARD: u8 = 1;
const NEITHER: u8 = 2;

/// "A nonland permanent" (CR 701.21a), chosen as the ability resolves from the opponent's permanents the caller lists.
const NONLAND_PERMANENT: Selection = Selection::of(exactly(1), &[select::NONLAND]).called("nonland permanent");

/// An intervening "if" (CR 603.4): delirium.
fn has_delirium(q: &dyn Query, asker: &Asker) -> bool {
    delirium(q, asker.controller)
}

/// "Each opponent may sacrifice a nonland permanent of their choice or discard a card. Then this creature deals damage equal
/// to its power to each opponent who didn't sacrifice a permanent or discard a card this way."
///
/// Each opponent in turn order (CR 101.4) chooses - to sacrifice a nonland permanent, to discard a card, or neither, an
/// answer offered only for what they have - and then every sacrifice and every discard happens together (the rulings,
/// `Game::together`). The damage comes after, from the creature's power as it last existed if it has left (CR 608.2h).
fn each_opponent_may_sacrifice_or_discard_else_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&CARD);
    let (mut sacrificed, mut discarded, mut neither) = (Vec::new(), Vec::new(), Vec::new());
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| g.state().opponents(item.controller).any(|o| o == p)).collect();
    for opponent in opponents {
        if !cantrip_mechanics::players::reached(g, opponent) {
            continue;
        }
        let asker = g.item_asker(item, opponent);
        let permanents: Vec<ObjectId> = g.permanents_of(opponent).into_iter().filter(|&id| nonland(g, &asker, id) && g.can_sacrifice(opponent, id)).collect();
        let hand = g.state().zone(Zone::hand(opponent)).to_vec();
        let mut options = Vec::new();
        if !permanents.is_empty() {
            options.push(SACRIFICE);
        }
        if !hand.is_empty() {
            options.push(DISCARD);
        }
        options.push(NEITHER);
        let choice = if options.len() == 1 {
            NEITHER
        } else {
            let labels = options.iter().map(|&i| Label::Card(card, i)).collect();
            options[choose::one_of(g, opponent, item.object, labels)? as usize]
        };
        match choice {
            SACRIFICE => sacrificed.extend(choose::select(g, opponent, item.object, Purpose::Sacrifice, permanents, NONLAND_PERMANENT)?.into_iter().map(|id| (opponent, id))),
            DISCARD => discarded.extend(choose::select(g, opponent, item.object, Purpose::Discard, hand, select::cards(exactly(1)))?),
            _ => neither.push(opponent),
        }
    }
    g.together(Together { sacrifices: &sacrificed, discards: &discarded, ..Default::default() });
    let power = damage_equal_to_power(g, item.object);
    let batch: Vec<(ObjectId, Target, u32)> = neither.into_iter().map(|p| (item.object, Target::Player(p), power)).collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        each_opponent_may_sacrifice_or_discard_else_damage,
        "Delirium \u{2014} At the beginning of your end step, if there are four or more card types among cards in your graveyard, each opponent may sacrifice a nonland permanent of their choice or discard a card. Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way.",
    )
    .condition(has_delirium)],
    labels: &["Sacrifice a nonland permanent", "Discard a card", "Do neither"],
    ..oracle::OSSEOUS_STICKTWISTER
};
