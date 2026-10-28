//! Replacement effects (CR 614) many cards print.

use cantrip_api::*;

/// "If one or more +1/+1 counters would be put on a [permanent of these types] you control, that many plus one +1/+1
/// counters are put on it instead" (CR 122.6, 614.1a): whether it replaces this event. Keyed to the recipient, never to
/// who puts the counters; a permanent entering with counters is judged as it would exist on the battlefield
/// (CR 614.12). Michelangelo, Weirdness to 11 (creatures) and Ozolith, the Shattered Spire (artifacts and creatures)
/// print it.
pub fn plus_one_counters_applies(q: &dyn Query, asker: &Asker, proposal: &Proposal, types: &[CardType]) -> bool {
    let Proposal::Counters(c) = proposal else { return false };
    c.controller == asker.controller
        && c.counts[CounterKind::PlusOne as usize] > 0
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

/// "... that many plus one": one more +1/+1 counter. Each such effect applies once, so a second adds another.
pub fn plus_one_counters_apply(proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        c.counts[CounterKind::PlusOne as usize] += 1;
    }
}

/// "If one or more tokens would be created under your control" (CR 111.1, 614.1a): whose they would be, not who makes
/// them.
fn tokens_under_your_control(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::TokenCreation(t) if t.controller == asker.controller && t.count > 0)
}

/// "... twice that many of those tokens are created instead."
fn twice_that_many_tokens(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::TokenCreation(t) = proposal {
        t.count = t.count.saturating_mul(2);
    }
}

fn plus_one_counters_on_your_creatures(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    plus_one_counters_applies(q, asker, proposal, &[CardType::Creature])
}

fn plus_one_counters_apply_to_proposal(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    plus_one_counters_apply(proposal);
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

/// "If one or more tokens would be created under your control, twice that many of those tokens are created instead."
/// Elspeth, Storm Slayer and Exalted Sunborn print it.
pub const DOUBLE_TOKENS: ReplacementDef =
    replacement(ReplacementKind::TokenCreation, tokens_under_your_control, twice_that_many_tokens, "twice that many tokens are created");

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

fn exile_instead_of_dying_applies(q: &dyn Query, _: &Asker, affected: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.object == affected && z.from.is(ZoneKind::Battlefield) && z.to.is(ZoneKind::Graveyard)
            && { let c = q.chars(affected); c.is(CardType::Creature) || c.is(CardType::Planeswalker) })
}

/// `exile_instead_apply` as the `apply` of a `granted_replacement`, which is also told the object it is about. Suplex, Mawloc
/// and Yawgmoth's Will's resolved effects print it.
pub fn exile_instead_apply_granted(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    exile_instead_apply(q, asker, proposal);
}

/// "If that creature would die this turn, exile it instead" (CR 614.1a, 700.4, 611.2c): a replacement effect the spell or
/// ability makes about one object for the turn, not an ability the creature gains. Only a creature or planeswalker moving
/// from the battlefield to a graveyard "dies" (CR 700.4), so one returned to its hand goes there, and one that has stopped
/// being a creature or planeswalker (Lion Sash made an Equipment) is put into the graveyard as an ordinary permanent. List it among a card's `granted_replacements`
/// and make the effect with `exile_if_it_would_die` before anything can kill the creature. Suplex and Mawloc print it.
pub const EXILE_INSTEAD_OF_DYING: GrantedReplacementDef =
    granted_replacement(ReplacementKind::ZoneChange, exile_instead_of_dying_applies, exile_instead_apply_granted, "exiled instead of dying");

/// Makes `EXILE_INSTEAD_OF_DYING` (the card's replacement number `index`) about `creature` until end of turn. It names the
/// object, so it does not follow the creature if it leaves and returns as a new object (CR 400.7).
pub fn exile_if_it_would_die(g: &mut dyn Game, source: ObjectId, index: u8, creature: ObjectId) {
    g.add_effect(source, creature, EffectKind::Replacement(index), Duration::EndOfTurn);
}

/// Which permanents `opponents_enter_tapped` taps: a bit set of these.
pub mod enter_tapped {
    pub const CREATURES: u8 = 1;
    pub const ARTIFACTS: u8 = 2;
    pub const NONBASIC_LANDS: u8 = 4;
}

fn opponents_enter_tapped_applies<const WHICH: u8>(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    z.to.is(ZoneKind::Battlefield) && !z.tapped && z.controller != asker.controller && {
        let c = q.chars(z.object);
        (WHICH & enter_tapped::CREATURES != 0 && c.is(CardType::Creature))
            || (WHICH & enter_tapped::ARTIFACTS != 0 && c.is(CardType::Artifact))
            || (WHICH & enter_tapped::NONBASIC_LANDS != 0 && c.is(CardType::Land) && !c.supertypes.basic)
    }
}

fn opponents_enter_tapped_apply(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.tapped = true;
    }
}

/// "[Creatures / artifacts and creatures / nonbasic lands] your opponents control enter tapped" (CR 614.1d): the
/// object is judged as it would exist on the battlefield (CR 614.12), under the player it enters under (CR 110.2a).
/// `WHICH` is a set of `enter_tapped` bits. Authority of the Consuls (creatures), Spider-Woman, Stunning Savior
/// (artifacts and creatures) and Archon of Emeria (nonbasic lands) print it.
pub const fn opponents_enter_tapped<const WHICH: u8>(hint: &'static str) -> ReplacementDef {
    replacement(ReplacementKind::ZoneChange, opponents_enter_tapped_applies::<WHICH>, opponents_enter_tapped_apply, hint)
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

/// "If an opponent would draw a card except the first one they draw in each of their draw steps" (CR 121.6, 614.1a): one
/// draw, judged as it is about to be made (CR 121.2), by an opponent of the effect's controller, unless it would be the first
/// card that player draws in their own draw step (a draw replaced before it was not drawn, CR 614.6). Hullbreacher and
/// Notion Thief print it.
pub fn opponent_would_draw_except_first_in_draw_step(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Draw(d)
        if q.state().opponents(asker.controller).any(|o| o == d.player) && !crate::history::first_draw_in_own_draw_step(q, d.player))
}

/// "If you would draw a card except the first one you draw in each of your draw steps" (CR 121.6, 614.1a): each draw on its own,
/// judged as it is about to be made, so the turn's first card drawn in the draw step is left alone, and the one after it is not.
fn you_would_draw_except_first_in_draw_step(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    you_would_draw_a_card(q, asker, proposal) && !crate::history::first_draw_in_own_draw_step(q, asker.controller)
}

/// "... draw two cards instead": two draws, each meeting the replacement effects that have not yet had their opportunity at the
/// draw this replaced (CR 614.5), so two such effects make four.
fn draw_two(g: &mut dyn Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    g.draw(asker.controller, 2);
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
fn dealt_to_this_creature_instead(_: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
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
    let Proposal::Damage(d) = proposal else { return false };
    d.amount > 0
        && !d.combat
        && q.controller(d.source) == asker.controller
        && match d.target {
            Target::Player(player) => q.state().opponents(asker.controller).any(|opponent| opponent == player),
            recipient => recipient.object().is_some_and(|id| q.controller(id) != asker.controller),
        }
}

/// "... it deals that much damage plus `N` instead" (CR 614.1): the damage about to be dealt is `N` greater.
pub fn plus<const N: u16>(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount = d.amount.saturating_add(N);
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
