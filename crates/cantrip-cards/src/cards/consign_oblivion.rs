use cantrip_api::*;
use cantrip_mechanics::{choose_from_hand, effects};
use cantrip_oracle as oracle;

/// "Target opponent discards two cards." (CR 115.1).
fn target_opponent_discards_two_cards_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Player(p) if q.state().opponents(asker.controller).collect:: < Vec <
        PlayerId > > ().contains(& p)
    )
}

/// "Target opponent discards two cards." (CR 608.2, 701.9a).
fn target_opponent_discards_two_cards(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let players_0: Vec<PlayerId> = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.player())
        .into_iter()
        .collect();
    let mut cards_0 = Vec::new();
    for player in players_0 {
        cards_0.extend(choose_from_hand(g, player, item.object, 2)?);
    }
    g.discard_together(&cards_0);
    Ok(())
}

pub static OBLIVION: CardDef = CardDef {
    spell: spell(
        &[target(target_opponent_discards_two_cards_target_0)],
        target_opponent_discards_two_cards,
    ),
    aftermath: true,
    ..oracle::OBLIVION
};

/// "Return target nonland permanent to its owner's hand." (CR 115.1).
fn return_target_nonland_permanent_owners_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (! (q
        .chars(id).is(CardType::Land)))
    )
}

pub static CONSIGN: CardDef = CardDef {
    spell: spell(
        &[target(return_target_nonland_permanent_owners_target_0)],
        effects::return_first_target_to_hand,
    ),
    ..oracle::CONSIGN
};

/// "Return target nonland permanent to its owner's hand." (CR 604.1).
pub static CARD: CardDef = CardDef {
    split: Some((&CONSIGN, &OBLIVION)),
    ..oracle::CONSIGN_OBLIVION
};

pub static EXTRA: &[&CardDef] = &[&CONSIGN, &OBLIVION];
