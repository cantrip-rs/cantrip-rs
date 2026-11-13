use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "Whenever a player attacks with one or more creatures" (CR 508.1m, 603.2).
fn player_attacks(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    let Event::AttackersDeclared { player, .. } = event else { return None };
    Some(Fired { subject: Target::Player(*player), ..Fired::default() })
}

/// Destroy that player's untapped non-Wall creatures that did not attack and were controlled since the turn began (CR 701.8a).
fn destroy_nonattackers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects = g.state().battlefield().iter().copied().filter(|&id| {
        let chars = g.chars(id);
        !g.obj(id).tapped
            && chars.is(CardType::Creature)
            && !chars.has_subtype(g.words(item.object).subtype(Subtype::Wall))
            && item.subject.player() == Some(g.controller(id))
            && !history::this_combat(g).iter().any(|r| matches!(r.event, Event::Attacks { attacker, .. } if attacker == id))
            && g.obj(id).controlled_since < g.state().turn.number
    }).collect::<Vec<_>>();
    g.destroy_all(&objects);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::ATTACKERS_DECLARED, player_attacks),
            destroy_nonattackers,
            "Whenever a player attacks with one or more creatures, destroy all untapped non-Wall creatures that player controls that didn't attack, except for creatures the player hasn't controlled continuously since the beginning of the turn.",
        ),
    ],
    ..oracle::TOTAL_WAR
};
