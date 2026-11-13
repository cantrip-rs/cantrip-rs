use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// X is the number of artifact or creature cards in one opponent's graveyard (CR 107.3a, 601.2b).
fn opponent(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player != asker.controller
}

fn graveyard_count(q: &dyn Query, _: &Asker, player: PlayerId) -> u32 {
    q.state().zone(Zone::graveyard(player)).iter().copied().filter(|&id| {
        let obj = q.state().obj(id);
        !obj.token && !obj.card_copy && (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature))
    }).fold(0u32, |count, _| count + 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(0, TargetSpec::COUNTED)], effects::distribute_plus_one_counters),
    divide: Some(Divide { total: TargetSpec::COUNTED, reads: None }),
    x_rule: Some(XRule::OfPlayer { players: opponent, value: graveyard_count }),
    ..oracle::SPOILS_OF_WAR
};
