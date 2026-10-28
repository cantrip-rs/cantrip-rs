//! Myriad (CR 702.116a): "Whenever this creature attacks, for each opponent other than defending player, you may create a
//! token that's a copy of this creature that's tapped and attacking that player or a planeswalker they control. If one or
//! more tokens are created this way, exile the tokens at end of combat." Each instance triggers separately (CR 702.116b).
//! Caller of the Pack, Dalek Squadron, Herald of the Host, Tabaxi Toucaneers, Tiamat's Fanatics, Warchief Giant and Wyrm's
//! Crossing Patrol print it.
//!
//! A card with myriad lists `TRIGGER` among its `abilities` and `EXILE` as its first `delayed` ability.

use crate::combat::{choose_among, player_or_their_planeswalkers};
use crate::triggers::THIS_ATTACKS;
use cantrip_api::*;

/// "You may create a token ... that player or a planeswalker they control", for one opponent: the choice of whether is the
/// choice of what the token attacks, none being no token. `{player}` is the opponent, whose player and planeswalkers are the
/// candidates.
const LABEL: &str = "Myriad: create a token copy tapped and attacking {player} or a planeswalker they control (choose none to create none)";

/// Whether to create the token for `opponent`, and if so what it attacks (CR 508.4): asked of each opponent other than the
/// defending player, in turn order, before any token is made.
fn choose_for(g: &mut dyn Game, you: PlayerId, source: ObjectId, opponent: PlayerId) -> Result<Option<Target>, Suspend> {
    let candidates = player_or_their_planeswalkers(g, opponent);
    let ask = RequestKind::ChooseTarget {
        source,
        purpose: Purpose::EnterAttacking,
        candidates,
        optional: true,
        least: 0,
        count: 1,
        any_number: false,
        label: LABEL,
        slot_player: Some(opponent),
        together: None,
    };
    Ok(match g.ask(you, ask)? {
        Answer::Targets(t) => t.first().copied(),
        _ => None,
    })
}

/// "For each opponent other than defending player, you may create a token that's a copy of this creature that's tapped and
/// attacking that player or a planeswalker they control": the copy is of this creature as the ability resolves, or as it last
/// existed if it has left (CR 707.2, 113.7a). The defending player is the one it was attacking, or the controller of the
/// planeswalker it was attacking, as it became an attacking creature this combat (the rulings). The tokens all enter together,
/// tapped (CR 614.1c), and are attacking (CR 508.4) - never declared as attackers, so nothing that triggers on attackers sees
/// them, myriad itself included. A replacement that makes more tokens (Doubling Season) makes more of them for each opponent,
/// and the controller chooses separately for each token what it attacks. The tokens are exiled at end of combat, if any was
/// created.
fn create_copies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (me, you) = (item.object, item.controller);
    let Some(defending) = g.defending_player(me) else { return Ok(()) };
    let opponents: Vec<PlayerId> = g.state().opponents(you).filter(|&p| p != defending).collect();
    let mut plan: Vec<(PlayerId, Target)> = Vec::new();
    for opponent in opponents {
        if let Some(target) = choose_for(g, you, me, opponent)? {
            plan.push((opponent, target));
        }
    }
    if plan.is_empty() {
        return Ok(());
    }
    let values = g.copiable_values(me);
    let created = g.create_token_copies_tapped(values, you, plan.len() as u32, true)?;
    // The copies among what was created, in order: a replacement may have added tokens of another kind.
    let copies: Vec<ObjectId> = created.into_iter().filter(|&t| g.obj(t).card == values.card).collect();
    // Each opponent's tokens are a share of them: a replacement multiplies the one it was asked for.
    let share = (copies.len() / plan.len()).max(1);
    let mut attacking: Vec<(Target, Vec<ObjectId>)> = Vec::new();
    for (i, &token) in copies.iter().enumerate() {
        let (opponent, first) = plan[(i / share).min(plan.len() - 1)];
        let target = if i % share == 0 {
            first
        } else {
            let candidates = player_or_their_planeswalkers(g, opponent);
            choose_among(g, you, me, &candidates)?.unwrap_or(first)
        };
        match attacking.iter_mut().find(|(t, _)| *t == target) {
            Some((_, tokens)) => tokens.push(token),
            None => attacking.push((target, vec![token])),
        }
    }
    for (target, tokens) in attacking {
        g.enter_attacking(&tokens, target);
    }
    if !copies.is_empty() {
        g.delayed_trigger_about(
            DelayedWhen::Step { step: Step::EndOfCombat, on_turn_of: None },
            me,
            you,
            AbilityRef::delayed(item.ability.card, 0),
            &copies,
        );
    }
    Ok(())
}

/// "Exile the tokens at end of combat" (CR 603.7): the tokens the ability created that are still on the battlefield, exiled
/// together (a token that has gone is gone).
fn exile_tokens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tokens = g.event_objects(item).to_vec();
    g.exile_all(&tokens);
    Ok(())
}

/// Myriad, with the reminder text its cards print.
pub const TRIGGER: AbilityDef = triggered(
    THIS_ATTACKS,
    create_copies,
    "Myriad (Whenever this creature attacks, for each opponent other than defending player, you may create a token copy that's tapped and attacking that player or a planeswalker they control. Exile the tokens at end of combat.)",
);

/// Myriad's delayed ability, the card's first `delayed` ability: the tokens are exiled at end of combat.
pub const EXILE: AbilityDef = delayed_trigger(exile_tokens, "Exile the tokens created by myriad at end of combat.");
