//! Static abilities that change what may be cast or activated and what it costs (CR 601.2f, 602.2b, 101.2, 606.3). They
//! are found by looking at what is on the battlefield, and at nothing else while the battlefield has none, so a game
//! without them pays one table lookup per permanent when something is cast or activated.

use cantrip_api::*;

use crate::Cx;

/// The permanents with a static ability of this kind, with the player who controls each.
fn sources<'a>(cx: &'a Cx, has: impl Fn(&CardDef) -> bool + 'a) -> impl Iterator<Item = (ObjectId, PlayerId, &'static CardDef)> + 'a {
    cx.s().battlefield().iter().filter_map(move |&id| {
        let obj = cx.s().obj(id);
        let def = cx.card(cx.cards().current(obj));
        // A permanent that has lost all its abilities has none of these rules (CR 613.1f).
        (has(def) && !cx.lost_all_abilities(id)).then_some((id, obj.controller, def))
    })
}

/// "Each power-up ability of permanents you control can be activated an additional time" (CR 702.193a, 602.5b): how many more
/// times than once `player`'s power-up abilities may be activated, the sum over the permanents they control with the ability
/// (`CardDef::extra_power_up_activations`) that have not lost their abilities (CR 613.1f).
pub fn extra_power_up_activations(cx: &Cx, player: PlayerId) -> u32 {
    sources(cx, |d| d.extra_power_up_activations > 0)
        .filter(|&(_, controller, _)| controller == player)
        .map(|(_, _, def)| u32::from(def.extra_power_up_activations))
        .sum()
}

/// The sources of the ordinary mana in `player`'s pool whose mana an ability of a permanent on the battlefield tells apart
/// (CR 106.3, `CardDef::mana_told_apart`): mana its payer chooses to spend or keep (CR 601.2h, 118.3a). Asked only while
/// that mana came from more than one source, since one source's mana is all alike: a pool of one land's mana searches
/// nothing. Restricted mana is told apart by its restriction alone (CR 106.6); none in content is told apart by source too.
pub fn mana_told_apart(q: &dyn Query, cards: &'static CardTable, player: PlayerId) -> Vec<ObjectId> {
    let s = q.state();
    let mut from: Vec<ObjectId> = Vec::new();
    for e in s.mana_from_of(player) {
        if e.restriction.is_none() && !from.contains(&e.source) {
            from.push(e.source);
        }
    }
    if from.len() < 2 {
        return Vec::new();
    }
    let rules: Vec<(ObjectId, PlayerId, &'static CardDef)> = s
        .battlefield()
        .iter()
        .filter_map(|&id| {
            let obj = s.obj(id);
            let def = cards.get(cards.current(obj));
            // A permanent that has lost all its abilities has none of these rules (CR 613.1f).
            (!def.mana_told_apart.is_empty() && !q.lost_all_abilities(id)).then_some((id, obj.controller, def))
        })
        .collect();
    if rules.is_empty() {
        return Vec::new();
    }
    from.retain(|&source| {
        rules.iter().any(|&(id, controller, def)| def.mana_told_apart.iter().any(|rule| rule(q, &Asker { source: id, controller, by: None, spell: false }, source)))
    });
    from
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
    for (id, controller, def) in sources(cx, |d| !d.cast_rules.is_empty()) {
        let asker = Asker { source: id, controller, by: None, spell: false };
        for from in ZoneKinds::CAST_FROM {
            if !barred.contains(from) && def.cast_rules.iter().any(|rule| !rule(cx, &asker, player, from)) {
                barred = barred.with(from);
            }
        }
    }
    barred
}

/// The permanents on the battlefield with a static ability that forbids casting spells with certain qualities
/// (`CardDef::spell_rules`, CR 101.2, 601.3), with their controllers: empty, so unallocated, in nearly every game.
pub fn spell_forbidders(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef)> {
    sources(cx, |d| !d.spell_rules.is_empty()).collect()
}

/// Whether one of `forbidders` (`spell_forbidders`) forbids `player` to cast `spell`, read through `q`: the card seen as the
/// face it would be cast as (CR 601.3e), or the spell on the stack once its proposal is complete (CR 601.6).
pub fn spell_forbidden(q: &dyn Query, forbidders: &[(ObjectId, PlayerId, &'static CardDef)], player: PlayerId, spell: ObjectId) -> bool {
    forbidders.iter().any(|&(id, controller, def)| def.spell_rules.iter().any(|rule| !rule(q, &Asker { source: id, controller, by: None, spell: false }, player, spell)))
}

/// The permanents on the battlefield with a static ability that keeps cards in some zones from entering the battlefield
/// (`CardDef::enter_rules`, CR 101.2), with their controllers: empty, so unallocated, in nearly every game.
pub fn enter_bans(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef)> {
    sources(cx, |d| !d.enter_rules.is_empty()).collect()
}

/// Whether one of `bans` (`enter_bans`) keeps `card` from being put onto the battlefield from where it is (CR 101.2): the card
/// judged as it exists there (`EnterRule`). A permanent spell resolving is put there from the stack, which no such ability
/// names, so it is not asked.
pub fn enter_banned(q: &dyn Query, bans: &[(ObjectId, PlayerId, &'static CardDef)], card: ObjectId) -> bool {
    !q.obj(card).zone.is(ZoneKind::Stack)
        && bans.iter().any(|&(id, controller, def)| def.enter_rules.iter().any(|rule| !rule(q, &Asker { source: id, controller, by: None, spell: false }, card)))
}

/// The permanents on the battlefield with a static ability that keeps some land plays from being made: one that keeps cards
/// from entering the battlefield from where they are (`CardDef::enter_rules`, as playing a land puts it there, CR 305.1) or
/// one that forbids playing some lands (`CardDef::land_play_rules`, CR 101.2), with their controllers. One look at the
/// battlefield for both, made only where a land could be played; empty, so unallocated, in nearly every game.
pub fn land_play_bans(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef)> {
    sources(cx, |d| !d.enter_rules.is_empty() || !d.land_play_rules.is_empty()).collect()
}

/// Whether one of `bans` (`land_play_bans`) forbids `player` to play `land`, where it is now, as `face` (`LandPlayRule`, CR
/// 101.2, 305.1, 712.12). A restriction takes precedence over whatever allows the play (CR 101.2). Whether the land could
/// enter the battlefield from there is `enter_banned`'s, over the same list.
pub fn land_play_forbidden(q: &dyn Query, bans: &[(ObjectId, PlayerId, &'static CardDef)], player: PlayerId, land: ObjectId, face: CardId) -> bool {
    bans.iter().any(|&(id, controller, def)| {
        def.land_play_rules.iter().any(|rule| !rule(q, &Asker { source: id, controller, by: None, spell: false }, player, land, face))
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
    for (id, controller, def) in sources(cx, |d| !d.unpreventable_damage.is_empty()) {
        let asker = Asker { source: id, controller, by: None, spell: false };
        if def.unpreventable_damage.iter().any(|rule| rule(cx, &asker, source, combat)) {
            return true;
        }
    }
    false
}

/// Which of the `damaged` permanents keep their damage through this cleanup step (CR 514.2): those a static ability of a
/// permanent on the battlefield says so of (`CardDef::damage_stays`), while that permanent has its abilities (CR 613.1f).
/// Asked only in a cleanup step with damage marked, so the battlefield is searched then.
pub fn damage_stays(cx: &Cx, damaged: &[ObjectId]) -> Vec<ObjectId> {
    let rules: Vec<(ObjectId, PlayerId, &'static CardDef)> = sources(cx, |d| !d.damage_stays.is_empty()).collect();
    if rules.is_empty() {
        return Vec::new();
    }
    damaged
        .iter()
        .copied()
        .filter(|&object| {
            rules.iter().any(|&(id, controller, def)| {
                let asker = Asker { source: id, controller, by: None, spell: false };
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
    sources(cx, |d| !d.zero_loyalty_stays.is_empty()).any(|(id, controller, def)| {
        let asker = Asker { source: id, controller, by: None, spell: false };
        def.zero_loyalty_stays.iter().any(|rule| rule(cx, &asker, planeswalker))
    })
}

/// How many creatures may attack each of `targets`, a player, planeswalker or battle, in a declaration of attackers (CR
/// 508.1c): the smallest a static ability of a permanent on the battlefield that has its abilities (CR 613.1f) says of
/// it (`CardDef::attack_limits`), with no entry for a target nothing caps. Asked as attackers are declared, only of the
/// players, planeswalkers and battles that may be attacked then.
pub fn attack_limits(cx: &Cx, targets: &[Target]) -> Vec<(Target, u32)> {
    let rules: Vec<(ObjectId, PlayerId, &'static CardDef)> = sources(cx, |d| !d.attack_limits.is_empty()).collect();
    if rules.is_empty() {
        return Vec::new();
    }
    targets
        .iter()
        .filter_map(|&target| {
            let mut limit = None;
            for &(id, controller, def) in &rules {
                let asker = Asker { source: id, controller, by: None, spell: false };
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
    sources(cx, |d| !d.legend_rule_exempt.is_empty()).any(|(id, controller, def)| {
        let asker = Asker { source: id, controller, by: None, spell: false };
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
        let def = cards.get(cards.current(obj));
        !def.cant_gain_life.is_empty() && !q.lost_all_abilities(id) && {
            let asker = Asker { source: id, controller: obj.controller, by: None, spell: false };
            def.cant_gain_life.iter().any(|rule| rule(q, &asker, player))
        }
    })
}

/// Whether `player` can't lose life (CR 119.8): a static ability of a permanent on the battlefield with its abilities says so
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
        let def = cards.get(cards.current(obj));
        !def.attacks_despite_defender.is_empty() && !q.lost_all_abilities(id) && {
            let asker = Asker { source: id, controller: obj.controller, by: None, spell: false };
            def.attacks_despite_defender.iter().any(|rule| rule(q, &asker, creature))
        }
    })
}

/// What each attack open to the active player's creatures costs to declare (CR 508.1d, 508.1h): for each of `eligible`
/// and each of `targets`, every static ability of a permanent on the battlefield that makes that attack cost something
/// (`CardDef::attack_costs`: Propaganda's "creatures can't attack you unless their controller pays {2} for each creature
/// they control that's attacking you"), added up, with the first such permanent named as the cost's source. Attacks that
/// cost nothing have no entry. A board with no such permanent, nearly every board, makes no list.
pub fn attack_costs(cx: &Cx, eligible: &[ObjectId], targets: &[Target]) -> Vec<AttackCost> {
    let taxes: Vec<(ObjectId, PlayerId, &'static CardDef)> = sources(cx, |d| !d.attack_costs.is_empty()).collect();
    if taxes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for &attacker in eligible {
        for &target in targets {
            let mut total = ManaCost::ZERO;
            let mut first = None;
            for &(id, controller, def) in &taxes {
                let asker = Asker { source: id, controller, by: None, spell: false };
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
pub fn spend_any_color_always(cx: &Cx, spenders: &[(ObjectId, PlayerId, &'static CardDef)], player: PlayerId) -> bool {
    lets_spend_any_color(cx, spenders, &Spending { player, source: None, purpose: SpendingFor::Other })
}

/// The permanents whose static abilities may let a player spend mana as though it were mana of any color (CR 609.4b): found
/// from the battlefield, and empty (so unallocated) in nearly every game. Asked once, then `lets_spend_any_color` per payment.
pub fn any_color_spenders(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef)> {
    sources(cx, |d| !d.spend_any_color.is_empty()).collect()
}

/// Whether one of the `spenders` (from `any_color_spenders`) lets the payment `spending` be made with mana spent as though it
/// were mana of any color (CR 609.4b). `q` is what the rule reads the payment's source through: a view of the face a card
/// would be cast as, where its castability is judged as a face other than the card itself (CR 712.11c, 709.3a, 715.4).
pub fn lets_spend_any_color(q: &dyn Query, spenders: &[(ObjectId, PlayerId, &'static CardDef)], spending: &Spending) -> bool {
    spenders
        .iter()
        .any(|&(id, controller, def)| def.spend_any_color.iter().any(|rule| rule(q, &Asker { source: id, controller, by: None, spell: false }, spending)))
}

/// Whether `player` may activate the abilities of `creature` as though it had haste (CR 302.6): a static ability of a
/// permanent `player` controls says so.
pub fn activates_as_though_haste(cx: &Cx, player: PlayerId, creature: ObjectId) -> bool {
    sources(cx, |d| !d.activate_as_though_haste.is_empty()).any(|(id, controller, def)| {
        controller == player && def.activate_as_though_haste.iter().any(|rule| rule(cx, &Asker { source: id, controller, by: None, spell: false }, creature))
    })
}

/// What `creature` assigns as combat damage (CR 510.1a): its power, or its toughness if a static ability says so.
/// An "as though its power were positive" rule (CR 609.4) uses negative power's magnitude only when assigning by power;
/// every other procedure still reads its actual negative power (CR 107.1b).
pub fn combat_damage_of(cx: &Cx, creature: ObjectId) -> u16 {
    let c = cx.chars(creature);
    let by_toughness = sources(cx, |d| !d.damage_by_toughness.is_empty()).any(|(id, controller, def)| {
        let asker = Asker { source: id, controller, by: None, spell: false };
        def.damage_by_toughness.iter().any(|rule| rule(cx, &asker, creature))
    });
    if by_toughness {
        return c.toughness.unwrap_or(0).max(0) as u16;
    }
    let power = c.power.unwrap_or(0);
    if power < 0 && sources(cx, |d| !d.damage_by_absolute_power.is_empty()).any(|(id, controller, def)| {
        let asker = Asker { source: id, controller, by: None, spell: false };
        def.damage_by_absolute_power.iter().any(|rule| rule(cx, &asker, creature))
    }) {
        return power.unsigned_abs();
    }
    power.max(0) as u16
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
pub fn flash_granters(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef)> {
    sources(cx, |d| !d.flash_rules.is_empty()).collect()
}

/// Whether one of the `granters` (from `flash_granters`) lets `player` cast `spell` as though it had flash (CR 307.5, 601.3b,
/// 702.8), asked of the spell it would be on the stack under `player` (CR 601.3e, `actions::SpellsAsCast`).
pub fn grants_flash(q: &dyn Query, granters: &[(ObjectId, PlayerId, &'static CardDef)], player: PlayerId, spell: ObjectId) -> bool {
    granters.iter().any(|&(id, controller, def)| {
        let asker = Asker { source: id, controller, by: None, spell: false };
        def.flash_rules.iter().any(|rule| rule(q, &asker, spell, player))
    })
}

/// The permanents on the battlefield with a static ability that lets a player pay another cost rather than an activated
/// ability's mana cost (`CardDef::ability_alternatives`, CR 118.9, 602.2b), with their controllers: empty, so unallocated,
/// in nearly every game.
pub fn activation_alternative_sources(cx: &Cx) -> Vec<(ObjectId, PlayerId, &'static CardDef)> {
    sources(cx, |d| !d.ability_alternatives.is_empty()).collect()
}

/// Each cost one of `alternative_sources` lets `player` pay rather than the mana cost of this ability of `permanent`
/// (CR 118.9, 602.2b), with the `Grant` that names it (the permanent and `AbilityRef::ability_alternative`): each a way of
/// activating it of its own.
pub fn activation_alternatives(
    cx: &Cx,
    alternative_sources: &[(ObjectId, PlayerId, &'static CardDef)],
    player: PlayerId,
    permanent: ObjectId,
    def: &ActivatedDef,
) -> Vec<(Grant, ManaCost)> {
    let mut all = Vec::new();
    for &(id, controller, card) in alternative_sources {
        let asker = Asker { source: id, controller, by: None, spell: false };
        let card_id = cx.cards().current(cx.s().obj(id));
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
    rule(q, &Asker { source: grant.object, controller: obj.controller, by: None, spell: false }, player, permanent, def)
}

/// Whether a static ability lets `player` activate this sorcery-speed ability whenever they could cast an instant.
pub fn instant_activation(cx: &Cx, player: PlayerId, source: ObjectId, index: u8) -> bool {
    for (id, controller, def) in sources(cx, |d| !d.instant_activation.is_empty()) {
        let asker = Asker { source: id, controller, by: None, spell: false };
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
            cx.cards().activation_rules[cx.cards().current(obj).0 as usize] && !cx.lost_all_abilities(id)
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
        let asker = Asker { source: id, controller: obj.controller, by: None, spell: false };
        cx.card(cx.cards().current(obj)).activation_rules.iter().any(|rule| !rule(cx, &asker, source, def))
    })
}

/// Whether the static abilities of the permanents on the battlefield let `creature` attack (CR 508.1c): each attack rule of
/// each permanent that has not lost its abilities (CR 613.1f), asked of the creature with the permanent as its source
/// ("this creature can't attack unless ...", Patchwork Beastie).
pub fn attack_rules_allow(q: &dyn Query, cards: &CardTable, creature: ObjectId) -> bool {
    q.state().battlefield().iter().all(|&id| {
        let card = cards.current(q.state().obj(id));
        !cards.attack_rules[card.0 as usize] || q.lost_all_abilities(id) || cards.get(card).attack_rules.iter().all(|rule| rule(q, id, creature))
    })
}

/// The requirements to block that the static abilities of the permanents on the battlefield with their abilities (CR 613.1f)
/// put on the blocks among `allowed` (CR 509.1c, `CardDef::block_requirements`: Lure's "all creatures able to block enchanted
/// creature do so"): one for each permanent and each block it requires, so two Lures on a creature make two of each creature
/// able to block it. A board with no such permanent, nearly every board, makes none.
pub fn block_requirements(cx: &Cx, allowed: &[(ObjectId, ObjectId)]) -> Vec<RequiredBlock> {
    let rules: Vec<(ObjectId, PlayerId, &'static CardDef)> = sources(cx, |d| !d.block_requirements.is_empty()).collect();
    let mut required = Vec::new();
    for &(id, _, def) in &rules {
        for &(blocker, attacker) in allowed {
            if def.block_requirements.iter().any(|rule| rule(cx, id, blocker, attacker)) {
                required.push(RequiredBlock { blocker: Some(blocker), attacker: Some(attacker) });
            }
        }
    }
    required
}

/// Whether a static ability of a permanent on the battlefield with its abilities (CR 613.1f) says `test` of its own rules.
/// Asked only where the rules need the answer - a player about to lose or win, damage to a player, counters about to be put -
/// so the battlefield is searched then.
fn any_static<R: Copy + 'static>(q: &dyn Query, cards: &'static CardTable, rules: fn(&CardDef) -> &'static [R], test: impl Fn(R, &Asker) -> bool) -> bool {
    let s = q.state();
    s.battlefield().iter().any(|&id| {
        let obj = s.obj(id);
        let list = rules(cards.get(cards.current(obj)));
        !list.is_empty() && !q.lost_all_abilities(id) && {
            let asker = Asker { source: id, controller: obj.controller, by: None, spell: false };
            list.iter().any(|&rule| test(rule, &asker))
        }
    })
}

/// Whether `player` doesn't lose the game for `reason` (CR 104.3, 704.5a-c): a static ability says so (`CardDef::loss_rules`:
/// Platinum Angel's "you can't lose the game", Phyrexian Unlife's "you don't lose the game for having 0 or less life").
pub fn doesnt_lose(q: &dyn Query, cards: &'static CardTable, player: PlayerId, reason: LossReason) -> bool {
    any_static(q, cards, |d| d.loss_rules, |rule, asker| rule(q, asker, player, reason))
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
    state.battlefield().iter().any(|&id| !cards.get(cards.current(state.obj(id))).counter_bans.is_empty())
}

/// Whether `player` can't get `kind` counters (CR 122.1, 101.2): Melira's "you can't get poison counters"
/// (`CardDef::player_counter_bans`).
pub fn player_counters_banned(q: &dyn Query, cards: &'static CardTable, player: PlayerId, kind: PlayerCounter) -> bool {
    any_static(q, cards, |d| d.player_counter_bans, |rule, asker| rule(q, asker, player, kind))
}

/// Whether a static ability forbids `player` to make `payment` for a cost paid for `paid_for` (CR 101.2, 118.3, 601.2h,
/// 602.2b, `CardDef::cost_bans`): Yasharn, Implacable Earth's "players can't pay life or sacrifice nonland permanents to cast
/// spells or activate abilities". Asked only of a cost that pays life or sacrifices a permanent, as it is cast or activated.
pub fn cost_banned(q: &dyn Query, cards: &'static CardTable, player: PlayerId, paid_for: CostFor, payment: CostPayment) -> bool {
    any_static(q, cards, |d| d.cost_bans, |rule, asker| rule(q, asker, player, paid_for, payment))
}

/// Whether a static ability of a permanent on the battlefield says the permanent `id` doesn't untap during its controller's
/// untap step (CR 502.3, `CardDef::untap_bans`: Intruder Alarm, Infinite Coursework). No ability of `id`'s, so it holds
/// whatever abilities `id` has lost (CR 613.1f); its own "doesn't untap" is `Keyword::DoesNotUntap`.
pub fn untap_banned(q: &dyn Query, cards: &'static CardTable, id: ObjectId) -> bool {
    any_static(q, cards, |d| d.untap_bans, |rule, asker| rule(q, asker, id))
}

/// Whether a static ability of a permanent on the battlefield with its abilities says the permanent `id` can't become
/// untapped (CR 101.2, 701.26b, `CardDef::cant_become_untapped`: Frozen in Ice, Blossombind). No ability of `id`'s, so it holds
/// whatever abilities `id` has lost (CR 613.1f). Asked only where a tapped permanent would untap.
pub fn cant_become_untapped(q: &dyn Query, cards: &'static CardTable, id: ObjectId) -> bool {
    any_static(q, cards, |d| d.cant_become_untapped, |rule, asker| rule(q, asker, id))
}

/// Whether the permanent `id` can't be sacrificed (CR 101.2, 701.21a), for any reason and by anyone: it has "this permanent
/// can't be sacrificed" (`Keyword::CantBeSacrificed`, printed or granted, so losing its abilities ends it), a resolved effect
/// says so (`EffectKind::CantBeSacrificed`, which it keeps whatever abilities it loses), or a static ability of a permanent on
/// the battlefield does (`CardDef::sacrifice_bans`: Assault Suit's equipped creature). Asked only where a sacrifice would be
/// made, chosen or paid.
pub fn cant_be_sacrificed(q: &dyn Query, cards: &'static CardTable, id: ObjectId) -> bool {
    q.chars(id).has(Keyword::CantBeSacrificed)
        || q.state().effects.iter().any(|e| e.affected == id && e.kind == EffectKind::CantBeSacrificed)
        || any_static(q, cards, |d| d.sacrifice_bans, |rule, asker| rule(q, asker, id))
}
