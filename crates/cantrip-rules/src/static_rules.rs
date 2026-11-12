//! Static abilities that change what may be cast or activated and what it costs (CR 601.2f, 602.2b, 101.2, 606.3). They
//! are found by looking at what is on the battlefield, and at nothing else while the battlefield has none, so a game
//! without them pays one table lookup per permanent when something is cast or activated.

use cantrip_api::*;
use cantrip_state::ManaPart;

use crate::Cx;

/// The permanents with a static ability of this kind, with the player who controls each, once for each definition it has
/// one in: a merged permanent has its other components' (CR 702.140e, `CardTable::texts`).
fn sources<'a>(cx: &'a Cx, has: impl Fn(&CardDef) -> bool + Copy + 'a) -> impl Iterator<Item = (ObjectId, PlayerId, &'static CardDef, Option<Linked>)> + 'a {
    cx.s().battlefield().iter().flat_map(move |&id| source_texts(cx, id, has))
}

/// The definitions of the permanent `id` with a static ability of this kind, as `sources` gives them for one permanent on
/// the battlefield.
fn source_texts<'a>(
    cx: &'a Cx,
    id: ObjectId,
    has: impl Fn(&CardDef) -> bool + Copy + 'a,
) -> impl Iterator<Item = (ObjectId, PlayerId, &'static CardDef, Option<Linked>)> + 'a {
    let obj = cx.s().obj(id);
    // A permanent that has lost all its abilities has none of these rules (CR 613.1f).
    let lost = std::cell::OnceCell::new();
    // Each asked as the card it is printed on (CR 607.1): a component's other than the one it has now by its own link.
    cx.cards()
        .texts_linked(cx.s(), id)
        .map(|(c, by)| (cx.card(c), by))
        .filter(move |&(def, _)| has(def) && !*lost.get_or_init(|| cx.lost_all_abilities(id)))
        .map(move |(def, by)| (id, obj.controller, def, by))
}

/// "Each power-up ability of permanents you control can be activated an additional time" (CR 702.193a, 602.5b): how many more
/// times than once `player`'s power-up abilities may be activated, the sum over the permanents they control with the ability
/// (`CardDef::extra_power_up_activations`) that have not lost their abilities (CR 613.1f).
pub fn extra_power_up_activations(cx: &Cx, player: PlayerId) -> u32 {
    sources(cx, |d| d.extra_power_up_activations > 0)
        .filter(|&(_, controller, _, _)| controller == player)
        .map(|(_, _, def, _)| u32::from(def.extra_power_up_activations))
        .sum()
}

/// The players whose unspent mana of some types a static ability keeps in their pool as steps and phases end (CR 500.5,
/// `CardDef::mana_kept`): each with those types, indexed by `Mana`. Empty while no permanent with its abilities has one
/// (CR 613.1f).
pub fn mana_kept(cx: &Cx) -> Vec<(PlayerId, [bool; 6])> {
    let mut kept: Vec<(PlayerId, [bool; 6])> = Vec::new();
    for (id, controller, def, by) in sources(cx, |d| !d.mana_kept.is_empty()) {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        for player in cx.s().seats() {
            for mana in Mana::ALL {
                if def.mana_kept.iter().any(|rule| rule(cx, &asker, player, mana)) {
                    match kept.iter_mut().find(|(p, _)| *p == player) {
                        Some((_, k)) => k[mana as usize] = true,
                        None => {
                            let mut k = [false; 6];
                            k[mana as usize] = true;
                            kept.push((player, k));
                        }
                    }
                }
            }
        }
    }
    kept
}

/// The parts of the ordinary mana in `player`'s pool told apart for its payer (CR 106.3): mana its payer chooses to spend
/// or keep (CR 601.2h, 118.3a). Each is a source with the delayed ability that rides its mana, 0 for none (`ManaPart`). A
/// source's mana is told apart where an ability of a permanent on the battlefield says so (`CardDef::mana_told_apart`);
/// mana a "when that mana is spent" delayed ability rides (`ManaFrom::rider`, CR 603.7a) is told apart by the rules
/// themselves, wherever its source has gone, from every other mana - its own source's other mana and every other rider's,
/// the same source's included: whether it is spent decides whether that ability triggers, and two riders are two abilities,
/// each with its own words (CR 603.7, 612). Asked only while the pool holds more than one part, since one part's mana is
/// all alike: a pool of one land's mana searches nothing. Restricted mana is told apart by its restriction alone (CR
/// 106.6); none in content is told apart by source too.
pub fn mana_told_apart(q: &dyn Query, cards: &'static CardTable, player: PlayerId) -> Vec<ManaPart> {
    let s = q.state();
    let mut parts: Vec<ManaPart> = Vec::new();
    for e in s.mana_from_of(player) {
        let part = (e.source, e.rider);
        if e.restriction.is_none() && !parts.contains(&part) {
            parts.push(part);
        }
    }
    if parts.len() < 2 {
        return Vec::new();
    }
    let riding = parts.iter().any(|&(_, rider)| rider != 0);
    let rules: Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> = s
        .battlefield()
        .iter()
        .flat_map(|&id| {
            let obj = s.obj(id);
            // A permanent that has lost all its abilities has none of these rules (CR 613.1f); a merged permanent has its
            // other components' (CR 702.140e).
            cards
                .texts_linked(s, id)
                .map(|(c, by)| (cards.get(c), by))
                .filter(move |(def, _)| !def.mana_told_apart.is_empty() && !q.lost_all_abilities(id))
                .map(move |(def, by)| (id, obj.controller, def, by))
        })
        .collect();
    if rules.is_empty() && !riding {
        return Vec::new();
    }
    parts.retain(|&(source, rider)| {
        rider != 0 || rules.iter().any(|&(id, controller, def, by)| def.mana_told_apart.iter().any(|rule| rule(q, &Asker { instance: None, source: id, controller, by, spell: false }, source)))
    });
    parts
}

/// The kinds of zone `player` can't cast spells from right now (CR 101.2, 601.3): those a static ability of a permanent on the
/// battlefield forbids (Voice of Victory forbids them all on its controller's turn, Drannith Magistrate every zone but the
/// hand), and those an effect about the player does (`EffectKind::CantCastSpellsFrom`, Avatar's Wrath). A restriction takes
/// precedence over any permission to cast (CR 101.2).
pub fn cast_barred(cx: &Cx, player: PlayerId) -> ZoneKinds {
    let mut barred = ZoneKinds::NONE;
    for e in &cx.s().effects {
        if let EffectKind::CantCastSpellsFrom(p, zones) = e.kind
            && p == player
        {
            barred = barred.union(zones);
        }
    }
    for (id, controller, def, by) in sources(cx, |d| !d.cast_rules.is_empty()) {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        for from in ZoneKinds::CAST_FROM {
            if !barred.contains(from) && def.cast_rules.iter().any(|rule| !rule(cx, &asker, player, from)) {
                barred = barred.with(from);
            }
        }
    }
    barred
}

/// What forbids one player to cast spells with certain qualities now (CR 101.2, 601.3), whatever zone they are cast from:
/// the static abilities of permanents on the battlefield that do (`CardDef::spell_rules`: Gaddock Teeg, Deafening Silence),
/// with their controllers, and the rule of the game that a player can't cast a legendary instant or sorcery spell unless
/// they control a legendary creature or a legendary planeswalker (CR 205.4e), which forbids nothing to a player who does.
/// Empty, so unallocated, in a game where the player controls a legend and nothing else forbids.
pub struct SpellBans {
    forbidders: Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)>,
    /// The player controls no legendary creature and no legendary planeswalker (CR 205.4e).
    legendless: bool,
    /// A static ability that functions outside the battlefield exists (`layers::statics_everywhere`), so that a spell may be
    /// other than the card where it is (`actions::SpellsAsCast`). Asked only where CR 205.4e is all that could forbid.
    everywhere: bool,
}

impl SpellBans {
    /// Nothing forbids the player any spell for its qualities.
    pub fn is_empty(&self) -> bool {
        self.forbidders.is_empty() && !self.legendless
    }

    /// Only CR 205.4e could forbid one, and then only a spell that is a legendary instant or sorcery; and no static ability
    /// that functions outside the battlefield could make a spell other than the card where it is.
    pub fn legendary_only_and_local(&self) -> bool {
        self.forbidders.is_empty() && !self.everywhere
    }
}

/// What forbids `player` to cast spells with certain qualities now (`SpellBans`): the player's legends read as they are now
/// (CR 205.4e: "controls a legendary creature or a legendary planeswalker", each a permanent with both qualities).
pub fn spell_bans(cx: &Cx, player: PlayerId) -> SpellBans {
    let legendless = !cx.s().battlefield().iter().any(|&id| {
        cx.s().obj(id).controller == player && {
            let chars = cx.chars(id);
            chars.supertypes.legendary && (chars.is(CardType::Creature) || chars.is(CardType::Planeswalker))
        }
    });
    let forbidders: Vec<_> = sources(cx, |d| !d.spell_rules.is_empty()).collect();
    let everywhere = legendless && forbidders.is_empty() && crate::layers::statics_everywhere(cx.cards(), cx.s());
    SpellBans { forbidders, legendless, everywhere }
}

/// Whether a spell with characteristics `chars` is a legendary instant or sorcery spell (CR 205.4e).
pub fn legendary_instant_or_sorcery(chars: &Chars) -> bool {
    chars.supertypes.legendary && (chars.is(CardType::Instant) || chars.is(CardType::Sorcery))
}

/// Whether `bans` (`spell_bans`, made for `player`) forbid `player` to cast `spell`, read through `q`: the card seen as the
/// face it would be cast as (CR 601.3e), or the spell on the stack once its proposal is complete (CR 601.6).
pub fn spell_forbidden(q: &dyn Query, bans: &SpellBans, player: PlayerId, spell: ObjectId) -> bool {
    (bans.legendless && legendary_instant_or_sorcery(&q.chars(spell)))
        || bans.forbidders.iter().any(|&(id, controller, def, by)| def.spell_rules.iter().any(|rule| !rule(q, &Asker { instance: None, source: id, controller, by, spell: false }, player, spell)))
}

/// The permanents on the battlefield with a static ability that keeps cards in some zones from entering the battlefield
/// (`CardDef::enter_rules`, CR 101.2), with their controllers: empty, so unallocated, in nearly every game.
pub fn enter_bans(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> {
    sources(cx, |d| !d.enter_rules.is_empty()).collect()
}

/// Whether one of `bans` (`enter_bans`) keeps `card` from being put onto the battlefield from where it is (CR 101.2): the card
/// judged as it exists there (`EnterRule`). A permanent spell resolving is put there from the stack, which no such ability
/// names, so it is not asked.
pub fn enter_banned(q: &dyn Query, bans: &[(ObjectId, PlayerId, &'static CardDef, Option<Linked>)], card: ObjectId) -> bool {
    !q.obj(card).zone.is(ZoneKind::Stack)
        && bans.iter().any(|&(id, controller, def, by)| def.enter_rules.iter().any(|rule| !rule(q, &Asker { instance: None, source: id, controller, by, spell: false }, card)))
}

/// The permanents on the battlefield with a static ability that keeps some land plays from being made: one that keeps cards
/// from entering the battlefield from where they are (`CardDef::enter_rules`, as playing a land puts it there, CR 305.1) or
/// one that forbids playing some lands (`CardDef::land_play_rules`, CR 101.2), with their controllers. One look at the
/// battlefield for both, made only where a land could be played; empty, so unallocated, in nearly every game.
pub fn land_play_bans(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> {
    sources(cx, |d| !d.enter_rules.is_empty() || !d.land_play_rules.is_empty()).collect()
}

/// Whether one of `bans` (`land_play_bans`) forbids `player` to play `land`, where it is now, as `face` (`LandPlayRule`, CR
/// 101.2, 305.1, 712.12). A restriction takes precedence over whatever allows the play (CR 101.2). Whether the land could
/// enter the battlefield from there is `enter_banned`'s, over the same list.
pub fn land_play_forbidden(q: &dyn Query, bans: &[(ObjectId, PlayerId, &'static CardDef, Option<Linked>)], player: PlayerId, land: ObjectId, face: CardId) -> bool {
    bans.iter().any(|&(id, controller, def, by)| {
        def.land_play_rules.iter().any(|rule| !rule(q, &Asker { instance: None, source: id, controller, by, spell: false }, player, land, face))
    })
}

/// Whether a static ability of a permanent on the battlefield keeps `card` from being put onto the battlefield from where it
/// is now (`enter_banned`, CR 101.2).
pub fn cant_enter(cx: &Cx, card: ObjectId) -> bool {
    let bans = enter_bans(cx);
    !bans.is_empty() && enter_banned(cx, &bans, card)
}

/// Whether a static ability makes the damage `source` is about to deal unpreventable (CR 615.12). Asked only when
/// protection would prevent it, which is rare, so the battlefield is searched then.
pub fn damage_cant_be_prevented(cx: &Cx, source: ObjectId, combat: bool) -> bool {
    for (id, controller, def, by) in sources(cx, |d| !d.unpreventable_damage.is_empty()) {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        if def.unpreventable_damage.iter().any(|rule| rule(cx, &asker, source, combat)) {
            return true;
        }
    }
    false
}

/// Whether a static ability of a permanent on the battlefield keeps the ability `item` from being countered (CR 101.2,
/// 701.6a, `CardDef::uncounterable_abilities`). Asked only as an effect would counter an ability, which is rare, so the
/// battlefield is searched then.
pub fn ability_cant_be_countered(cx: &Cx, item: &StackItem) -> bool {
    sources(cx, |d| !d.uncounterable_abilities.is_empty()).any(|(id, controller, def, by)| {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        def.uncounterable_abilities.iter().any(|rule| rule(cx, &asker, item))
    })
}

/// Which of the `damaged` permanents keep their damage through this cleanup step (CR 514.2): those a static ability of a
/// permanent on the battlefield says so of (`CardDef::damage_stays`), while that permanent has its abilities (CR 613.1f).
/// Asked only in a cleanup step with damage marked, so the battlefield is searched then.
pub fn damage_stays(cx: &Cx, damaged: &[ObjectId]) -> Vec<ObjectId> {
    let rules: Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> = sources(cx, |d| !d.damage_stays.is_empty()).collect();
    if rules.is_empty() {
        return Vec::new();
    }
    damaged
        .iter()
        .copied()
        .filter(|&object| {
            rules.iter().any(|&(id, controller, def, by)| {
                let asker = Asker { instance: None, source: id, controller, by, spell: false };
                def.damage_stays.iter().any(|rule| rule(cx, &asker, object))
            })
        })
        .collect()
}

/// Whether the planeswalker, with 0 loyalty, stays on the battlefield (CR 704.5i): a static ability of a permanent on the
/// battlefield says it isn't put into its owner's graveyard for having 0 loyalty (`CardDef::zero_loyalty_stays`), while
/// that permanent has its abilities (CR 613.1f). Asked only of a planeswalker with no loyalty, so the battlefield is
/// searched then.
pub fn zero_loyalty_stays(cx: &Cx, planeswalker: ObjectId) -> bool {
    sources(cx, |d| !d.zero_loyalty_stays.is_empty()).any(|(id, controller, def, by)| {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        def.zero_loyalty_stays.iter().any(|rule| rule(cx, &asker, planeswalker))
    })
}

/// How many creatures may attack each of `targets`, a player, planeswalker or battle, in a declaration of attackers (CR
/// 508.1c): the smallest a static ability of a permanent on the battlefield that has its abilities (CR 613.1f) says of
/// it (`CardDef::attack_limits`), with no entry for a target nothing caps. Asked as attackers are declared, only of the
/// players, planeswalkers and battles that may be attacked then.
pub fn attack_limits(cx: &Cx, targets: &[Target]) -> Vec<(Target, u32)> {
    let rules: Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> = sources(cx, |d| !d.attack_limits.is_empty()).collect();
    if rules.is_empty() {
        return Vec::new();
    }
    targets
        .iter()
        .filter_map(|&target| {
            let mut limit = None;
            for &(id, controller, def, by) in &rules {
                let asker = Asker { instance: None, source: id, controller, by, spell: false };
                for rule in def.attack_limits {
                    if let Some(n) = rule(cx, &asker, target) {
                        limit = Some(limit.map_or(n, |most: u32| most.min(n)));
                    }
                }
            }
            limit.map(|n| (target, n))
        })
        .collect()
}

/// Whether a static ability of a permanent on the battlefield says the "legend rule" doesn't apply to `legend` (CR 704.5j,
/// `CardDef::legend_rule_exempt`: Mirror Gallery), while that permanent has its abilities (CR 613.1f): once it leaves, the
/// rule applies the next time state-based actions are checked (Mirror Gallery's ruling). Asked only of a legendary
/// permanent, so the battlefield is searched then.
pub fn legend_rule_exempt(cx: &Cx, legend: ObjectId) -> bool {
    sources(cx, |d| !d.legend_rule_exempt.is_empty()).any(|(id, controller, def, by)| {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        def.legend_rule_exempt.iter().any(|rule| rule(cx, &asker, legend))
    })
}

/// Whether `player` can't gain life (CR 119.7): a resolved effect about them says so (`EffectKind::CantGainLife`, Screaming
/// Nemesis's "for the rest of the game"), or a static ability of a permanent on the battlefield does (`CardDef::cant_gain_life`,
/// Sunspine Lynx's "Players can't gain life"), which stops when the permanent leaves or loses its abilities (CR 611.3a,
/// 613.1f). Asked only when a player would gain life, so the battlefield is searched then.
pub fn cant_gain_life(q: &dyn Query, cards: &'static CardTable, player: PlayerId) -> bool {
    let s = q.state();
    if s.effects.iter().any(|e| e.kind == EffectKind::CantGainLife(player)) {
        return true;
    }
    s.battlefield().iter().any(|&id| {
        let obj = s.obj(id);
        cards.texts_linked(s, id).map(|(c, by)| (cards.get(c), by)).any(|(def, by)| {
            !def.cant_gain_life.is_empty() && !q.lost_all_abilities(id) && {
                let asker = Asker { instance: None, source: id, controller: obj.controller, by, spell: false };
                def.cant_gain_life.iter().any(|rule| rule(q, &asker, player))
            }
        })
    })
}

/// Whether `player` can't lose life (CR 119.8): a static ability of a permanent with its abilities, or of an emblem, says so
/// (`CardDef::cant_lose_life`: Platinum Emperion's "your life total can't change"). Asked only when a player would lose or
/// pay life.
pub fn cant_lose_life(q: &dyn Query, cards: &'static CardTable, player: PlayerId) -> bool {
    any_static(q, cards, |d| d.cant_lose_life, |rule, asker| rule(q, asker, player))
}

/// Whether `creature` may attack as though it didn't have defender (CR 702.3b, 609.4): a resolved effect says so this turn
/// (`EffectKind::CanAttackDespiteDefender`, Assault Formation's {G} ability), or a static ability of a permanent on the
/// battlefield does (`CardDef::attacks_despite_defender`: Demon Wall's "as long as this creature has a counter on it",
/// which stops when the permanent leaves or loses its abilities, CR 611.3a, 613.1f). It is a permission about attacking
/// alone: the creature keeps defender, which everything else still sees (CR 609.4), and nothing here lifts any other
/// restriction on attacking (CR 508.1a, 508.1c). Asked only of a creature with defender, so the battlefield is searched
/// then.
pub fn may_attack_despite_defender(q: &dyn Query, cards: &'static CardTable, creature: ObjectId) -> bool {
    let s = q.state();
    if s.effects.iter().any(|e| e.affected == creature && e.kind == EffectKind::CanAttackDespiteDefender) {
        return true;
    }
    s.battlefield().iter().any(|&id| {
        let obj = s.obj(id);
        cards.texts_linked(s, id).map(|(c, by)| (cards.get(c), by)).any(|(def, by)| {
            !def.attacks_despite_defender.is_empty() && !q.lost_all_abilities(id) && {
                let asker = Asker { instance: None, source: id, controller: obj.controller, by, spell: false };
                def.attacks_despite_defender.iter().any(|rule| rule(q, &asker, creature))
            }
        })
    })
}

/// Live sources of attack-only summoning-sickness exceptions (CR 302.6, 609.4).
/// Asked lazily only when an otherwise eligible creature is summoning sick.
/// Empty sources allocate nothing and read no extra characteristics.
pub(crate) fn attack_haste_sources(cx: &Cx) -> Vec<(Asker, &'static CardDef)> {
    sources(cx, |d| !d.attacks_as_though_haste.is_empty())
        .map(|(source, controller, def, by)| (Asker { source, controller, by, spell: false, instance: None }, def)).collect()
}

/// What each attack open to the active player's creatures costs to declare (CR 508.1d, 508.1h): for each of `eligible`
/// and each of `targets`, every static ability of a permanent on the battlefield that makes that attack cost something
/// (`CardDef::attack_costs`: Propaganda's "creatures can't attack you unless their controller pays {2} for each creature
/// they control that's attacking you"), added up, with the first such permanent named as the cost's source. Attacks that
/// cost nothing have no entry. A board with no such permanent, nearly every board, makes no list.
pub fn attack_costs(cx: &Cx, eligible: &[ObjectId], targets: &[Target]) -> Vec<AttackCost> {
    let taxes: Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> = sources(cx, |d| !d.attack_costs.is_empty()).collect();
    if taxes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for &attacker in eligible {
        for &target in targets {
            let mut total = ManaCost::ZERO;
            let mut first = None;
            for &(id, controller, def, by) in &taxes {
                let asker = Asker { instance: None, source: id, controller, by, spell: false };
                for rule in def.attack_costs {
                    if let Some(cost) = rule(cx, &asker, attacker, target)
                        && !cost.is_zero()
                    {
                        total = total.plus(&cost);
                        first.get_or_insert(id);
                    }
                }
            }
            if let Some(source) = first {
                out.push(AttackCost { attacker, target, cost: total, source });
            }
        }
    }
    out
}

/// Whether `player` may spend mana as though it were of any color to pay a cost for `source` (CR 609.4b): to activate an
/// ability of it, to cast it (a spell on the stack), or anything else (`purpose`).
pub fn spend_any_color_for(cx: &Cx, player: PlayerId, source: ObjectId, purpose: SpendingFor) -> bool {
    lets_spend_any_color(cx, &any_color_spenders(cx), &Spending { player, source: Some(source), purpose })
}

/// Whether `player` may spend mana as though it were of any color for every payment, whatever it is for (CR 609.4b:
/// Mycosynth Lattice), of the `spenders` (from `any_color_spenders`): what the advice of what can be paid reads once
/// (`Available`).
pub fn spend_any_color_always(cx: &Cx, spenders: &[(ObjectId, PlayerId, &'static CardDef, Option<Linked>)], player: PlayerId) -> bool {
    lets_spend_any_color(cx, spenders, &Spending { player, source: None, purpose: SpendingFor::Other })
}

/// The permanents whose static abilities may let a player spend mana as though it were mana of any color (CR 609.4b): found
/// from the battlefield, and empty (so unallocated) in nearly every game. Asked once, then `lets_spend_any_color` per payment.
pub fn any_color_spenders(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> {
    sources(cx, |d| !d.spend_any_color.is_empty()).collect()
}

/// Whether one of the `spenders` (from `any_color_spenders`) lets the payment `spending` be made with mana spent as though it
/// were mana of any color (CR 609.4b). `q` is what the rule reads the payment's source through: a view of the face a card
/// would be cast as, where its castability is judged as a face other than the card itself (CR 712.11c, 709.3a, 715.4).
pub fn lets_spend_any_color(q: &dyn Query, spenders: &[(ObjectId, PlayerId, &'static CardDef, Option<Linked>)], spending: &Spending) -> bool {
    spenders
        .iter()
        .any(|&(id, controller, def, by)| def.spend_any_color.iter().any(|rule| rule(q, &Asker { instance: None, source: id, controller, by, spell: false }, spending)))
}

/// Whether `player` may activate the abilities of `creature` as though it had haste (CR 302.6): a static ability of a
/// permanent `player` controls says so.
pub fn activates_as_though_haste(cx: &Cx, player: PlayerId, creature: ObjectId) -> bool {
    sources(cx, |d| !d.activate_as_though_haste.is_empty()).any(|(id, controller, def, by)| {
        controller == player && def.activate_as_though_haste.iter().any(|rule| rule(cx, &Asker { instance: None, source: id, controller, by, spell: false }, creature))
    })
}

/// Whether `creature` stations permanents using its toughness rather than its power (CR 702.184c): a static ability of a
/// permanent on the battlefield says so (`CardDef::stations_by_toughness`), asked as the station ability resolves, of the
/// creature as it last existed on the battlefield if it has left.
pub fn stations_by_toughness(cx: &Cx, creature: ObjectId) -> bool {
    sources(cx, |d| !d.stations_by_toughness.is_empty()).any(|(id, controller, def, by)| {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        def.stations_by_toughness.iter().any(|rule| rule(cx, &asker, creature))
    })
}

/// What `creature` assigns as combat damage (CR 510.1a): its power, or its toughness if it has the ability to
/// (`Keyword::AssignsDamageByToughness`, printed or granted, as the layers left it), a static ability of a permanent says so
/// (`CardDef::damage_by_toughness`), or an effect about its controller does (`EffectKind::CreaturesAssignDamageByToughness`).
/// An "as though its power were positive" rule (CR 609.4) uses negative power's magnitude only when assigning by power;
/// every other procedure still reads its actual negative power (CR 107.1b).
pub fn combat_damage_of(cx: &Cx, creature: ObjectId) -> u32 {
    let c = cx.chars(creature);
    let by_toughness = c.has(Keyword::AssignsDamageByToughness) || sources(cx, |d| !d.damage_by_toughness.is_empty()).any(|(id, controller, def, by)| {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        def.damage_by_toughness.iter().any(|rule| rule(cx, &asker, creature))
    }) || (c.toughness.unwrap_or(0) > c.power.unwrap_or(0)
        && cx.s().effects.iter().any(|e| e.kind == EffectKind::CreaturesAssignDamageByToughness && e.controller == cx.s().obj(creature).controller));
    if by_toughness {
        return c.toughness.unwrap_or(0).max(0) as u32;
    }
    let power = c.power.unwrap_or(0);
    if power < 0 && sources(cx, |d| !d.damage_by_absolute_power.is_empty()).any(|(id, controller, def, by)| {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        def.damage_by_absolute_power.iter().any(|rule| rule(cx, &asker, creature))
    }) {
        return power.unsigned_abs();
    }
    power.max(0) as u32
}

/// What `creature`, with `power`, counts as towards a crew, saddle or station cost of `permanent` (CR 702.122b, 702.171a,
/// 702.184a): its power, or what a static ability of its own makes it count as - "Sulu stations Spacecraft and crews
/// Vehicles as though his power were tripled" (Sulu, Heroic Helmsman, `CardDef::tap_power`). Such an ability is about the
/// creature that has it and no other, so only `creature`'s own are asked, while it is on the battlefield with its abilities
/// (CR 613.1f). Asked of each creature tapped for such a cost.
pub fn tapped_power(cx: &Cx, creature: ObjectId, power: i32, permanent: ObjectId) -> i32 {
    let obj = cx.s().obj(creature);
    if !obj.zone.is(ZoneKind::Battlefield) || obj.became.is_some() || obj.last_known.is_some() {
        return power;
    }
    source_texts(cx, creature, |d| !d.tap_power.is_empty()).fold(power, |power, (id, controller, def, by)| {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        def.tap_power.iter().fold(power, |power, rule| rule(cx, &asker, power, permanent))
    })
}

/// The card types `player` may cast as though they had flash (CR 307.5, 702.8): from effects that gave them that.
pub fn flash_types(cx: &Cx, player: PlayerId) -> TypeSet {
    flash_type_grants(cx, player).fold(TypeSet::NONE, |types, (_, t)| TypeSet(types.0 | t.0))
}

/// Each effect that lets `player` cast spells of some card types as though they had flash (CR 307.5, 702.8): its source and
/// the types.
pub fn flash_type_grants<'a>(cx: &'a Cx, player: PlayerId) -> impl Iterator<Item = (ObjectId, TypeSet)> + 'a {
    cx.s().effects.iter().filter_map(move |e| match e.kind {
        EffectKind::CastAsThoughFlash(t) if e.controller == player => Some((e.source, t)),
        _ => None,
    })
}

/// The permanents whose static abilities let their controller cast some spells as though they had flash (CR 307.5): found
/// from the battlefield, and empty (so unallocated) in nearly every game. Asked once per question, then `grants_flash` per card.
pub fn flash_granters(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> {
    sources(cx, |d| !d.flash_rules.is_empty()).collect()
}

/// Whether one of the `granters` (from `flash_granters`) lets `player` cast `spell` as though it had flash (CR 307.5, 601.3b,
/// 702.8), asked of the spell it would be on the stack under `player` (CR 601.3e, `actions::SpellsAsCast`).
pub fn grants_flash(q: &dyn Query, granters: &[(ObjectId, PlayerId, &'static CardDef, Option<Linked>)], player: PlayerId, spell: ObjectId) -> bool {
    granters.iter().any(|&(id, controller, def, by)| {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        def.flash_rules.iter().any(|rule| rule(q, &asker, spell, player))
    })
}

/// The permanents on the battlefield with a static ability that lets a player pay another cost rather than an activated
/// ability's mana cost (`CardDef::ability_alternatives`, CR 118.9, 602.2b), with their controllers: empty, so unallocated,
/// in nearly every game.
pub fn activation_alternative_sources(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> {
    sources(cx, |d| !d.ability_alternatives.is_empty()).collect()
}

/// Each cost one of `alternative_sources` lets `player` pay rather than the mana cost of this ability of `permanent`
/// (CR 118.9, 602.2b), with the `Grant` that names it (the permanent and `AbilityRef::ability_alternative`): each a way of
/// activating it of its own.
pub fn activation_alternatives(
    cx: &Cx,
    alternative_sources: &[(ObjectId, PlayerId, &'static CardDef, Option<Linked>)],
    player: PlayerId,
    permanent: ObjectId,
    def: &ActivatedDef,
) -> Vec<(Grant, ManaCost)> {
    let mut all = Vec::new();
    for &(id, controller, card, by) in alternative_sources {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        let card_id = cx.card_id(card);
        for (k, rule) in card.ability_alternatives.iter().enumerate() {
            if let Some(cost) = rule(cx, &asker, player, permanent, def) {
                all.push((Grant { object: id, ability: AbilityRef::ability_alternative(card_id, k) }, cost));
            }
        }
    }
    all
}

/// The cost the alternative `grant` names (`activation_alternatives`) lets `player` pay for this ability of `permanent`
/// now, read from the permanent's card: none where it no longer offers one.
pub fn named_activation_alternative(q: &dyn Query, grant: Grant, player: PlayerId, permanent: ObjectId, def: &ActivatedDef) -> Option<ManaCost> {
    let k = grant.ability.as_ability_alternative()?;
    let rule = q.card(grant.ability.card).ability_alternatives.get(k)?;
    let obj = q.state().obj(grant.object);
    if !obj.zone.is(ZoneKind::Battlefield) || q.lost_all_abilities(grant.object) {
        return None;
    }
    // Asked as the card it is printed on: a merged permanent's component's by its own link (CR 607.1, 702.140e).
    let by = q.state().merge_of(grant.object).map(|_| Linked { object: grant.object, text: q.linked_card(grant.ability.card), acquired: Acquired::OWN }).filter(|&l| l != q.text_linked(grant.object));
    rule(q, &Asker { instance: None, source: grant.object, controller: obj.controller, by, spell: false }, player, permanent, def)
}

/// Whether a static ability lets `player` activate this sorcery-speed ability whenever they could cast an instant.
pub fn instant_activation(cx: &Cx, player: PlayerId, source: ObjectId, index: u8) -> bool {
    for (id, controller, def, by) in sources(cx, |d| !d.instant_activation.is_empty()) {
        let asker = Asker { instance: None, source: id, controller, by, spell: false };
        if def.instant_activation.iter().any(|rule| rule(cx, &asker, source, index, player)) {
            return true;
        }
    }
    false
}

/// Whether an effect lets `player` activate the loyalty abilities of `planeswalker`, a permanent they control, any time they
/// could cast an instant (`EffectKind::InstantLoyalty`, CR 606.3, 611.2c). Read from the effects in force, so a game with
/// none pays one pass over them, and only when a loyalty ability is asked about outside sorcery timing.
pub fn instant_loyalty(cx: &Cx, player: PlayerId, planeswalker: ObjectId) -> bool {
    cx.s().effects.iter().any(|e| match e.kind {
        EffectKind::InstantLoyalty(kind) if e.controller == player => {
            let c = cx.chars(planeswalker);
            c.is(CardType::Planeswalker) && kind.is_none_or(|t| c.has_subtype(t))
        }
        _ => false,
    })
}

/// The permanents whose cards forbid activating abilities (CR 602.5): found from the board, not the card table, and empty
/// (so unallocated) in nearly every game. Asked once per question, then `activation_barred` per ability.
pub fn activation_forbidders(cx: &Cx) -> Vec<ObjectId> {
    cx.s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let obj = cx.s().obj(id);
            cx.cards().texts(cx.s(), obj).any(|c| cx.cards().activation_rules[c.0 as usize]) && !cx.lost_all_abilities(id)
        })
        .collect()
}

/// Whether one of the `forbidders` (from `activation_forbidders`) stops `source`'s ability `def` being activated (CR 602.5).
pub fn activation_barred(cx: &Cx, forbidders: &[ObjectId], source: ObjectId, def: &ActivatedDef) -> bool {
    // A special action is no activated ability (CR 116.2, 602.1): what forbids activating abilities (CR 602.5) - an effect on
    // the object, a static ability's rule - forbids none of them. Turning a face-down creature face up, foretelling,
    // suspending, plotting and unlocking a door stay allowed; what stops the cast suspend needs is judged as a cast is.
    if def.special_action {
        return false;
    }
    // "Its activated abilities can't be activated", from a resolved effect on the object itself, and "power-up abilities can't be
    // activated", from one on the game, in the extra turn it is about and no other (CR 602.5, 611.2).
    let extra = cx.s().turn.extra;
    let barred = |e: &ContinuousEffect| match e.kind {
        EffectKind::CantActivateAbilities => e.affected == source,
        EffectKind::CantActivatePowerUp => def.power_up && e.duration == Duration::EndOfExtraTurn(extra),
        _ => false,
    };
    if cx.s().effects.iter().any(barred) {
        return true;
    }
    forbidders.iter().any(|&id| {
        let obj = cx.s().obj(id);
        cx.cards().texts_linked(cx.s(), id).any(|(c, by)| {
            let asker = Asker { instance: None, source: id, controller: obj.controller, by, spell: false };
            cx.card(c).activation_rules.iter().any(|rule| !rule(cx, &asker, source, def))
        })
    })
}

/// The permanents on the battlefield with a static ability that restricts attacking and their abilities (CR 508.1c, 613.1f;
/// `CardDef::attack_rules`), each with the `Asker` its rules are asked with. Nearly every board has none.
fn attack_restrictors(q: &dyn Query, cards: &CardTable) -> Vec<(Asker, &'static CardDef)> {
    q.state()
        .battlefield()
        .iter()
        .flat_map(|&id| {
            let obj = q.state().obj(id);
            cards
                .texts_linked(q.state(), id)
                .filter(move |(card, _)| cards.attack_rules[card.0 as usize] && !q.lost_all_abilities(id))
                .map(move |(card, by)| (Asker { instance: None, source: id, controller: obj.controller, by, spell: false }, cards.get(card)))
        })
        .collect()
}

/// The permanents on the battlefield with a static ability that goads creatures (`CardDef::goads`, CR 701.15a), each text
/// that has one with the `Asker` its rules are asked with: every text the permanent has (CR 707.2, 613.2a: a copy's, a
/// merged permanent's components', an exception's), and none of one that has lost its abilities (CR 613.1f). Nearly every
/// board has none.
pub fn goad_sources(q: &dyn Query, cards: &CardTable) -> Vec<(Asker, &'static CardDef)> {
    q.state()
        .battlefield()
        .iter()
        .flat_map(|&id| {
            let obj = q.state().obj(id);
            cards
                .texts_linked(q.state(), id)
                .filter(move |(card, _)| cards.goads[card.0 as usize] && !q.lost_all_abilities(id))
                .map(move |(card, by)| (Asker { instance: None, source: id, controller: obj.controller, by, spell: false }, cards.get(card)))
        })
        .collect()
}

/// Every player goading `creature` (CR 701.15a), in seat order, each once (CR 701.15d): the goader of a resolved goad effect,
/// and the controller of a source among `sources` (`goad_sources`) whose static ability says the creature is goaded (CR
/// 701.15b).
pub fn goaders_among(q: &dyn Query, sources: &[(Asker, &'static CardDef)], creature: ObjectId) -> Vec<PlayerId> {
    let effects = q.state().effects.iter().filter(|e| e.affected == creature && e.kind == EffectKind::Goaded).map(|e| e.controller);
    let statics = sources.iter().filter(|(asker, def)| def.goads.iter().any(|rule| rule(q, asker, creature))).map(|(asker, _)| asker.controller);
    let mut goaders: Vec<PlayerId> = effects.chain(statics).collect();
    goaders.sort_unstable_by_key(|p| p.0);
    goaders.dedup();
    goaders
}

/// Every player goading `creature` now (`goaders_among` the battlefield's `goad_sources`).
pub fn goaders(q: &dyn Query, cards: &CardTable, creature: ObjectId) -> Vec<PlayerId> {
    goaders_among(q, &goad_sources(q, cards), creature)
}

/// The creatures among `creatures` that a static ability forbids to attack alone (CR 508.1c; `CardDef::attack_alone_rules`),
/// each asked of a rule of the permanent that says so with that permanent's `Asker`. Pet Sehlat.
pub fn attacks_alone_barred(q: &dyn Query, cards: &CardTable, creatures: &[ObjectId]) -> Vec<ObjectId> {
    let restrictors: Vec<(Asker, &'static CardDef)> = q
        .state()
        .battlefield()
        .iter()
        .flat_map(|&id| {
            let obj = q.state().obj(id);
            cards
                .texts_linked(q.state(), id)
                .filter(move |(card, _)| cards.attack_alone_rules[card.0 as usize] && !q.lost_all_abilities(id))
                .map(move |(card, by)| (Asker { instance: None, source: id, controller: obj.controller, by, spell: false }, cards.get(card)))
        })
        .collect();
    creatures
        .iter()
        .copied()
        .filter(|&creature| {
            restrictors
                .iter()
                .any(|(asker, def)| def.attack_alone_rules.iter().any(|rule| !rule(q, asker, creature, Target::None)))
        })
        .collect()
}

/// Whether every restriction of `restrictors` lets `creature` attack `at` (CR 508.1c).
fn restrictors_allow(q: &dyn Query, restrictors: &[(Asker, &'static CardDef)], creature: ObjectId, at: Target) -> bool {
    restrictors.iter().all(|(asker, def)| def.attack_rules.iter().all(|rule| rule(q, asker, creature, at)))
}

/// What the static abilities of the permanents on the battlefield forbid `creatures` to attack among `targets` (CR 508.1c):
/// each attack some attack rule of a permanent that has not lost its abilities (CR 613.1f) says no to, asked with that
/// permanent's `Asker` ("this creature can't attack unless ...", Patchwork Beastie; "creatures with flying can't attack
/// you", Storm, Windrider). A creature none of whose attacks is left can't attack at all.
pub fn attacks_barred(q: &dyn Query, cards: &CardTable, creatures: &[ObjectId], targets: &[Target]) -> Vec<(ObjectId, Target)> {
    let restrictors = attack_restrictors(q, cards);
    let mut barred = lasting_rules_barring_attacks(q, cards, creatures, targets);
    if restrictors.is_empty() {
        return barred;
    }
    for &creature in creatures {
        for &at in targets {
            if !restrictors_allow(q, &restrictors, creature, at) && !barred.contains(&(creature, at)) {
                barred.push((creature, at));
            }
        }
    }
    barred
}

/// Attacks forbidden by resolved rules (CR 508.1c, 611.2c): Chronomantic Escape's open set is asked
/// as attackers are declared; Promise of Loyalty names individual objects. Neither grants an ability,
/// and both outlive their source. The named player is fixed on resolution; their planeswalkers, if the
/// wording includes them, are read under their current controller and types. Battles are not included.
pub fn lasting_rules_barring_attacks(q: &dyn Query, cards: &CardTable, creatures: &[ObjectId], targets: &[Target]) -> Vec<(ObjectId, Target)> {
    let mut barred = Vec::new();
    for e in &q.state().effects {
        let (player, planeswalkers) = match e.kind {
            EffectKind::CantAttackPlayer { player, planeswalkers } => (player, planeswalkers),
            EffectKind::LastingRule { card, index, about: Target::Player(player) }
                if cards.get(card).lasting_rules.get(index as usize).is_some_and(|r| r.rule == LastingRuleKind::CantAttackYou) => (player, false),
            _ => continue,
        };
        for &creature in creatures {
            let applies = match e.kind {
                EffectKind::CantAttackPlayer { .. } => e.affected == creature,
                EffectKind::LastingRule { card, index, about } =>
                    (cards.get(card).lasting_rules[index as usize].applies)(q, &Asker::new(e.source, e.controller), about, creature),
                _ => unreachable!(),
            };
            if !applies { continue; }
            for &at in targets {
                let protected = match at {
                    Target::Player(p) => p == player,
                    Target::Object(id) => planeswalkers && q.controller(id) == player && q.chars(id).is(CardType::Planeswalker),
                    _ => false,
                };
                if protected && !barred.contains(&(creature, at)) { barred.push((creature, at)); }
            }
        }
    }
    barred
}

/// Whether the static abilities of the permanents on the battlefield let `creature` attack any of `targets` (CR 508.1c),
/// or `targets` is empty; the targets are worked out only where some permanent restricts attacking.
pub fn attack_rules_allow_any(q: &dyn Query, cards: &CardTable, creature: ObjectId, targets: impl FnOnce() -> Vec<Target>) -> bool {
    let restrictors = attack_restrictors(q, cards);
    if restrictors.is_empty() && q.state().effects.is_empty() {
        return true;
    }
    let targets = targets();
    let barred = lasting_rules_barring_attacks(q, cards, &[creature], &targets);
    targets.is_empty() || targets.iter().any(|&at| !barred.contains(&(creature, at)) && restrictors_allow(q, &restrictors, creature, at))
}

/// The requirements to block that the static abilities of the permanents on the battlefield with their abilities (CR 613.1f)
/// put on the blocks among `allowed` (CR 509.1c, `CardDef::block_requirements`: Lure's "all creatures able to block enchanted
/// creature do so"): one for each permanent and each block it requires, so two Lures on a creature make two of each creature
/// able to block it. A board with no such permanent, nearly every board, makes none.
pub fn block_requirements(cx: &Cx, allowed: &[(ObjectId, ObjectId)]) -> Vec<RequiredBlock> {
    let rules: Vec<(ObjectId, PlayerId, &'static CardDef, Option<Linked>)> =
        sources(cx, |d| !d.block_requirements.is_empty() || !d.must_be_blocked.is_empty()).collect();
    let mut required = Vec::new();
    for &(id, _, def, _) in &rules {
        for &(blocker, attacker) in allowed {
            if def.block_requirements.iter().any(|rule| rule(cx, id, blocker, attacker)) {
                required.push(RequiredBlock { blocker: Some(blocker), attacker: Some(attacker) });
            }
        }
        // "Must be blocked if able" (`CardDef::must_be_blocked`): one requirement on an attacker some block is allowed for,
        // however many creatures may block it.
        let mut asked: Vec<ObjectId> = Vec::new();
        for &(_, attacker) in allowed {
            if !asked.contains(&attacker) {
                asked.push(attacker);
                if def.must_be_blocked.iter().any(|rule| rule(cx, id, attacker)) {
                    required.push(RequiredBlock { blocker: None, attacker: Some(attacker) });
                }
            }
        }
    }
    required
}

/// Whether a static ability of a permanent with its abilities (CR 613.1f), or an emblem in the command zone
/// (CR 114.4), says `test` of its own rules. Gideon of the Trials and Platinum Angel both forbid losing and winning.
/// Asked only when the rules need the answer: losing or winning, damage, or counters about to be put.
fn any_static<R: Copy + 'static>(q: &dyn Query, cards: &CardTable, rules: fn(&CardDef) -> &'static [R], test: impl Fn(R, &Asker) -> bool) -> bool {
    let s = q.state();
    // As in `actions::given`, commanders have no functioning abilities here (CR 113.6); emblems have no types.
    let emblems = s.zone(Zone::COMMAND).iter().filter(|&&id| cards.get(cards.current(s.obj(id))).types == TypeSet::NONE);
    s.battlefield().iter().chain(emblems).any(|&id| {
        let obj = s.obj(id);
        cards.texts_linked(s, id).any(|(card, by)| {
            let list = rules(cards.get(card));
            !list.is_empty() && !q.lost_all_abilities(id) && {
                let asker = Asker { instance: None, source: id, controller: obj.controller, by, spell: false };
                list.iter().any(|&rule| test(rule, &asker))
            }
        })
    })
}

/// Whether `player` doesn't lose the game for `reason` (CR 104.3, 704.5a-c): a static ability says so (`CardDef::loss_rules`:
/// Platinum Angel's "you can't lose the game", Phyrexian Unlife's "you don't lose the game for having 0 or less life").
pub fn doesnt_lose(q: &dyn Query, cards: &'static CardTable, player: PlayerId, reason: LossReason) -> bool {
    any_static(q, cards, |d| d.loss_rules, |rule, asker| rule(q, asker, player, reason))
}

/// Every miracle ability a card in its owner's hand has (CR 702.94a): its own first, then those the permanents on the
/// battlefield give it (`CardDef::granted_miracles`: Lorehold, the Historian), in the battlefield's order, each rules text of
/// a merged permanent's included (CR 702.140e), each named by the card it is printed on.
pub fn miracles(q: &dyn Query, cards: &'static CardTable, card: ObjectId) -> Vec<Miracle> {
    let mut all: Vec<Miracle> = q.card(q.obj(card).card).miracle.map(|cost| Miracle { granter: None, cost: Some(cost) }).into_iter().collect();
    let s = q.state();
    for &id in s.battlefield() {
        let o = s.obj(id);
        for (granter, by) in cards.texts_linked(s, id) {
            let def = cards.get(granter);
            // A permanent that has lost all its abilities gives nothing (CR 613.1f).
            if def.granted_miracles.is_empty() || q.lost_all_abilities(id) {
                continue;
            }
            let asker = Asker { instance: None, source: id, controller: o.controller, by, spell: false };
            for (i, grant) in def.granted_miracles.iter().enumerate() {
                if (grant.has)(q, &asker, card) {
                    all.push(Miracle { granter: Some((granter, i as u8)), cost: (grant.cost)(q, q.obj(card).card).map(|c| reduced_by(c, u32::from(grant.less)).0) });
                }
            }
        }
    }
    all
}

/// Whether `player` can't draw `n` more cards right now (CR 121.2b, 121.3, 614.17, `Game::cant_draw`): "each player can't
/// draw more than one card each turn" (`CardDef::cant_draw`: Spirit of the Labyrinth).
pub fn cant_draw(q: &dyn Query, cards: &'static CardTable, player: PlayerId, n: u32) -> bool {
    any_static(q, cards, |d| d.cant_draw, |rule, asker| rule(q, asker, player, n))
}

/// Whether the coins `player` is about to flip together come up heads and are won (CR 705.3): a static ability of a permanent
/// with its abilities, or of an emblem, says so (`CardDef::coin_flip_rules`: Edgar, King of Figaro's "the first time you flip
/// one or more coins each turn, those coins come up heads and you win those flips").
pub fn coin_flips_won_on_heads(q: &dyn Query, cards: &'static CardTable, player: PlayerId) -> bool {
    any_static(q, cards, |d| d.coin_flip_rules, |rule, asker| rule(q, asker, player))
}

/// Whether `player` can't win the game (CR 104.2b): Platinum Angel's "your opponents can't win the game" (`CardDef::win_rules`).
pub fn cant_win(q: &dyn Query, cards: &'static CardTable, player: PlayerId) -> bool {
    any_static(q, cards, |d| d.win_rules, |rule, asker| rule(q, asker, player))
}

/// Whether damage dealt to `player` now is dealt as though its source had infect (CR 702.90b): Phyrexian Unlife's
/// (`CardDef::infect_damage_to`).
pub fn damage_as_though_infect(q: &dyn Query, cards: &'static CardTable, player: PlayerId) -> bool {
    any_static(q, cards, |d| d.infect_damage_to, |rule, asker| rule(q, asker, player))
}

/// Whether `object`, controlled by `controller` (as it enters, the player it enters under), can't have `kind` counters put on
/// it (CR 122.6, 101.2): Melira, Sylvok Outcast's "creatures you control can't have -1/-1 counters put on them"
/// (`CardDef::counter_bans`).
pub fn counters_banned(q: &dyn Query, cards: &'static CardTable, object: ObjectId, controller: PlayerId, kind: CounterKind) -> bool {
    any_static(q, cards, |d| d.counter_bans, |rule, asker| rule(q, asker, object, controller, kind))
}

/// Whether a permanent on the battlefield has a rule about the counters a permanent can't have put on it
/// (`CardDef::counter_bans`), whatever its abilities: the guard of judging a permanent about to enter with counters as it
/// would exist on the battlefield (CR 614.12, 122.6).
pub fn counter_bans_possible(state: &GameState, cards: &'static CardTable) -> bool {
    state.battlefield().iter().any(|&id| cards.texts(state, state.obj(id)).any(|c| !cards.get(c).counter_bans.is_empty()))
}

/// Whether `player` can't get `kind` counters (CR 122.1, 101.2): Melira's "you can't get poison counters"
/// (`CardDef::player_counter_bans`).
pub fn player_counters_banned(q: &dyn Query, cards: &'static CardTable, player: PlayerId, kind: PlayerCounter) -> bool {
    any_static(q, cards, |d| d.player_counter_bans, |rule, asker| rule(q, asker, player, kind))
}

/// Whether a static ability forbids `player` to make `payment` for a cost paid for `paid_for` (CR 101.2, 118.3, 601.2h,
/// 602.2b, `CardDef::cost_bans`): Yasharn, Implacable Earth's "players can't pay life or sacrifice nonland permanents to cast
/// spells or activate abilities". Asked of life, sacrifice and beam-up payments as a spell is cast or an ability activated.
pub fn cost_banned(q: &dyn Query, cards: &'static CardTable, player: PlayerId, paid_for: CostFor, payment: CostPayment) -> bool {
    any_static(q, cards, |d| d.cost_bans, |rule, asker| rule(q, asker, player, paid_for, payment))
}

/// Whether a static ability of a permanent or of an emblem says the permanent `id` doesn't untap during its controller's
/// untap step (CR 502.3, `CardDef::untap_bans`: Intruder Alarm, Infinite Coursework). No ability of `id`'s, so it holds
/// whatever abilities `id` has lost (CR 613.1f); its own "doesn't untap" is `Keyword::DoesNotUntap`.
pub fn untap_banned(q: &dyn Query, cards: &'static CardTable, id: ObjectId) -> bool {
    any_static(q, cards, |d| d.untap_bans, |rule, asker| rule(q, asker, id))
}

/// Whether a static ability of a permanent on the battlefield says the permanent `id` can't phase in (CR 702.26a, 101.2,
/// `CardDef::phase_in_bans`: The City on the Edge of Forever's "Permanents can't phase in"). No ability of `id`'s, so it
/// holds whatever abilities `id` has lost (CR 613.1f), and it keeps `id` phased out rather than phasing it in and out
/// again.
pub fn phase_in_banned(q: &dyn Query, cards: &'static CardTable, id: ObjectId) -> bool {
    any_static(q, cards, |d| d.phase_in_bans, |rule, asker| rule(q, asker, id))
}

/// The rules about an open set of objects that resolved spells and abilities made (CR 611.2c, `EffectKind::LastingRule`) that
/// say `rule` of the permanent `id` now: one for each such effect whose set takes in `id` as it is now, asked as the effect's
/// source and controller with what its words named (`OpenSet`). A rule of the game, not an ability of `id` or of the source,
/// so it holds whatever abilities either has lost (CR 613.1f), and it outlives the source (CR 611.2a).
pub fn lasting_rules_on<'a>(
    q: &'a dyn Query,
    cards: &'static CardTable,
    id: ObjectId,
    rule: LastingRuleKind,
) -> impl Iterator<Item = &'a ContinuousEffect> + 'a {
    q.state().effects.iter().filter(move |e| match e.kind {
        EffectKind::LastingRule { card, index, about } => cards
            .get(card)
            .lasting_rules
            .get(index as usize)
            .is_some_and(|r| r.rule == rule && (r.applies)(q, &Asker::new(e.source, e.controller), about, id)),
        _ => false,
    })
}

/// Whether a resolved effect says the permanent `id`, which `active` controls, doesn't untap during `active`'s untap step
/// (CR 502.3, 611.2c): one on `id` (`EffectKind::DoesntUntap`: Tangle, Frost Breath) or a rule about an open set that takes it
/// in (`LastingRuleKind::DoesntUntap`: Exhaustion), for as long as it lasts - one "during [a player's] next untap step"
/// (`Duration::DuringNextUntapStepOf`) only in that player's (the Exhaustion ruling). No ability of `id`'s, so it holds
/// whatever abilities `id` has lost (CR 613.1f).
pub fn untap_step_skips(q: &dyn Query, cards: &'static CardTable, id: ObjectId, active: PlayerId) -> bool {
    let this_step = |e: &ContinuousEffect| !matches!(e.duration, Duration::DuringNextUntapStepOf(p) if p != active);
    q.state().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::DoesntUntap && this_step(e))
        || lasting_rules_on(q, cards, id, LastingRuleKind::DoesntUntap).any(this_step)
}

/// Whether the permanent `id` can't become untapped (CR 101.2, 701.26b): a resolved effect says so
/// (`EffectKind::CantBecomeUntapped`: Spider-Woman, Secret Agent), or a static ability of a permanent with its abilities, or
/// of an emblem, does (`CardDef::cant_become_untapped`: Frozen in Ice, Blossombind). No ability of `id`'s, so it holds
/// whatever abilities `id` has lost (CR 613.1f). Asked only where a tapped permanent would untap.
pub fn cant_become_untapped(q: &dyn Query, cards: &CardTable, id: ObjectId) -> bool {
    q.state().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::CantBecomeUntapped)
        || any_static(q, cards, |d| d.cant_become_untapped, |rule, asker| rule(q, asker, id))
}

/// Whether the face-down permanent `id` can't be turned face up (CR 101.2, 708.7): a static ability of a permanent or of
/// an emblem says so (`CardDef::face_up_bans`: Unable to Scream's enchanted creature, Karlov Watchdog's "permanents your
/// opponents control ... during your turn"). No ability of `id`'s, so it holds whatever abilities `id` has lost (CR 613.1f).
/// Asked where a face-down permanent would be turned face up - as its special actions are offered (CR 116.2b) and as it is
/// turned up, by them or by an effect - so it stops every way, and lifts the moment the ban does.
pub fn cant_be_turned_face_up(q: &dyn Query, cards: &'static CardTable, id: ObjectId) -> bool {
    any_static(q, cards, |d| d.face_up_bans, |rule, asker| rule(q, asker, id))
}

/// Whether the permanent `id` can't be sacrificed (CR 101.2, 701.21a), for any reason and by anyone: it has "this permanent
/// can't be sacrificed" (`Keyword::CantBeSacrificed`, printed or granted, so losing its abilities ends it), a resolved effect
/// says so (`EffectKind::CantBeSacrificed`, which it keeps whatever abilities it loses), or a static ability of a permanent or
/// of an emblem does (`CardDef::sacrifice_bans`: Assault Suit's equipped creature). Asked only where a sacrifice would be
/// made, chosen or paid.
pub fn cant_be_sacrificed(q: &dyn Query, cards: &'static CardTable, id: ObjectId) -> bool {
    q.chars(id).has(Keyword::CantBeSacrificed)
        || q.state().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::CantBeSacrificed)
        || any_static(q, cards, |d| d.sacrifice_bans, |rule, asker| rule(q, asker, id))
}

/// Whether a continuous effect or a functioning static ability forbids
/// regenerating `id` (CR 101.2, 701.19b-c). Asked when destruction would be
/// replaced, not when a spell or ability creates a regeneration shield.
pub fn cant_be_regenerated(q: &dyn Query, cards: &'static CardTable, id: ObjectId) -> bool {
    q.state().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::CantBeRegenerated)
        || any_static(q, cards, |d| d.regeneration_bans, |rule, asker| rule(q, asker, id))
        // A granted ability has the holder as its source, and the layer-6
        // result decides whether it survived a later loss of abilities. Do not
        // reject all grants merely because the holder lost its printed ones.
        || q.state().battlefield().iter().any(|&source| {
            q.chars(source).abilities.into_iter().flatten().filter(|a| a.is_delayed()).any(|ability| {
                match cards.ability(ability) {
                    AbilityDef::RegenerationBan(ban) => (ban.rule)(q, &Asker::new(source, q.controller(source)), id),
                    _ => false,
                }
            })
        })
}
