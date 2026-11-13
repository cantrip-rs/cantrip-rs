use cantrip_api::*;
use cantrip_mechanics::{choose, effects, players, select, targets};
use cantrip_oracle as oracle;

/// II: "Each player may discard a card. Each player who doesn't loses 3 life." (CR 101.4, 608.2c, 701.9): in turn order from
/// the active player each player still in the game chooses a card of their hand or none (nothing to choose from an empty hand),
/// then the chosen cards are discarded together. A player who did not discard a card, by choice, with an empty hand or because
/// the discard was stopped, loses 3 life, all at the same time.
fn each_player_discards_or_loses_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let order: Vec<PlayerId> = g.state().apnap().collect();
    let mut chosen = Vec::new();
    for &player in &order {
        if !players::reached(g, player) {
            continue;
        }
        let hand = g.state().zone(Zone::hand(player)).to_vec();
        let card = choose::select(g, player, item.object, Purpose::Discard, hand, select::cards(select::up_to(1)))?;
        chosen.push((player, card));
    }
    let all: Vec<ObjectId> = chosen.iter().flat_map(|(_, cards)| cards.iter().copied()).collect();
    let discarded = g.discard_together(&all);
    g.end_sentence();
    // The cards discarded are new objects (CR 400.7): each is its owner's, who is therefore a player who did discard.
    let discarders: Vec<PlayerId> = discarded.iter().map(|&card| g.obj(card).owner).collect();
    let losses: Vec<(PlayerId, i32)> = chosen.iter().filter(|(player, _)| !discarders.contains(player)).map(|&(player, _)| (player, 3)).collect();
    g.lose_life_together(&losses);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::destroy_first_target, "I \u{2014} Destroy target creature.").targets(&[targets::CREATURE]),
        chapter(2, each_player_discards_or_loses_three, "II \u{2014} Each player may discard a card. Each player who doesn't loses 3 life."),
        chapter(3, effects::exile_target_players_graveyards, "III \u{2014} Exile any number of target players' graveyards.")
            .targets(&[targets::PLAYER.between(0, TargetSpec::ANY_NUMBER)]),
    ],
    ..oracle::THE_DEATH_OF_GWEN_STACY
};
