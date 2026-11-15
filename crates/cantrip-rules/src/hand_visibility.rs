//! Continuous public hands: Enduring Renewal, Revelation and Zur's Weirding; and
//! single cards of a hand a resolved rule keeps revealed (Firestorm Phoenix).
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

/// The cards of hands that a resolved rule keeps public one by one (`LastingRuleKind::RevealedInHand`, CR 701.20a: Firestorm
/// Phoenix's "that player plays with that card revealed in their hand"), beside the whole hands `mask` makes public. The hands
/// are walked only while some resolved rule about an open set lasts, which is none in nearly every game.
pub(crate) fn cards(q: &dyn Query, table: &'static CardTable) -> Vec<ObjectId> {
    let s = q.state();
    if !s.effects.iter().any(|e| matches!(e.kind, EffectKind::LastingRule { .. })) {
        return Vec::new();
    }
    s.seats()
        .flat_map(|p| s.zone(Zone::hand(p)).iter().copied())
        .filter(|&id| crate::static_rules::lasting_rules_on(q, table, id, LastingRuleKind::RevealedInHand).next().is_some())
        .collect()
}

/// Whether `id`, a card in a hand, is public now: its whole hand is (`mask`), or a rule keeps it so alone (`cards`).
pub(crate) fn revealed(q: &dyn Query, table: &'static CardTable, id: ObjectId) -> bool {
    let zone = q.obj(id).zone;
    zone.is(ZoneKind::Hand)
        && (mask(q, table) & (1 << zone.player.0) != 0
            || crate::static_rules::lasting_rules_on(q, table, id, LastingRuleKind::RevealedInHand).next().is_some())
}

/// Whether `player` has been shown `card` since it arrived where it is (an object in a hand is that card only from its
/// arrival, CR 400.7): read back through the history to that arrival.
fn shown_since_it_arrived(q: &dyn Query, player: PlayerId, card: ObjectId) -> bool {
    for r in q.history().all().iter().rev() {
        match r.event {
            Event::Shown { player: p, card: c } if p == player && c == card => return true,
            Event::ZoneChange { new, .. } if new == card => return false,
            _ => {}
        }
    }
    false
}

pub(crate) fn sync(cx: &mut Cx) {
    let mask = mask(cx, cx.cards());
    let gained = mask & !cx.s().shown_hands;
    cx.st().set_shown_hands(mask);
    // A card a rule keeps public alone, in a hand not public as a whole, is shown to each other seat once, as it becomes public.
    for card in cards(cx, cx.cards()) {
        let owner = cx.s().obj(card).zone.player;
        if mask & (1 << owner.0) != 0 {
            continue;
        }
        let seats: Vec<_> = cx.s().apnap().filter(|&p| p != owner).collect();
        for player in seats {
            if !shown_since_it_arrived(cx, player, card) {
                cx.record(Event::Shown { player, card });
            }
        }
    }
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
                    cx.record(Event::Shown { player, card });
                }
            }
        }
    }
}
