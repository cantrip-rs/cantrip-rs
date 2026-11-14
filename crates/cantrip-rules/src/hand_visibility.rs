//! Continuous public hands: Enduring Renewal, Revelation and Zur's Weirding.
//! A visibility rule is not a reveal action. History records what seats were
//! shown, and moves record visibility on each side of their simultaneous batch.
use crate::{Cx, actions, layers};
use cantrip_api::*;

pub(crate) fn mask(q: &dyn Query, cards: &CardTable) -> u8 {
    let s = q.state();
    // A live-source guard, including copies and granted rules. Merely having a
    // visibility card in the process-wide catalog must not enable the full walk.
    let interested = match s.hand_visibility_interest.get() {
        Some((epoch, interested)) if epoch == s.epoch => interested,
        _ => {
            let interested = s.battlefield().iter().any(|&id| {
                cards
                    .texts(s, s.obj(id))
                    .any(|c| cards.hand_visibility[c.0 as usize])
            }) || layers::play_rules_given_possible(cards, s)
                || s.effects.iter().any(|e| {
                    matches!(e.kind, EffectKind::PlayerPlayRule(a)
            if cards.hand_visibility[a.card.0 as usize])
                });
            s.hand_visibility_interest.set(Some((s.epoch, interested)));
            interested
        }
    };
    if !interested {
        return 0;
    }
    let mut mask = 0;
    for controller in s.apnap() {
        actions::each_play_rule(q, cards, controller, false, |source, rule| {
            if let PlayRule::HandRevealed(applies) = rule {
                let asker = Asker::new(source, controller);
                for player in s.apnap() {
                    if applies(q, &asker, player) {
                        mask |= 1 << player.0;
                    }
                }
            }
        });
    }
    mask
}

pub(crate) fn sync(cx: &mut Cx) {
    let mask = mask(cx, cx.cards());
    let gained = mask & !cx.s().shown_hands;
    cx.st().set_shown_hands(mask);
    if gained == 0 {
        return;
    }
    // No Reveal event: keeping a hand public is not repeatedly performing the
    // reveal keyword action. Existing information-only records preserve memory
    // after the source leaves, and make fresh and incremental history folds agree.
    let seats: Vec<_> = cx.s().apnap().collect();
    for &owner in &seats {
        if gained & (1 << owner.0) == 0 {
            continue;
        }
        let hand = cx.s().zone(Zone::hand(owner)).to_vec();
        for card in hand {
            for &player in &seats {
                if player != owner {
                    cx.record_only(Event::Shown { player, card });
                }
            }
        }
    }
}
