//! Attachment (CR 301.5 Equipment, 301.6 Fortifications, 303.4 Auras, 701.3): what a legal host is, shared by the effect that attaches
//! (CR 701.3b: an illegal attachment does not move), the state-based actions that shake an attachment off
//! (CR 704.5m-n) and an Aura choosing its host as it enters (CR 303.4f), so they can never disagree.

use cantrip_api::*;

use crate::layers::live;
use crate::{Cx, R, ops};

/// Whether `attachment` could be attached to `host` right now (CR 701.3a-b): the host is still the object it was,
/// is not the attachment itself and does not have protection from it (CR 702.16c-d), and is one the attachment's
/// enchant abilities, Equipment's creature, or Fortification's land on the battlefield (CR 301.5c, 301.6), allow.
/// Every restriction applies when it has several attachment subtypes. An Aura that is also a creature can
/// enchant nothing (CR 303.4d). Judged without targeting, so hexproof does not matter.
pub fn host_allows(q: &dyn Query, attachment: ObjectId, host: ObjectId, attaching: bool) -> bool {
    host_allows_as(q, attachment, &q.chars(attachment), q.controller(attachment), q.mana_value(attachment), host, attaching)
}

/// The same authority for a prospective attachment, before it enters (CR 614.12).
pub(crate) fn host_allows_as(q: &dyn Query, attachment: ObjectId, ac: &Chars, controller: PlayerId, mana_value: u32, host: ObjectId, attaching: bool) -> bool {
    if attachment == host || !live(q.state(), host) {
        return false;
    }
    let hc = q.chars(host);
    if hc.protected_from_by(ac, || mana_value, controller) {
        return false;
    }
    let asker = Asker { instance: None, source: attachment, controller, by: None, spell: false };
    kinds_allow(q, ac, &asker, Target::Object(host), attaching)
}

/// Each subtype restricts attachment independently; Battles cannot be attached
/// at all (CR 310.10). Used for current and prospective objects and player hosts.
fn kinds_allow(q: &dyn Query, ac: &Chars, asker: &Asker, host: Target, attaching: bool) -> bool {
    if ac.is(CardType::Battle) {
        return false;
    }
    let equipment = ac.has_subtype(Subtype::Equipment);
    let fortification = ac.has_subtype(Subtype::Fortification);
    if equipment {
        // An Equipment that's a creature can't equip unless it has reconfigure (CR 301.5c), and then only as it attaches.
        let creature_ok = attaching && q.card(ac.card).reconfigure;
        if (ac.is(CardType::Creature) && !creature_ok)
            || !host.object().is_some_and(|id| q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature)) {
            return false;
        }
    }
    if fortification && (ac.is(CardType::Creature)
        || !host.object().is_some_and(|id| q.is_on_battlefield(id) && q.chars(id).is(CardType::Land))) {
        return false;
    }
    if ac.has_subtype(Subtype::Aura) {
        return !ac.is(CardType::Creature) && enchants_allow(q, ac, asker, host);
    }
    equipment || fortification
}

/// The enchant abilities an Aura has (CR 702.5a): among its abilities as the layers leave them (`AbilityRef::enchant`), its
/// card's printed one and each an effect gave it (`EffectKind::Enchant`: Necromancy, `EntersAs::aura`). A "loses all
/// abilities" takes them as it takes any ability (CR 613.1f).
fn enchants<'a>(q: &'a dyn Query, chars: &'a Chars) -> impl Iterator<Item = TargetSpec> + 'a {
    chars.abilities().filter_map(move |a| match a.index {
        AbilityRef::ENCHANT => q.card(a.card).enchant,
        AbilityRef::ENCHANT_GIVEN => q.card(a.card).becomes_aura_enchant,
        _ => None,
    })
}

/// Whether an Aura with these characteristics may enchant `what` (CR 303.4a, 303.4c, 704.5m): each of its enchant
/// abilities allows it, as each restricts what it can enchant (CR 702.5c). One that has lost its enchant abilities is
/// restricted by none: what it is attached to is not illegal on that ground.
fn enchants_allow(q: &dyn Query, chars: &Chars, asker: &Asker, what: Target) -> bool {
    enchants(q, chars).all(|spec| (spec.filter)(q, asker, what))
}

/// Whether an Aura with "enchant player" can enchant `player` right now (CR 303.4, 704.5m): a player still in the game,
/// without protection from it (CR 702.16c: from everything, CR 702.16j, or from the Aura's controller, 702.16k), whom its
/// enchant abilities allow. An Aura that is also a creature can enchant no one (CR 303.4d).
pub fn player_allows(cx: &Cx, aura: ObjectId, player: PlayerId) -> bool {
    let q: &dyn Query = cx;
    if q.state().player(player).lost
        || crate::layers::player_protection(cx.cards(), cx.s(), &cx.g.log, player, q.controller(aura)).is_some()
    {
        return false;
    }
    let ac = q.chars(aura);
    let asker = Asker { instance: None, source: aura, controller: q.controller(aura), by: None, spell: false };
    kinds_allow(q, &ac, &asker, Target::Player(player), false)
}

/// What an Aura card about to enter would enchant (CR 303.4f: "a legal object or player"): the battlefield, then the
/// graveyards, as the C# engine lists them, leaving out itself and everything entering with it; then the players, in turn
/// order, still in the game and without protection from its controller (CR 702.16k). Judged on the Aura as it would exist there
/// (`aura_chars`, CR 614.12) under `controller`: each of its enchant abilities allows it, and it has no protection from the
/// Aura. An Aura that would be a creature can enchant nothing (CR 303.4d).
pub(crate) fn aura_hosts(cx: &Cx, aura: ObjectId, aura_chars: &Chars, controller: PlayerId, mana_value: u32, entering: &[ObjectId]) -> Vec<Target> {
    if aura_chars.is(CardType::Creature) {
        return Vec::new();
    }
    let asker = Asker { instance: None, source: aura, controller, by: None, spell: false };
    let state = cx.s();
    let mut zones = vec![Zone::BATTLEFIELD];
    zones.extend(state.seats().map(Zone::graveyard));
    let mut hosts = Vec::new();
    for zone in zones {
        for &id in state.zone(zone) {
            let target = Target::Object(id);
            if entering.contains(&id) || !host_allows_as(cx, aura, aura_chars, controller, mana_value, id, true) {
                continue;
            }
            hosts.push(target);
        }
    }
    for p in state.apnap() {
        if kinds_allow(cx, aura_chars, &asker, Target::Player(p), true)
            && crate::layers::player_protection(cx.cards(), state, &cx.g.log, p, controller).is_none()
        {
            hosts.push(Target::Player(p));
        }
    }
    hosts
}

/// Whether a card about to enter does so as an Aura that chooses what it enchants (CR 303.4f): one that prints an enchant
/// ability, or one the effect putting it there makes an Aura (`EntersAs::aura`).
fn enters_as_aura(cx: &Cx, m: &ops::Moving) -> bool {
    let Some(e) = m.entering else { return false };
    let card = e.copy.map(|c| c.card).unwrap_or(cx.s().obj(m.id).card);
    cx.card(card).enchant.is_some() || e.becomes.is_some_and(|b| b.what.aura)
}

/// Auras entering without being cast choose what they enchant, in turn order (CR 303.4f, 101.4); a choice made for
/// one is no target (hexproof does not matter), and an Aura with nothing to enchant does not enter (CR 303.4g). A card an
/// effect puts there as an Aura (`EntersAs::aura`) is one as it enters.
pub fn choose_aura_hosts(cx: &mut Cx, moves: &mut Vec<ops::Moving>) -> R<()> {
    let mut auras: Vec<usize> = (0..moves.len()).filter(|&i| enters_as_aura(cx, &moves[i])).collect();
    if auras.is_empty() {
        return Ok(());
    }
    let entering: Vec<ObjectId> = moves.iter().map(|m| m.id).collect();
    let apnap: Vec<PlayerId> = cx.s().apnap().collect();
    auras.sort_by_key(|&i| {
        let controller = moves[i].entering.map(|e| e.controller);
        apnap.iter().position(|&p| Some(p) == controller)
    });
    let mut dropped = Vec::new();
    for i in auras {
        let Some(mut e) = moves[i].entering else { continue };
        // Judged on the Aura as it would be on the battlefield (CR 614.12): its enchant abilities as the layers leave them,
        // and a creature card an effect returns as an Aura enchantment no creature.
        let prospective = ops::as_it_would_exist(cx, &e);
        let hosts = aura_hosts(cx, moves[i].id, &prospective.chars, e.controller, prospective.mana_value, &entering);
        let chosen = match hosts.len() {
            0 => {
                dropped.push(i);
                continue;
            }
            1 => hosts[0],
            _ => {
                let ask = RequestKind::ChooseTarget { replacing: None, label: "", slot_player: None, together: None,
                    source: moves[i].id,
                    purpose: Purpose::Attach,
                    candidates: hosts,
                    optional: false,
                    least: 1,
                    count: 1,
                    any_number: false,
                };
                match cx.ask_core(e.controller, ask, false)? {
                    Answer::Targets(t) if t.len() == 1 => t[0],
                    _ => continue,
                }
            }
        };
        e.attach = chosen.object();
        e.attach_player = chosen.player();
        moves[i].entering = Some(e);
    }
    dropped.sort_unstable();
    for i in dropped.into_iter().rev() {
        moves.remove(i);
    }
    Ok(())
}
