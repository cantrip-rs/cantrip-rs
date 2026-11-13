//! Replacement effects (CR 614) many cards print.

use cantrip_api::*;

fn add_a_finality_counter(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::Finality as usize] += 1;
    }
}

/// "That creature enters with a finality counter" (CR 614.1c, 122.6): a replacement effect about a spell, applied as it
/// enters the battlefield from the stack (`next_creature_counter::spell_enters`); Leonardo, Sewer Samurai and Overflow
/// Submatrix.
pub const SPELL_ENTERS_WITH_FINALITY: GrantedReplacementDef = granted_replacement(
    ReplacementKind::ZoneChange,
    crate::next_creature_counter::spell_enters,
    add_a_finality_counter,
    "enters with a finality counter on it",
);

/// "If one or more [counters of this kind] would be put on a [permanent of these types] you control, that many plus one
/// [of that kind] are put on it instead" (CR 122.6, 614.1a): whether it replaces this event. Keyed to the recipient, never
/// to who puts the counters; a permanent entering with counters is judged as it would exist on the battlefield (CR 614.12).
/// Michelangelo, Weirdness to 11 (creatures), Ozolith, the Shattered Spire (artifacts and creatures) and Optimization Annex
/// (charge counters on artifacts) print it.
pub fn plus_one_counters_applies(
    q: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
    kind: CounterKind,
    types: &[CardType],
) -> bool {
    let Proposal::Counters(c) = proposal else { return false };
    c.controller == asker.controller
        && c.counts[kind as usize] > 0
        && (c.entering || q.is_on_battlefield(c.object))
        && types.iter().any(|&t| q.chars(c.object).is(t))
}

/// "If you would put one or more counters on a permanent or player, put twice that many of each of those kinds of
/// counters on that permanent or player instead" (CR 122.6, 614.1a): whether it replaces this event. Keyed to who puts
/// the counters, never to the recipient. Innkeeper's Talent (level 3) and Vorinclex, Monstrous Raider print it.
pub fn double_counters_applies(proposal: &Proposal, asker: &Asker) -> bool {
    match proposal {
        Proposal::Counters(c) => c.putter == asker.controller && c.counts.iter().any(|&n| n > 0),
        Proposal::PlayerCounters(c) => c.putter == asker.controller && (c.poison > 0 || c.energy > 0 || c.experience > 0),
        _ => false,
    }
}

/// ... twice that many of each kind.
pub fn double_counters_apply(proposal: &mut Proposal) {
    match proposal {
        Proposal::Counters(c) => {
            for n in c.counts.iter_mut().filter(|n| **n > 0) {
                *n *= 2;
            }
        }
        Proposal::PlayerCounters(c) => {
            c.poison = c.poison.saturating_mul(2);
            c.energy = c.energy.saturating_mul(2);
            c.experience = c.experience.saturating_mul(2);
        }
        _ => {}
    }
}

/// "... that many plus one": one more counter of that kind. Each such effect applies once, so a second adds another.
pub fn plus_one_counters_apply(proposal: &mut Proposal, kind: CounterKind) {
    if let Proposal::Counters(c) = proposal {
        c.counts[kind as usize] += 1;
    }
}

/// "... that many plus one of each of those kinds of counters": one more of each kind being put, and none of a kind that
/// wasn't (CR 122.6, 614.1a). Each such effect applies once (CR 614.5), so a second adds another. Doc Samson, Super
/// Psychiatrist and Brad Boimler, Eager Ensign print it.
pub fn plus_one_of_each_kind_apply(proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        for n in c.counts.iter_mut().filter(|n| **n > 0) {
            *n += 1;
        }
    }
}

/// "If one or more tokens would be created under your control" (CR 111.1, 614.1a): whose they would be, not who makes
/// them. The condition of `DOUBLE_TOKENS`, Holo-Engineer and Quina, Qu Gourmet.
pub fn tokens_under_your_control(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::TokenCreation(t) if t.controller == asker.controller && t.count > 0)
}

/// "... twice that many of those tokens are created instead."
fn twice_that_many_tokens(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::TokenCreation(t) = proposal {
        t.count = t.count.saturating_mul(2);
    }
}

fn plus_one_counters_on_your_creatures(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    plus_one_counters_applies(q, asker, proposal, CounterKind::PlusOne, &[CardType::Creature])
}

/// "... that many plus one +1/+1 counters are put on it instead" (CR 122.6, 614.1a): one more +1/+1 counter, as a replacement's
/// apply. Hardened Scales (through `PLUS_ONE_COUNTERS_ON_YOUR_CREATURES`), Ozolith, the Shattered Spire and Caradora, Heart
/// of Alacria print it.
pub fn plus_one_counters_apply_to_proposal(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    plus_one_counters_apply(proposal, CounterKind::PlusOne);
}

/// "If one or more +1/+1 counters would be put on a creature you control, that many plus one +1/+1 counters are put on
/// it instead." Hardened Scales, Michelangelo, Weirdness to 11 and Yoshimaru, Beloved Companion print it. Each effect
/// applies once (CR 614.5), so each adds one.
pub const PLUS_ONE_COUNTERS_ON_YOUR_CREATURES: ReplacementDef = replacement(
    ReplacementKind::Counters,
    plus_one_counters_on_your_creatures,
    plus_one_counters_apply_to_proposal,
    "puts that many plus one +1/+1 counters instead",
);

/// "... twice that many +1/+1 counters are put on it instead": the +1/+1 counters alone; counters of other kinds put with
/// them are left as they are.
fn twice_that_many_plus_one_counters(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        c.counts[CounterKind::PlusOne as usize] *= 2;
    }
}

/// "If one or more +1/+1 counters would be put on a creature you control, twice that many +1/+1 counters are put on it
/// [that creature] instead" (CR 122.6, 614.1a): on a creature its controller controls, or one entering under them with
/// counters, whoever puts them. Each applies once (CR 614.5), so two make four times as many. Corpsejack Menace and The
/// Earth Crystal print it.
pub const DOUBLE_PLUS_ONE_COUNTERS_ON_YOUR_CREATURES: ReplacementDef = replacement(
    ReplacementKind::Counters,
    plus_one_counters_on_your_creatures,
    twice_that_many_plus_one_counters,
    "puts twice that many +1/+1 counters instead",
);

/// "If one or more tokens would be created under your control, twice that many of those tokens are created instead."
/// Elspeth, Storm Slayer and Exalted Sunborn print it.
pub const DOUBLE_TOKENS: ReplacementDef =
    replacement(ReplacementKind::TokenCreation, tokens_under_your_control, twice_that_many_tokens, "twice that many tokens are created");

/// "As long as this permanent is attached to a permanent, the first time you would create one or more tokens each turn"
/// (CR 111.1, 614.1a): tokens that are yours, this turn's first creation. What was created earlier this turn is read from the
/// history (a token created as a copy of a permanent spell is not created, CR 111.13), and a creation the player declined to
/// replace still used the turn's first time up. Mirrormind Crown (equipped creature) and Moonlit Meditation (enchanted permanent).
pub fn first_tokens_you_would_create_this_turn_while_attached(q: &dyn Query, me: &Asker, p: &Proposal) -> bool {
    if !matches!(p, Proposal::TokenCreation(t) if t.controller == me.controller) || q.attached_to(me.source).is_none() {
        return false;
    }
    !q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).to(ZoneKind::Battlefield).reason(ZoneReason::Created).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { new, .. } if q.obj(new).owner == me.controller))
}

/// "... you may instead create that many tokens that are copies of the permanent it is attached to": the same count, each a copy
/// of that permanent's copiable values (CR 707.2) - a token's own characteristics if it is one, what it copies if it is a copy.
pub fn copies_of_the_attached_permanent(q: &dyn Query, me: &Asker, p: &mut Proposal) {
    let (Proposal::TokenCreation(t), Some(permanent)) = (p, q.attached_to(me.source)) else { return };
    let values = q.copiable_values(permanent);
    t.token = values.card;
    t.copy = Some(values);
}

fn buyback_applies(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.object == asker.source
            && z.reason == ZoneReason::Resolve
            && z.to.is(ZoneKind::Graveyard)
            && q.obj(asker.source).kicked > 0)
}

fn buyback_apply(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::hand(q.obj(z.object).owner);
    }
}

/// Buyback's second half (CR 702.27a): "If the buyback cost was paid, put this spell into its owner's hand instead of
/// into that player's graveyard as it resolves." The cost is the card's `kicker`; only the resolution move is
/// replaced, so a countered spell goes to the graveyard. Corpse Dance, Capsize and Lava Burst print buyback.
pub const BUYBACK: ReplacementDef = replacement(ReplacementKind::ZoneChange, buyback_applies, buyback_apply, "returned to its owner's hand by buyback")
    .own_everywhere();

/// "... exile it instead" (CR 614.1a) as the `apply` of a permanent's `replacement(ReplacementKind::ZoneChange, ..)`: the
/// object goes to the exile zone of the player whose zone it was headed for, in place of the zone it would have gone to.
/// Rest in Peace and Garruk, Veiled Butcher print it.
pub fn exile_instead_apply(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::exile(z.to.player);
    }
}

/// "... exile it instead" by a permanent that has an ability referring to the cards "exiled with" it, as the `apply` of a
/// replacement it prints: `exile_instead_apply`, and the card is exiled with the permanent (CR 607.2b, 406.6;
/// `ZoneChange::exiled_with`) by the text the effect is printed in, as the object it is now, so a permanent that leaves and
/// returns finds none of them (CR 400.7). Valgavoth, Terror Eater, Gisa, Glorious Resurrector and Void Maw print it.
pub fn exile_instead_linked_apply(q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    exile_instead_apply(q, asker, proposal);
    if let Proposal::ZoneChange(z) = proposal {
        z.exiled_with = Some(q.linked_to(asker));
    }
}

/// "If a card would be put into an opponent's graveyard from anywhere" (CR 614.1a, 400.3): a card, not a token or a copy of a
/// spell (CR 108.2), going to the graveyard of a player who is an opponent of the effect's controller, from any zone - a
/// spell resolving or countered, a creature dying, a discard, a mill. "An opponent's graveyard" is the destination's owner,
/// whoever controlled the card. Leyline of the Void, Dauthi Voidwalker and Valgavoth, Terror Eater print it.
pub fn card_to_an_opponents_graveyard(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.is_card
            && z.to.is(ZoneKind::Graveyard)
            && q.state().opponents(asker.controller).any(|opponent| opponent == z.to.player))
}

/// "If a nontoken creature an opponent controls would die" (CR 614.1a, 700.4, 111.1): `opponents_creature_would_die`, of a
/// creature that is not a token. Kalitas, Traitor of Ghet, The Darkness Crystal, Draugr Necromancer and Valentin, Dean of the
/// Vein print it.
pub fn opponents_nontoken_creature_would_die(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    opponents_creature_would_die(q, asker, proposal) && matches!(proposal, Proposal::ZoneChange(z) if !q.obj(z.object).token)
}

/// "If a creature an opponent controls would die" (CR 614.1a, 700.4): a creature controlled by an opponent of this
/// effect's controller, moving from the battlefield to a graveyard. A token is a creature too. Garruk, Vren and Malicious
/// Eclipse (for the turn, as a granted replacement) read it.
pub fn opponents_creature_would_die(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.from.is(ZoneKind::Battlefield)
            && z.to.is(ZoneKind::Graveyard)
            && q.chars(z.object).is(CardType::Creature)
            && q.state().opponents(asker.controller).any(|o| o == q.controller(z.object)))
}

/// "If a creature an opponent controls would die, exile it instead." as a permanent's static replacement effect.
/// Garruk, Veiled Butcher and Vren, the Relentless print it.
pub const OPPONENTS_CREATURES_EXILED_INSTEAD_OF_DYING: ReplacementDef = replacement(
    ReplacementKind::ZoneChange,
    opponents_creature_would_die,
    exile_instead_apply,
    "exiles a creature an opponent controls instead of letting it die",
);

fn exile_instead_of_dying_applies(_: &dyn Query, _: &Asker, affected: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.object == affected && z.from.is(ZoneKind::Battlefield) && z.to.is(ZoneKind::Graveyard))
}

fn creature_exile_instead_of_dying_applies(q: &dyn Query, asker: &Asker, affected: ObjectId, proposal: &Proposal) -> bool {
    exile_instead_of_dying_applies(q, asker, affected, proposal) && q.chars(affected).is(CardType::Creature)
}

/// `exile_instead_apply` as the `apply` of a `granted_replacement`, which is also told the object it is about. Suplex, Mawloc
/// and Yawgmoth's Will's resolved effects print it.
pub fn exile_instead_apply_granted(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    exile_instead_apply(q, asker, proposal);
}

/// "If that creature would die this turn, exile it instead" (CR 614.1a, 700.4, 611.2c): a replacement effect the spell or
/// ability makes about one object for the turn, not an ability the creature gains. "Dies" means a move from the battlefield
/// to a graveyard (CR 700.4); the bound object remains covered after losing its creature type, but not when returned to hand.
/// List it among a card's `granted_replacements`
/// and make the effect with `exile_if_it_would_die` before anything can kill the creature. Suplex and Mawloc print it.
pub const EXILE_INSTEAD_OF_DYING: GrantedReplacementDef =
    granted_replacement(ReplacementKind::ZoneChange, exile_instead_of_dying_applies, exile_instead_apply_granted, "exiled instead of dying");

/// "If a creature dealt damage this way would die this turn, exile it instead" (CR 614.1a, 700.4): the caller records the
/// actual damaged permanent, and this open creature restriction is checked when it would die. Anger of the Gods,
/// Red Sun's Zenith and Yamabushi's Flame print it. A damaged planeswalker that becomes a creature can qualify later.
pub const CREATURE_EXILED_INSTEAD_OF_DYING: GrantedReplacementDef =
    granted_replacement(ReplacementKind::ZoneChange, creature_exile_instead_of_dying_applies, exile_instead_apply_granted, "exiles a damaged creature instead of letting it die");

/// Makes the caller's granted exile replacement (the resolving spell or ability's card's replacement number `index`) about `creature` until
/// end of turn. It names the object, so it does not follow the creature if it leaves and returns as a new object (CR 400.7).
pub fn exile_if_it_would_die(g: &mut dyn Game, item: &StackItem, index: u8, creature: ObjectId) {
    g.add_effect(item.object, creature, EffectKind::Replacement { card: item.ability.card, index }, Duration::EndOfTurn);
}

/// Which permanents `opponents_enter_tapped` taps: a bit set of these.
pub mod enter_tapped {
    pub const CREATURES: u8 = 1;
    pub const ARTIFACTS: u8 = 2;
    pub const NONBASIC_LANDS: u8 = 4;
    pub const LANDS: u8 = 8;
}

/// Whether `proposal` is a permanent of one of the `WHICH` kinds entering the battlefield untapped, judged as it would exist
/// there (CR 614.12); the part of the condition `opponents_enter_tapped` and `permanents_enter_tapped` share.
fn untapped_entry_of_kind<const WHICH: u8>(q: &dyn Query, proposal: &Proposal) -> Option<PlayerId> {
    let Proposal::ZoneChange(z) = proposal else { return None };
    let c = q.chars(z.object);
    let kind = (WHICH & enter_tapped::CREATURES != 0 && c.is(CardType::Creature))
        || (WHICH & enter_tapped::ARTIFACTS != 0 && c.is(CardType::Artifact))
        || (WHICH & enter_tapped::NONBASIC_LANDS != 0 && c.is(CardType::Land) && !c.supertypes.basic)
        || (WHICH & enter_tapped::LANDS != 0 && c.is(CardType::Land));
    (z.to.is(ZoneKind::Battlefield) && !z.tapped && kind).then_some(z.controller)
}

fn opponents_enter_tapped_applies<const WHICH: u8>(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    untapped_entry_of_kind::<WHICH>(q, proposal).is_some_and(|controller| controller != asker.controller)
}

fn permanents_enter_tapped_applies<const WHICH: u8>(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    untapped_entry_of_kind::<WHICH>(q, proposal).is_some()
}

pub fn opponents_enter_tapped_apply(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.tapped = true;
    }
}

/// "[Creatures / artifacts and creatures / nonbasic lands / artifacts, creatures, and lands] your opponents control enter tapped" (CR 614.1d): the
/// object is judged as it would exist on the battlefield (CR 614.12), under the player it enters under (CR 110.2a).
/// `WHICH` is a set of `enter_tapped` bits. Authority of the Consuls (creatures), Spider-Woman, Stunning Savior
/// (artifacts and creatures), Archon of Emeria (nonbasic lands), and Kismet, Frozen Aether and Loxodon Gatekeeper
/// (artifacts, creatures, and lands) print it.
pub const fn opponents_enter_tapped<const WHICH: u8>(hint: &'static str) -> ReplacementDef {
    replacement(ReplacementKind::ZoneChange, opponents_enter_tapped_applies::<WHICH>, opponents_enter_tapped_apply, hint)
}

/// "[Artifacts and lands / nonbasic lands] enter tapped" (CR 614.1d), whoever controls them: `opponents_enter_tapped` without
/// the opponents filter, the object judged as it would exist on the battlefield (CR 614.12). `WHICH` is a set of
/// `enter_tapped` bits. Root Maze (artifacts and lands) and Zhao, the Moon Slayer (nonbasic lands) print it.
pub const fn permanents_enter_tapped<const WHICH: u8>(hint: &'static str) -> ReplacementDef {
    replacement(ReplacementKind::ZoneChange, permanents_enter_tapped_applies::<WHICH>, opponents_enter_tapped_apply, hint)
}

/// "If you would gain life" (CR 119.10, 614.1a): a gain of life by the effect's controller. Zero is no gain.
fn you_would_gain_life(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::LifeChange(l) if l.player == asker.controller && l.amount > 0)
}

fn twice_that_much_life(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::LifeChange(l) = proposal {
        l.amount = l.amount.saturating_mul(2);
    }
}

fn that_much_life_plus_one(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::LifeChange(l) = proposal {
        l.amount = l.amount.saturating_add(1);
    }
}

/// "If you would draw a card while your library has no cards in it": one draw (CR 121.1), judged before any card is moved,
/// with nothing left to draw (CR 121.6a).
fn you_would_draw_from_nothing(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Draw(d) if d.player == asker.controller && q.state().zone(Zone::library(d.player)).is_empty())
}

/// "... instead": the card is not drawn (CR 614.6), what the effect does in its place being its `then`. The `apply` of every
/// replacement effect that replaces one draw (CR 121.6): Laboratory Maniac's, Notion Thief's, Thought Reflection's, dredge's.
pub fn instead_of_drawing(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Draw(d) = proposal {
        d.replaced = true;
    }
}

/// "If you would draw a card" (CR 121.6, 614.1a): one draw of the effect's controller, judged as it is about to be made
/// (CR 121.2), so each draw of an instruction to draw several is replaced on its own; an empty library is no matter
/// (CR 121.6a). Thought Reflection, Underrealm Lich and Abundance print it.
pub fn you_would_draw_a_card(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Draw(d) if d.player == asker.controller)
}

/// "If you would draw a card, draw two cards instead." (CR 121.6, 614.1a) Thought Reflection and Mr. Spock, Science Officer print this.
/// The two resulting draws are each considered by other draw replacements (CR 614.5), but not by this effect again;
/// a second copy of this effect can therefore turn one draw into four.
pub const DRAW_TWO_INSTEAD: ReplacementDef =
    replacement(ReplacementKind::Draw, you_would_draw_a_card, instead_of_drawing, "draws two cards instead").then(draw_two);

/// "... draw two cards instead": two draws, each meeting the replacement effects that have not yet had their opportunity at the
/// draw this replaced (CR 614.5), so two such effects make four.
fn draw_two(g: &mut dyn Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    g.draw(asker.controller, 2);
}

/// "If an opponent would draw a card except the first one they draw in each of their draw steps" (CR 121.6, 614.1a): one
/// draw, judged as it is about to be made (CR 121.2), by an opponent of the effect's controller, unless it would be the first
/// card that player draws in their own draw step (a draw replaced before it was not drawn, CR 614.6). Hullbreacher and
/// Notion Thief print it.
pub fn opponent_would_draw_except_first_in_draw_step(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Draw(d)
        if q.state().opponents(asker.controller).any(|o| o == d.player) && !crate::history::first_draw_in_own_draw_step(q, d.player))
}

/// "If an opponent would mill one or more cards" (CR 701.17a, 614.1a): a mill of at least one card by an opponent of the
/// effect's controller, judged once for the whole instruction before any card moves. Bruvac the Grandiloquent and The Water
/// Crystal print it.
pub fn an_opponent_would_mill(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Mill(m) if m.count >= 1 && q.state().opponents(asker.controller).any(|o| o == m.player))
}

/// "If you would draw a card except the first one you draw in each of your draw steps" (CR 121.6, 614.1a): each draw on its own,
/// judged as it is about to be made, so the turn's first card drawn in the draw step is left alone, and the one after it is not.
fn you_would_draw_except_first_in_draw_step(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    you_would_draw_a_card(q, asker, proposal) && !crate::history::first_draw_in_own_draw_step(q, asker.controller)
}

/// "If you would draw a card except the first one you draw in each of your draw steps, draw two cards instead." Alhammarret's
/// Archive and Bard, King of Dale print it (and Teferi's Ageless Insight).
pub const DRAW_TWO_INSTEAD_EXCEPT_THE_FIRST_IN_YOUR_DRAW_STEP: ReplacementDef =
    replacement(ReplacementKind::Draw, you_would_draw_except_first_in_draw_step, instead_of_drawing, "draws two cards instead").then(draw_two);

/// "You win the game instead": unless a player can't win or an opponent can't lose (Platinum Angel), when it does nothing
/// and the draw is still replaced, so nobody loses for it either (CR 104.2b, Laboratory Maniac's ruling).
fn win_instead(g: &mut dyn Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    g.win_game(asker.controller);
}

/// "If you would draw a card while your library has no cards in it, you win the game instead" (CR 614.1a, 121.6). Each draw
/// of an instruction to draw several is asked about on its own. Laboratory Maniac and Jace, Wielder of Mysteries print it.
pub const WIN_INSTEAD_OF_DRAWING_FROM_AN_EMPTY_LIBRARY: ReplacementDef =
    replacement(ReplacementKind::Draw, you_would_draw_from_nothing, instead_of_drawing, "wins the game instead of drawing from an empty library")
        .then(win_instead);

/// "If you would gain life, you gain twice that much life instead." Each applies once to each gain, so two make four
/// times as much (Boon Reflection's ruling); a life total set higher is a gain of the difference (CR 119.5), doubled.
/// Boon Reflection, Rhox Faithmender, Alhammarret's Archive and The Wind Crystal print it.
pub const GAIN_TWICE_THAT_MUCH_LIFE: ReplacementDef =
    replacement(ReplacementKind::LifeChange, you_would_gain_life, twice_that_much_life, "you gain twice that much life");

/// "If you would gain life, you gain that much life plus 1 instead." Once to each life gain event, however much it is
/// (Angel of Vitality's ruling); a second such effect adds another. Angel of Vitality, Leyline of Hope, Heron of Hope
/// and Honor Troll print it.
pub const GAIN_THAT_MUCH_LIFE_PLUS_ONE: ReplacementDef =
    replacement(ReplacementKind::LifeChange, you_would_gain_life, that_much_life_plus_one, "you gain that much life plus 1");

/// "If an opponent would gain life": a gain by a player other than the effect's controller who is their opponent.
fn opponent_would_gain_life(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::LifeChange(l)
        if l.amount > 0 && q.state().opponents(asker.controller).any(|p| p == l.player))
}

fn loses_that_much_life(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::LifeChange(l) = proposal {
        l.amount = -l.amount;
    }
}

/// "If an opponent would gain life, that player loses that much life instead." The gain becomes a loss, which the
/// effects on losing life then meet (CR 616.1f), and no effect on gaining life applies to it any more (Tainted Remedy's
/// ruling). Tainted Remedy and Plague Drone print it.
pub const OPPONENTS_LOSE_LIFE_INSTEAD_OF_GAINING: ReplacementDef =
    replacement(ReplacementKind::LifeChange, opponent_would_gain_life, loses_that_much_life, "you lose that much life instead");

/// "All damage that would be dealt to you and other permanents you control is dealt to this creature instead" (CR 614.1a,
/// 120.3): whether it replaces this damage - to its controller, or to a permanent its controller controls other than
/// itself, from any source. The damage is then dealt to it by the same source, and the effects not yet applied are judged
/// against it (CR 614.6, 616.1f): a second such creature redirects it once more, and protection of the creature prevents
/// it. Ancient Adamantoise and Palisade Giant print it.
fn damage_to_you_and_your_other_permanents(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::Damage(d) = proposal else { return false };
    d.amount > 0
        && match d.target {
            Target::Player(p) => p == asker.controller,
            Target::Object(id) => id != asker.source && q.is_on_battlefield(id) && q.controller(id) == asker.controller,
            _ => false,
        }
}

/// "... is dealt to this creature instead."
pub fn dealt_to_this_creature_instead(_: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.target = Target::Object(asker.source);
    }
}

/// See [`damage_to_you_and_your_other_permanents`].
pub const DAMAGE_TO_YOU_AND_YOUR_OTHER_PERMANENTS_IS_DEALT_TO_THIS: ReplacementDef = replacement(
    ReplacementKind::Damage,
    damage_to_you_and_your_other_permanents,
    dealt_to_this_creature_instead,
    "the damage is dealt to this creature instead",
);

/// "If a source you control would deal noncombat damage to an opponent or a permanent an opponent controls" (CR 614.1,
/// 120.3): damage of any source its controller controls (a spell or an ability on the stack, a permanent), of an amount
/// above zero (damage already prevented to nothing is no damage to add to, CR 615.6) that is not combat damage, dealt to
/// an opponent of the controller or a permanent an opponent controls. Artist's Talent (at level 3, with `plus::<2>`) and
/// Tomik, Izzet Sparkmage (with `plus::<1>`) print it.
pub fn noncombat_damage_you_deal_to_an_opponent_or_their_permanent(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if !d.combat) && damage_you_deal_to_an_opponent_or_their_permanent(q, asker, proposal)
}

/// "If a source you control would deal damage to an opponent or a permanent an opponent controls" (CR 614.1, 120.3): damage of
/// any source its controller controls, combat or not, of an amount above zero (CR 615.6), dealt to an opponent of the
/// controller or a permanent an opponent controls. Fated Firepower prints it, and the noncombat form above narrows it.
pub fn damage_you_deal_to_an_opponent_or_their_permanent(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::Damage(d) = proposal else { return false };
    d.amount > 0
        && q.controller(d.source) == asker.controller
        && match d.target {
            Target::Player(player) => q.state().opponents(asker.controller).any(|opponent| opponent == player),
            recipient => recipient.object().is_some_and(|id| q.controller(id) != asker.controller),
        }
}

/// "... it deals that much damage plus `N` instead" (CR 614.1): the damage about to be dealt is `N` greater.
pub fn plus<const N: u32>(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount = d.amount + N;
    }
}

/// "... it deals double that damage instead" / "... twice that much damage instead" (CR 614.1a, 120.3): the damage about to be
/// dealt is twice as much, still dealt by the same source to the same recipient (the rulings), once it has been divided or
/// assigned among its recipients. Two such effects make it four times as much (CR 614.5), and the player or controller of the
/// permanent it is dealt to orders it with other replacement and prevention effects (CR 616.1). Furnace of Rath, Gisela, Blade
/// of Goldnight, Trance Kuja, Collective Inferno, Mjölnir, Hammer of Thor and Neriv, Heart of the Storm print it.
pub fn double_that_damage(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount = d.amount * 2;
    }
}

/// "If [this card] would be put into a graveyard from anywhere, reveal [it] and shuffle it into its owner's library instead"
/// (CR 614.1a, 701.20a, 701.24a): whether this move is that. An ability that says where it functions functions there
/// (CR 113.6b), so the replacement is `own_everywhere`: from a hand, a library, the stack and the battlefield alike, and only
/// on the card's own move.
fn this_would_be_put_into_a_graveyard(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Graveyard) && z.object == asker.source)
}

/// "... reveal it and shuffle it into its owner's library instead."
fn reveal_and_shuffle_into_library(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        // A card only ever goes to its owner's graveyard (CR 400.3), so that player's library is its owner's.
        z.to = Zone::library(q.obj(z.object).owner);
        z.reveal = true;
        z.shuffle = true;
    }
}

/// "If [this card] would be put into a graveyard from anywhere, reveal [this card] and shuffle it into its owner's library
/// instead." Blightsteel Colossus and Progenitus print it.
pub const REVEAL_AND_SHUFFLE_INTO_LIBRARY_INSTEAD_OF_GRAVEYARD: ReplacementDef =
    replacement(ReplacementKind::ZoneChange, this_would_be_put_into_a_graveyard, reveal_and_shuffle_into_library, "shuffles it into its owner's library")
        .own_everywhere();

/// "Lands you control enter untapped." (CR 614.1d), as Spelunking and The Wandering Minstrel print it: every land entering the
/// battlefield under the asker's control, judged as it would exist there (CR 614.12), whether it would enter tapped as the event
/// stands or not. So a land whose own "enters tapped" also applies has its controller order the two (CR 616.1): its own first,
/// then this, and it enters untapped; this first, then its own, and it enters tapped (the rulings).
fn lands_you_control_enter_applies(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    z.to.is(ZoneKind::Battlefield) && z.controller == asker.controller && q.chars(z.object).is(CardType::Land)
}

/// The apply half of [`LANDS_YOU_CONTROL_ENTER_UNTAPPED`]: the land enters untapped.
fn lands_enter_untapped(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.tapped = false;
    }
}

/// "Lands you control enter untapped." (CR 614.1d): the whole replacement, as Spelunking and The Wandering Minstrel print it.
pub const LANDS_YOU_CONTROL_ENTER_UNTAPPED: ReplacementDef =
    replacement(ReplacementKind::ZoneChange, lands_you_control_enter_applies, lands_enter_untapped, "lands you control enter untapped");

/// A count of what a replacement adds a +1/+1 counter for, as the card's own ability reads it (CR 614.1c, 205.3m): read as
/// the permanent would enter, so the object entering is not counted yet (CR 614.12).
pub type AlreadyControlled = fn(&dyn Query, &Asker) -> i32;

/// "Each [subtype] you control enters with an additional +1/+1 counter on it for each [already]" (CR 614.1c, 614.12, 122.6):
/// a permanent about to enter the battlefield under this ability's controller that is one of `subtype` as it would exist
/// there (CR 614.12), card or token alike, with at least one counter for `already` to add. A permanent entering at the same
/// time as another is not on the battlefield yet, so it counts neither for that one nor for itself. Giada, Font of Hope and
/// Jean-Luc, Mon Capitaine print it.
pub fn a_subtype_you_control_enters_with_a_counter_for_each(
    q: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
    subtype: Subtype,
    already: AlreadyControlled,
) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.to.is(ZoneKind::Battlefield)
            && z.controller == asker.controller
            && q.chars(z.object).has_subtype(subtype))
        && already(q, asker) > 0
}

/// The other half of `a_subtype_you_control_enters_with_a_counter_for_each`: that many +1/+1 counters, all at once, as part
/// of how the permanent enters (CR 614.1c, 122.6).
pub fn add_a_counter_for_each(q: &dyn Query, asker: &Asker, proposal: &mut Proposal, already: AlreadyControlled) {
    let more = already(q, asker);
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::PlusOne as usize] += more;
    }
}
