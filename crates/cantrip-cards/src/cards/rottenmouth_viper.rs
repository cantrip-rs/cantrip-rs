use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::targets::filters::nonland;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_oracle as oracle;

/// The choices of each opponent, as `labels` below.
const SACRIFICE: u8 = 0;
const DISCARD: u8 = 1;
const LOSE: u8 = 2;

/// "Put a blight counter on it. Then for each blight counter on it, each opponent loses 4 life unless that player sacrifices
/// a nonland permanent of their choice or discards a card." The counter goes on it only while it is on the battlefield
/// (`add_counters` puts none on an object that has left, CR 400.7); the counters are counted on it, or as it last existed
/// there (CR 608.2h).
///
/// Each opponent in turn order (CR 101.4) makes all of their choices - one for each counter: to sacrifice a nonland
/// permanent, to discard a card, or to lose 4 life, a permanent or card already chosen not chosen again, and losing 4 life
/// the one choice left to a player with neither - and then all the actions happen together (the rulings, CR 101.4): every
/// sacrifice, every discard and every loss of life at once, the player losing 4 life "the appropriate number of times" - a
/// loss of 4 for each time it was chosen, each its own occurrence (`Game::together`).
fn blight_and_punish(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Blight, 1);
    let counters = g.obj(item.object).counters(CounterKind::Blight).max(0);
    let card = g.card_id(&CARD);
    let (mut sacrificed, mut discarded, mut losses) = (Vec::new(), Vec::new(), Vec::new());
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| g.state().opponents(item.controller).any(|o| o == p)).collect();
    for opponent in opponents {
        let asker = g.item_asker(item, opponent);
        for _ in 0..counters {
            if !cantrip_mechanics::players::reached(g, opponent) {
                break;
            }
            let permanents: Vec<ObjectId> =
                g.permanents_of(opponent).into_iter().filter(|&id| nonland(g, &asker, id) && !sacrificed.iter().any(|&(_, s)| s == id) && g.can_sacrifice(opponent, id)).collect();
            let hand = g.state().zone(Zone::hand(opponent)).to_vec();
            let cards: Vec<ObjectId> = hand.iter().copied().filter(|c| !discarded.contains(c)).collect();
            let mut options = Vec::new();
            if !permanents.is_empty() {
                options.push(SACRIFICE);
            }
            if !cards.is_empty() {
                options.push(DISCARD);
            }
            options.push(LOSE);
            let choice = if options.len() == 1 {
                LOSE
            } else {
                let labels = options.iter().map(|&i| Label::Card(card, i)).collect();
                options[choose::one_of(g, opponent, item.object, labels)? as usize]
            };
            match choice {
                SACRIFICE => sacrificed.extend(choose::cards(g, opponent, item.object, Purpose::Sacrifice, permanents, 1, false)?.into_iter().map(|id| (opponent, id))),
                DISCARD => discarded.extend(choose::cards_from(g, opponent, item.object, Purpose::Discard, hand, cards, 1, false)?),
                _ => losses.push((opponent, -4)),
            }
        }
    }
    g.together(Together { sacrifices: &sacrificed, discards: &discarded, life: &losses });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "As an additional cost to cast this spell, you may sacrifice any number of nonland permanents. This spell costs {1}
    // less to cast for each permanent sacrificed this way" (CR 601.2b, 601.2f, 118.7a): how many is announced, the total
    // cost determined with it, and which ones chosen as it is paid.
    additional_cost: Some(Cost { variable: Some(VariableCost::SacrificeAnyNumber { filter: nonland, less_each: 1 }), ..Cost::NONE }),
    abilities: &[triggered(THIS_ENTERS_OR_ATTACKS, blight_and_punish, "Whenever this creature enters or attacks, put a blight counter on it. Then for each blight counter on it, each opponent loses 4 life unless that player sacrifices a nonland permanent of their choice or discards a card.")],
    labels: &["Sacrifice a nonland permanent", "Discard a card", "Lose 4 life"],
    ..oracle::ROTTENMOUTH_VIPER
};
