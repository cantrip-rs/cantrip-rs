use cantrip_api::*;
use cantrip_mechanics::library::exile_top_may_play;
use cantrip_oracle as oracle;

/// "Cards from their hand" (CR 611.2c): any card in a hand as the rule is read, so one drawn later is covered too and one that
/// left the hand is not. It binds whoever would play it (`Target::None` names no player, CR 101.2).
fn card_in_a_hand(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.state().obj(id).zone.is(ZoneKind::Hand)
}

/// "Each player exiles the top seven cards of their library. Until your next turn, players may play cards they exiled this way,
/// and they can't play cards from their hand." (CR 406, 305.1, 601.3, 611.2a): one instruction, in turn order from the active
/// player (CR 101.4). Each player's exiled cards are playable by that player (their owner, as each exiled their own top cards)
/// until the next turn of the ability's controller begins - wherever their own turns fall in between - and the hand rule lasts as
/// long. A land among them is played as a land play, under the ordinary rules (CR 305.2). The exiled cards stay in exile
/// unplayable once the permission ends.
fn each_player_exiles_seven_and_plays_only_those(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let until = Duration::UntilNextTurnOf(item.controller);
    let grant = PlayGrant { lands: true, expiry: Expiry::UntilNextTurnOf(item.controller), ..PlayGrant::CAST };
    let players: Vec<PlayerId> = g.state().apnap().collect();
    instruction(g, |g| {
        for player in players {
            exile_top_may_play(g, player, 7, player, grant);
        }
    });
    g.end_sentence();
    g.add_lasting_rule(item, 0, Target::None, until);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(EXILE_THIS),
        each_player_exiles_seven_and_plays_only_those,
        "{T}, Exile this artifact: Each player exiles the top seven cards of their library. Until your next turn, players may play cards they exiled this way, and they can't play cards from their hand. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    lasting_rules: &[LastingRule { text: "Until your next turn, players can't play cards from their hand.", rule: LastingRuleKind::CantBePlayed, applies: card_in_a_hand }],
    ..oracle::MEMORY_VESSEL
};
