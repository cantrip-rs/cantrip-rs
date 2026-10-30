//! Replacement effects (CR 614, 616): finding the ones that apply to a proposed event, having the affected player
//! order them when there are several, and applying them one at a time until none is left.
//!
//! The effects belong to cards (`CardDef::replacements`): the core knows no card. An event that nothing on the board
//! could replace costs one scan of the battlefield for a flag per card, and nothing is built. One keyword's replacement
//! is the core's own and is ordered with the cards' like any other: madness (CR 702.35a), of a card being discarded.
//!
//! Prevention effects (CR 615) are ordered with them (CR 616.1): a static ability's (`CardDef::preventions`), one a
//! resolved spell or ability made (`EffectKind::Prevention`) and protection's (CR 702.16e). Each that prevents damage
//! records it (`Event::DamagePrevented`, CR 615.6), and none applies to damage that can't be prevented (CR 615.12): such
//! a prevention effect would prevent nothing, and a shield is not reduced by it.

use cantrip_api::*;

use crate::layers::live;
use crate::Cx;

/// Where a replacement effect is defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Origin {
    /// One of its source card's `replacements`.
    Printed,
    /// One a spell or ability made (CR 611.2, `EffectKind::Replacement`), named by that effect's timestamp: one of the
    /// `granted_replacements` of the card the effect names.
    Granted(u32),
    /// Madness's static ability (CR 702.35a), the core's own keyword replacement, of the discarded card itself.
    Madness,
    /// A spell's own "exile this card instead of putting it anywhere else any time it would leave the stack": flashback's,
    /// harmonize's or aftermath's (CR 702.34a, 702.180a, 702.127a; `ZoneChange::flashback`), of the spell leaving.
    Flashback,
    /// "If that spell would be put into your graveyard, exile it instead" (CR 614.1a; `ZoneChange::exile_instead`): the
    /// replacement the effect that had the spell cast made, of the spell leaving the stack for its graveyard.
    ExileInstead,
    /// An Adventure spell's exile as it resolves (CR 715.3d; `ZoneChange::adventure`).
    Adventure,
    /// An Omen spell's shuffle into its owner's library as it resolves (CR 720.3d; `ZoneChange::omen`).
    Omen,
    /// Rebound's exile as the spell resolves (CR 702.88a; `ZoneChange::rebound`).
    Rebound,
    /// "If it would leave the battlefield, exile it instead of putting it anywhere else": unearth's (CR 702.84a) and the
    /// effects worded as it is (From the Catacombs), of a permanent marked so (`Obj::exile_on_leave`).
    Unearth,
    /// Finality counters' "if this permanent would be put into a graveyard from the battlefield, exile it instead" (CR 122.1h).
    Finality,
    /// A regeneration shield (CR 701.19a, 614.8): of a permanent that would be destroyed (`ZoneChange::destroy`).
    Regeneration,
    /// Shield counters' replacement of a destruction and prevention of damage (CR 122.1c): of the permanent they are on.
    ShieldCounter,
    /// Protection's prevention of damage (CR 702.16e, 702.16j), a prevention effect ordered with the replacement effects that
    /// would change the same damage (CR 616.1): of the protected permanent, or the source of a player's protection.
    Protection,
    /// One of its source card's `preventions`, a static ability's prevention effect (CR 615, 604).
    Prevention,
    /// A prevention effect a spell or ability made (CR 611.2, 615, `EffectKind::Prevention`), named by that effect's
    /// timestamp: one of its source card's `granted_preventions`.
    GrantedPrevention(u32),
    /// "Each land played this way enters tapped" (CR 305.1, 614.1d, `PlayGrant::enters_tapped`): the replacement the effect
    /// that gave a permission to play the land made, of the land played by it.
    PlayedTapped,
    /// The entering permanent's own "enters tapped", "enters with" or "as it enters" (CR 614.1c-d, 614.12): this card's
    /// `CardDef::enters`, the one in `Entering::own` at the candidate's index. Ordered with the others that modify how it
    /// enters (CR 616.1): a land's own "enters tapped" and Spelunking's "lands you control enter untapped" in either order.
    Enters(CardId),
}

/// One effect that applies to the event: its source, and which of the source's replacements it is.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Candidate {
    source: ObjectId,
    index: u8,
    origin: Origin,
}

/// An effect already applied to the event (CR 614.5): its source, its index and where it is defined.
pub(crate) type Applied = (ObjectId, u8, Origin);

/// What madness's replacement says it does, for the player who orders it with others (CR 616.1).
const MADNESS_HINT: &str = "madness discards it into exile, then it may be cast for its madness cost";

/// What a spell's flashback (or harmonize, or aftermath) says it does, for the player who orders it with others (CR 616.1).
const FLASHBACK_HINT: &str = "exile it instead of putting it anywhere else, as it was cast with flashback";

/// What the "exile it instead" of the effect a spell was cast by says it does, for the player who orders it (CR 616.1).
const EXILE_INSTEAD_HINT: &str = "exile it instead of putting it into the graveyard, as the effect it was cast by says";

/// What the dream-counter exile of a spell says it does (Goliath Daydreamer's trigger; CR 616.1).
const DREAM_EXILE_HINT: &str = "exile it with a dream counter on it instead of putting it into the graveyard";

/// What the rules' replacements of a spell resolving, a permanent leaving and a destruction say they do, for the player who
/// orders them with others (CR 616.1).
const ADVENTURE_HINT: &str = "exile it on an Adventure, to be cast later as itself";
const OMEN_HINT: &str = "shuffle it into its owner's library, as an Omen";
const REBOUND_HINT: &str = "exile it with rebound, to be cast again at the next upkeep";
const UNEARTH_HINT: &str = "exile it instead of putting it anywhere else";
const FINALITY_HINT: &str = "exile it, as it has a finality counter";
const REGENERATION_HINT: &str = "regenerate it instead of destroying it";
const SHIELD_DESTROY_HINT: &str = "remove a shield counter from it instead of destroying it";
const SHIELD_DAMAGE_HINT: &str = "prevent the damage and remove a shield counter from it";

/// What protection's prevention says it does, for the player who orders it with others (CR 616.1).
const PROTECTION_HINT: &str = "protection prevents the damage";

/// What a permission's "enters tapped" says it does, for the player who orders it with others (CR 616.1).
const PLAYED_TAPPED_HINT: &str = "a land played this way enters tapped";

/// What a permanent's own entering replacement says it does where its text has no line for it (CR 616.1).
const ENTERS_HINT: &str = "its own replacement as it enters";

/// What a permanent's own entering replacement (`CardDef::enters`) says it does, for the player who orders it with others
/// (CR 616.1): the line of its card's rules text that says how it enters ("This land enters tapped.", "As this creature
/// enters, choose a color.") - not reminder text ("(As this Saga enters ...)"), nor a triggered ability's line ("When this
/// creature enters, ...", "Landfall — Whenever a land you control enters, ...").
fn enters_hint(def: &CardDef) -> &'static str {
    let text: &'static str = def.text;
    text.lines().map(str::trim).find(|line| says_how_it_enters(line)).unwrap_or(ENTERS_HINT)
}

/// Whether a line of rules text is a static ability's that speaks of entering: "enter" outside its reminder text, and no
/// trigger word starting the ability, after an ability word if it has one (CR 207.2c).
pub(crate) fn says_how_it_enters(line: &str) -> bool {
    let mut words = String::with_capacity(line.len());
    let mut depth = 0u32;
    for c in line.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => words.push(c),
            _ => {}
        }
    }
    let words = words.trim();
    let ability = match words.split_once(" — ") {
        Some((word, rest)) if !word.contains(['.', ',', ':']) => rest,
        _ => words,
    };
    words.contains("enter") && !["When ", "Whenever ", "At "].iter().any(|t| ability.starts_with(t))
}

/// Whether some replacement effect could apply to `change`, other than an entering permanent's own (`Entering::own`): the
/// guard that keeps an entry with nothing to order free of the ordering.
pub(crate) fn zone_change_possible(cx: &Cx, change: &ZoneChange) -> bool {
    possible(cx, Some(change.object)) || madness_applies(cx, change) || played_tapped_applies(cx, change) || flashback_applies(change) || exile_instead_applies(cx, change)
        || resolving_applies(change)
        || leaving_applies(cx, change)
}

/// `change` sent to `instead` by the entering permanent's own replacement (`Entering::instead`, CR 614.12): no entry any more,
/// but a move there of the object as it is, under its controller (Mox Diamond's "put it into its owner's graveyard").
pub(crate) fn sent_instead(cx: &Cx, mut change: ZoneChange, instead: Zone) -> ZoneChange {
    change.to = instead;
    change.tapped = false;
    change.controller = cx.s().obj(change.object).controller;
    change.counters_by = change.controller;
    change
}

/// The replacement effects that modify how a permanent enters (CR 614.12) applied to its entry: its own still to apply
/// (`Entering::own`), which change `entering`, among the others, in the order its controller chooses (CR 616.1). Called by
/// `ops::entry_replacements` with the permanent judged as it would exist (`Cx::entering`).
pub(crate) fn entry(cx: &mut Cx, change: ZoneChange, entering: &mut Entering) -> ZoneChange {
    let mut proposal = Proposal::ZoneChange(change);
    apply_to(cx, &mut proposal, Some(entering));
    match proposal {
        Proposal::ZoneChange(change) => change,
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// "Each land played this way enters tapped" (CR 305.1, 614.1d): a land played by a permission that says so, entering
/// untapped as the proposal stands. Judged before the land moves, while it is still where it was played from and the
/// permission is there to read: the land play keeps only the permission it was made by (`actions::act`).
fn played_tapped_applies(cx: &Cx, z: &ZoneChange) -> bool {
    z.reason == ZoneReason::PlayLand
        && z.to.is(ZoneKind::Battlefield)
        && !z.tapped
        && cx.s().permissions.iter().any(|p| p.what == Playable::Card(z.object) && p.grant.enters_tapped)
}

/// Madness (CR 702.35a): "If a player would discard this card, that player discards it, but exiles it instead of putting
/// it into their graveyard." A static ability of the card itself, functioning while it is in a hand; once another
/// effect has sent the card elsewhere than the graveyard it no longer applies (CR 616.1f).
fn madness_applies(cx: &Cx, z: &ZoneChange) -> bool {
    z.reason == ZoneReason::Discard
        && z.from.is(ZoneKind::Hand)
        && z.to.is(ZoneKind::Graveyard)
        && cx.cards().get(card_of(cx, z.object)).madness.is_some()
        && !cx.lost_all_abilities(z.object)
}

/// Flashback's, harmonize's or aftermath's replacement (CR 702.34a, 702.180a, 702.127a): "exile this card instead of putting it
/// anywhere else any time it would leave the stack" - so of a card (a copy of a spell, or of a card, is no card, CR 707.10)
/// that would leave the stack for any zone but exile. Once another effect has exiled it, it no longer applies (CR 616.1f).
fn flashback_applies(z: &ZoneChange) -> bool {
    z.flashback && z.is_card && z.from.is(ZoneKind::Stack) && !z.to.is(ZoneKind::Exile)
}

/// "If that spell would be put into your graveyard, exile it instead" (CR 614.1a, `ExileInstead`): of the spell leaving the
/// stack for its owner's graveyard and nowhere else, and for `ExileInstead::InstantOrSorcery` only while it is an instant
/// or a sorcery spell, as it now is (CR 709.3b, 715.3b: a half, an Adventure or a face cast is judged by its own types).
fn exile_instead_applies(cx: &Cx, z: &ZoneChange) -> bool {
    if !z.is_card || !z.from.is(ZoneKind::Stack) || !z.to.is(ZoneKind::Graveyard) {
        return false;
    }
    match z.exile_instead {
        ExileInstead::Never => false,
        ExileInstead::Spell | ExileInstead::Dream => true,
        ExileInstead::InstantOrSorcery => {
            let chars = cx.chars(z.object);
            chars.is(CardType::Instant) || chars.is(CardType::Sorcery)
        }
    }
}

/// The rules' "instead of putting it into its owner's graveyard as it resolves" for the way a spell was cast: an Adventure's
/// exile (CR 715.3d), an Omen's shuffle (CR 720.3d), rebound's exile (CR 702.88a) - of a card, as it would be put into its
/// graveyard from the stack. Once another effect has sent it elsewhere, none of them applies any more (CR 616.1f).
fn resolving_to_graveyard(z: &ZoneChange) -> bool {
    z.is_card && z.from.is(ZoneKind::Stack) && z.to.is(ZoneKind::Graveyard)
}

fn resolving_applies(z: &ZoneChange) -> bool {
    (z.adventure || z.omen || z.rebound) && resolving_to_graveyard(z)
}

/// "Exile it instead of putting it anywhere else" (CR 702.84a): a permanent marked so leaving the battlefield for any zone
/// but exile.
fn unearth_applies(cx: &Cx, z: &ZoneChange) -> bool {
    z.from.is(ZoneKind::Battlefield) && !z.to.is(ZoneKind::Exile) && cx.s().obj(z.object).exile_on_leave
}

/// CR 122.1h: a permanent with a finality counter that would be put into a graveyard from the battlefield.
fn finality_applies(cx: &Cx, z: &ZoneChange) -> bool {
    z.from.is(ZoneKind::Battlefield) && z.to.is(ZoneKind::Graveyard) && cx.s().obj(z.object).counters(CounterKind::Finality) > 0
}

/// CR 701.19a, 614.8: "the next time [it] would be destroyed", a permanent with a regeneration shield that would be destroyed
/// (CR 701.8a: moved from the battlefield to its owner's graveyard). Once another effect has sent it elsewhere, the event is
/// no destruction any more (CR 614.6), and the shield does not apply.
fn regeneration_applies(cx: &Cx, z: &ZoneChange) -> bool {
    z.destroy
        && !z.regenerated
        && !z.shielded
        && z.from.is(ZoneKind::Battlefield)
        && z.to.is(ZoneKind::Graveyard)
        && cx.s().effects.iter().any(|e| e.kind == EffectKind::RegenerationShield && e.affected == z.object)
}

/// CR 122.1c: "if this permanent would be destroyed as the result of an effect, instead remove a shield counter from it": a
/// permanent with a shield counter that an effect that says "destroy" would put into its owner's graveyard (CR 701.8a-b). Not a
/// sacrifice, and not a destruction by lethal damage or deathtouch (state-based actions, CR 704.5g-h): the rulings. Once another
/// effect has sent it elsewhere, the event is no destruction any more (CR 614.6), and the counter does not apply.
fn shield_destroy_applies(cx: &Cx, z: &ZoneChange) -> bool {
    matches!(z.reason, ZoneReason::Destroy | ZoneReason::DestroyNoRegen)
        && !z.regenerated
        && !z.shielded
        && z.from.is(ZoneKind::Battlefield)
        && z.to.is(ZoneKind::Graveyard)
        && cx.s().obj(z.object).counters(CounterKind::Shield) > 0
}

/// CR 122.1c: "if damage would be dealt to this permanent, prevent that damage and remove a shield counter from it": damage
/// to a permanent with a shield counter, or to one whose counter has applied to this damage event already (`DamageEvent::shielded`:
/// one replacement for the simultaneous event, CR 120.4). Unpreventable damage is still dealt and the counter still removed
/// (the rulings).
fn shield_damage_applies(cx: &Cx, d: &DamageEvent) -> Option<ObjectId> {
    let Target::Object(id) = d.target else { return None };
    (d.amount > 0 && live(cx.s(), id) && (d.shielded == Some(id) || cx.s().obj(id).counters(CounterKind::Shield) > 0)).then_some(id)
}

fn leaving_applies(cx: &Cx, z: &ZoneChange) -> bool {
    z.from.is(ZoneKind::Battlefield) && (unearth_applies(cx, z) || finality_applies(cx, z) || regeneration_applies(cx, z) || shield_destroy_applies(cx, z))
}

/// The card whose replacement effects `id` has: `card_of`, but for a permanent about to enter, judged as it would exist on
/// the battlefield (CR 614.12), the card it would have there (a copy's), as whether it would have lost its abilities is
/// (`Query::lost_all_abilities`).
fn source_card(cx: &Cx, id: ObjectId) -> CardId {
    match cx.entering {
        Some(w) if w.object == id => w.chars.card,
        _ => card_of(cx, id),
    }
}

/// The card whose definition a permanent's abilities come from: the card it copies, if it copies one, or the face that is up
/// (CR 712.8d-f: a double-faced permanent with its back face up has only that face's abilities).
fn card_of(cx: &Cx, id: ObjectId) -> CardId {
    let obj = cx.s().obj(id);
    obj.copied_card().unwrap_or_else(|| cx.cards().face(obj.card, obj.transformed))
}

/// Whether any effect could apply to an event about `subject`: a permanent with replacements, or the subject's own
/// card, which has some that function from every zone. The guard that keeps the hot path free of replacement work.
pub(crate) fn possible(cx: &Cx, subject: Option<ObjectId>) -> bool {
    // An effect a spell or ability made, which lasts (CR 611.2).
    if cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::Replacement { .. } | EffectKind::ReplacementRule { .. } | EffectKind::Prevention { .. })) {
        return true;
    }
    let cards = cx.cards();
    if !cards.any_replaces {
        return false;
    }
    subject.is_some_and(|id| cards.replaces[source_card(cx, id).0 as usize])
        || cx.s().battlefield().iter().any(|&id| cards.replaces[card_of(cx, id).0 as usize])
}

/// Whether a card in `player`'s graveyard has an ability that replaces their draws there (CR 702.52a: dredge).
fn graveyard_replaces_draws(cx: &Cx, player: PlayerId) -> bool {
    let cards = cx.cards();
    cards.any_replaces
        && cx.s().zone(Zone::graveyard(player)).iter().any(|&id| {
            let card = card_of(cx, id);
            cards.replaces[card.0 as usize] && cards.get(card).replacements.iter().any(|r| r.in_graveyard)
        })
}

/// Whether every draw `player` has left would only repeat the attempt from an empty library already made (CR 121.4): the
/// library is empty and was drawn from (or no draw can happen, CR 121.2b), and no effect would replace a draw of this
/// player's (CR 121.6): an unrelated replacement effect in the game does not defeat the stop, only one that applies to
/// this draw does (Hullbreacher).
pub fn draws_only_repeat_an_attempt_from_an_empty_library(cx: &Cx, player: PlayerId) -> bool {
    cx.s().zone(Zone::library(player)).is_empty()
        && (cx.s().player(player).drew_from_empty || cantrip_api::cant_draw(cx, player, 1))
        && !graveyard_replaces_draws(cx, player)
        && (!possible(cx, None) || candidates(cx, &Proposal::Draw(CardDraw { player, replaced: false }), &[], None).is_empty())
}

fn subject_of(proposal: &Proposal) -> Option<ObjectId> {
    match proposal {
        Proposal::ZoneChange(z) => Some(z.object),
        Proposal::Counters(c) => Some(c.object),
        // The permanent tapped for mana. Its own `own_everywhere` effects are looked at where a cost has moved it (a
        // sacrificed Treasure's): no printed card has one of this kind, which would function from its new zone here.
        Proposal::ManaProduction(m) => Some(m.permanent),
        Proposal::DrawInstruction(_)
        | Proposal::Draw(_)
        | Proposal::TokenCreation(_)
        | Proposal::PlayerCounters(_)
        | Proposal::LifeChange(_) => None,
        Proposal::Damage(d) => match d.target {
            Target::Object(id) => Some(id),
            _ => None,
        },
    }
}

/// "You" for an effect: its controller; for an effect of the very permanent entering, the player it enters under
/// (CR 110.2a, 614.12).
fn controller_of(cx: &Cx, source: ObjectId, proposal: &Proposal) -> PlayerId {
    match proposal {
        Proposal::ZoneChange(z) if z.object == source => z.controller,
        Proposal::Counters(c) if c.object == source && c.entering => c.controller,
        _ => cx.s().obj(source).controller,
    }
}

/// The player CR 616.1 gives the choice to: whose permanent it is (the player it enters under), or who would draw.
fn affected(cx: &Cx, proposal: &Proposal) -> PlayerId {
    match proposal {
        Proposal::Damage(d) => match d.target {
            Target::Player(p) => p,
            Target::Object(id) => cx.s().obj(id).controller,
            _ => cx.s().turn.active,
        },
        Proposal::ZoneChange(z) => z.controller,
        Proposal::Counters(c) => c.controller,
        Proposal::PlayerCounters(c) => c.player,
        Proposal::DrawInstruction(d) => d.player,
        Proposal::Draw(d) => d.player,
        Proposal::TokenCreation(t) => t.controller,
        Proposal::LifeChange(l) => l.player,
        // "If multiple replacement effects would modify the mana an ability you control produces, choose one to apply"
        // (Damping Sphere's ruling): the player who tapped the permanent for it.
        Proposal::ManaProduction(m) => m.player,
    }
}

/// Every effect that applies to the event and has not been applied to it yet (CR 614.5), in a deterministic order:
/// by source, then by the order the card lists them.
fn candidates(cx: &Cx, proposal: &Proposal, applied: &[Applied], entry: Option<&Entering>) -> Vec<Candidate> {
    // Damage that has been prevented in full is no event any more: nothing is left to replace (CR 615.1); nor is a draw that
    // an effect has replaced, which never happens (CR 614.6).
    if let Proposal::Damage(d) = proposal
        && d.amount == 0
    {
        return Vec::new();
    }
    if let Proposal::Draw(d) = proposal
        && d.replaced
    {
        return Vec::new();
    }
    // Nor is a move a regeneration shield, a shield counter or another replacement effect replaced by nothing (CR 614.6).
    if let Proposal::ZoneChange(z) = proposal
        && (z.regenerated || z.shielded || z.replaced)
    {
        return Vec::new();
    }
    let cards = cx.cards();
    let kind = proposal.kind();
    let subject = subject_of(proposal);
    // Damage a prevention effect may prevent (CR 615.12): where it can't be, none of them applies.
    let preventable = match proposal {
        Proposal::Damage(d) => crate::ops::damage_can_be_prevented(cx, d.source, d.combat),
        _ => false,
    };
    let mut found = Vec::new();
    // Where the object is: on the battlefield (`Field`), its own card elsewhere (`Own`), or a card in a graveyard whose
    // abilities function there (`Graveyard`, CR 702.52a).
    #[derive(PartialEq)]
    enum At {
        Field,
        Own,
        Graveyard,
    }
    let mut consider = |id: ObjectId, at: At| {
        // A permanent about to enter is judged as it would exist on the battlefield (CR 614.12): the card its replacements
        // of its own are read from is the one it would have there (a copy's), as its abilities are whether it would have
        // lost them (`Query::lost_all_abilities`).
        let card = source_card(cx, id);
        // A permanent that has lost all its abilities has no replacement effects (CR 613.1f).
        if !cards.replaces[card.0 as usize] || cx.lost_all_abilities(id) {
            return;
        }
        // "You" for an ability of a card in a graveyard is its owner (CR 108.4a: it has no controller).
        let controller = if at == At::Graveyard { cx.s().obj(id).owner } else { controller_of(cx, id, proposal) };
        let asker = Asker { source: id, controller, by: None, spell: false };
        for (index, def) in cards.get(card).replacements.iter().enumerate() {
            let index = index as u8;
            let functions = match at {
                At::Field => !def.in_graveyard,
                At::Own => def.own_everywhere,
                At::Graveyard => def.in_graveyard,
            };
            if def.kind != kind || !functions || applied.contains(&(id, index, Origin::Printed)) {
                continue;
            }
            if (def.applies)(cx, &asker, proposal)
                && valid_redirection(cx, proposal, |after| (def.apply)(cx, &asker, after))
            {
                found.push(Candidate { source: id, index, origin: Origin::Printed });
            }
        }
        // Its static abilities' prevention effects function only while it is on the battlefield (CR 604.1).
        if let Proposal::Damage(d) = proposal
            && preventable
            && at == At::Field
        {
            for (index, def) in cards.get(card).preventions.iter().enumerate() {
                let index = index as u8;
                if !applied.contains(&(id, index, Origin::Prevention)) && (def.applies)(cx, &asker, d) {
                    found.push(Candidate { source: id, index, origin: Origin::Prevention });
                }
            }
        }
    };
    for &id in cx.s().battlefield() {
        if live(cx.s(), id) {
            consider(id, At::Field);
        }
    }
    // The subject's own effects that function in every zone (CR 113.6b), when it is not on the battlefield already.
    if let Some(id) = subject
        && !cx.s().obj(id).zone.is(ZoneKind::Battlefield)
    {
        consider(id, At::Own);
    }
    // The drawing player's graveyard, whose cards' dredge replaces their draws (CR 702.52a).
    if let Proposal::Draw(d) = proposal
        && graveyard_replaces_draws(cx, d.player)
    {
        for &id in cx.s().zone(Zone::graveyard(d.player)) {
            consider(id, At::Graveyard);
        }
    }
    // The replacement effects spells and abilities made, still lasting (CR 611.2): one about an object ends with it.
    for e in &cx.s().effects {
        let (card, index, about_player) = match e.kind {
            EffectKind::Replacement { card, index } => (card, index, false),
            EffectKind::ReplacementRule { card, index } => (card, index, true),
            // A prevention effect a spell or ability made (CR 615.3): of damage to whom it names, if it names anyone, and of
            // the damage its definition describes. One about an object ends with it (CR 400.7).
            EffectKind::Prevention { card, index, to, .. } => {
                if let Proposal::Damage(d) = proposal
                    && preventable
                    && (to == Target::None || to == d.target)
                    && (to.object().is_none() || live(cx.s(), e.affected))
                    && !applied.contains(&(e.source, index, Origin::GrantedPrevention(e.timestamp)))
                {
                    let def = &cards.get(card).granted_preventions[index as usize];
                    let asker = Asker { source: e.source, controller: e.controller, by: None, spell: false };
                    if (def.applies)(cx, &asker, e.affected, d) {
                        found.push(Candidate { source: e.source, index, origin: Origin::GrantedPrevention(e.timestamp) });
                    }
                }
                continue;
            }
            _ => continue,
        };
        if !about_player && !live(cx.s(), e.affected) {
            continue;
        }
        let def = &cards.get(card).granted_replacements[index as usize];
        if def.kind != kind || applied.contains(&(e.source, index, Origin::Granted(e.timestamp))) {
            continue;
        }
        let asker = Asker { source: e.source, controller: e.controller, by: None, spell: false };
        if (def.applies)(cx, &asker, e.affected, proposal)
            && valid_redirection(cx, proposal, |after| (def.apply)(cx, &asker, e.affected, after))
        {
            found.push(Candidate { source: e.source, index, origin: Origin::Granted(e.timestamp) });
        }
    }
    found.sort_by_key(|c| (c.source, c.index, c.origin));
    // The entering permanent's own replacements still to apply (CR 614.12), ordered with the rest (CR 616.1), while it still
    // enters - judged as it would exist with those applied so far, which may have taken its abilities (CR 613.1f). Offered
    // first, as they were printed on it.
    if let (Some(e), Proposal::ZoneChange(z)) = (entry, proposal)
        && z.object == e.object
        && z.to.is(ZoneKind::Battlefield)
        && !cx.entering.is_some_and(|w| w.object == e.object && w.lost)
    {
        let own = e.own.map(|card| Candidate { source: e.object, index: 0, origin: Origin::Enters(card) });
        found.splice(0..0, own);
    }
    // The discarded card's own madness (CR 702.35a), ordered with the rest (CR 616.1). It is offered first, as the
    // order its owner almost always wants: declining the cast still puts the card into the graveyard, where the other
    // effects meet it again. A player or bot who takes the first offer keeps the madness.
    if let Proposal::ZoneChange(z) = proposal
        && !applied.contains(&(z.object, 0, Origin::Madness))
        && madness_applies(cx, z)
    {
        found.insert(0, Candidate { source: z.object, index: 0, origin: Origin::Madness });
    }
    // A spell's own replacements of its leaving the stack (CR 702.34a, 614.1a), ordered with the rest (CR 616.1): of the spell
    // itself, which is what the player ordering them is shown. Offered first, as the order its controller almost always wants.
    if let Proposal::ZoneChange(z) = proposal {
        if !applied.contains(&(z.object, 0, Origin::ExileInstead)) && exile_instead_applies(cx, z) {
            found.insert(0, Candidate { source: z.object, index: 0, origin: Origin::ExileInstead });
        }
        if !applied.contains(&(z.object, 0, Origin::Flashback)) && flashback_applies(z) {
            found.insert(0, Candidate { source: z.object, index: 0, origin: Origin::Flashback });
        }
        // The rules' own for how it was cast, as it resolves (CR 715.3d, 720.3d, 702.88a), and for a permanent leaving the
        // battlefield (CR 702.84a, 122.1h, 701.19a): of the object itself, offered first.
        let own = [
            (resolving_to_graveyard(z) && z.adventure, Origin::Adventure),
            (resolving_to_graveyard(z) && z.omen, Origin::Omen),
            (resolving_to_graveyard(z) && z.rebound, Origin::Rebound),
            (unearth_applies(cx, z), Origin::Unearth),
            (finality_applies(cx, z), Origin::Finality),
            (regeneration_applies(cx, z), Origin::Regeneration),
            (shield_destroy_applies(cx, z), Origin::ShieldCounter),
        ];
        for (applies, origin) in own {
            if applies && !applied.contains(&(z.object, 0, origin)) {
                found.insert(0, Candidate { source: z.object, index: 0, origin });
            }
        }
    }
    // A permission's "each land played this way enters tapped" (CR 614.1d), ordered with the rest (CR 616.1): it is of the
    // land itself, which is what the player ordering them is shown.
    if let Proposal::ZoneChange(z) = proposal
        && !applied.contains(&(z.object, 0, Origin::PlayedTapped))
        && played_tapped_applies(cx, z)
    {
        found.push(Candidate { source: z.object, index: 0, origin: Origin::PlayedTapped });
    }
    // A shield counter of the permanent the damage would now be dealt to (CR 122.1c), ordered with the effects that would change
    // it (CR 616.1): of the permanent itself, which is what the player ordering them is shown.
    if let Proposal::Damage(d) = proposal
        && let Some(id) = shield_damage_applies(cx, d)
        && !applied.contains(&(id, 0, Origin::ShieldCounter))
    {
        found.insert(0, Candidate { source: id, index: 0, origin: Origin::ShieldCounter });
    }
    // Protection of whoever the damage would now be dealt to (CR 702.16e), ordered with the effects that would change it
    // (CR 616.1): prevented first, nothing is redirected; redirected first, it is judged again at its new recipient. It is
    // offered first, as the order the protected player almost always wants. Only asked about where some other effect
    // applies too: alone, `ops::deal_damage_with` prevents the damage itself.
    if let Proposal::Damage(d) = proposal
        && !found.is_empty()
        && let Some(by) = crate::ops::protection_prevents(cx, d.source, d.target, d.combat)
        && !applied.contains(&(by, 0, Origin::Protection))
    {
        found.insert(0, Candidate { source: by, index: 0, origin: Origin::Protection });
    }
    // A card discarded to pay a cost that specifies a characteristic of it can't pay it from a hidden zone it was put into
    // unrevealed (CR 701.9c): an effect that would put it there is not offered, and it goes where the others leave it.
    if let Proposal::ZoneChange(z) = proposal
        && z.pays_characteristic
    {
        found.retain(|c| {
            let mut after = *proposal;
            reshape(cx, c, &mut after);
            !matches!(after, Proposal::ZoneChange(a) if matches!(a.to.kind(), ZoneKind::Hand | ZoneKind::Library) && !a.reveal)
        });
    }
    found
}

/// The lasting replacement effect a spell or ability made (CR 611.2) that has this timestamp, with its definition: one of
/// the `granted_replacements` of the card the effect names, whatever its source has become since.
fn granted<'a>(cx: &'a Cx, timestamp: u32) -> Option<(&'a ContinuousEffect, &'static GrantedReplacementDef)> {
    let e = cx.s().effects.iter().find(|e| e.timestamp == timestamp)?;
    let (EffectKind::Replacement { card, index } | EffectKind::ReplacementRule { card, index }) = e.kind else { return None };
    Some((e, &cx.cards().get(card).granted_replacements[index as usize]))
}

/// What an effect says it does, for the player who orders several (CR 616.1).
fn hint_of(cx: &Cx, proposal: &Proposal, c: &Candidate) -> &'static str {
    match c.origin {
        Origin::Printed => cx.cards().get(card_of(cx, c.source)).replacements[c.index as usize].hint,
        Origin::Granted(timestamp) => granted(cx, timestamp).map_or("", |(_, def)| def.hint),
        Origin::Madness => MADNESS_HINT,
        Origin::Flashback => FLASHBACK_HINT,
        Origin::ExileInstead => match proposal {
            Proposal::ZoneChange(z) if z.exile_instead == ExileInstead::Dream => DREAM_EXILE_HINT,
            _ => EXILE_INSTEAD_HINT,
        },
        Origin::Adventure => ADVENTURE_HINT,
        Origin::Omen => OMEN_HINT,
        Origin::Rebound => REBOUND_HINT,
        Origin::Unearth => UNEARTH_HINT,
        Origin::Finality => FINALITY_HINT,
        Origin::Regeneration => REGENERATION_HINT,
        Origin::ShieldCounter if matches!(proposal, Proposal::Damage(_)) => SHIELD_DAMAGE_HINT,
        Origin::ShieldCounter => SHIELD_DESTROY_HINT,
        Origin::Protection => PROTECTION_HINT,
        Origin::Prevention => cx.cards().get(card_of(cx, c.source)).preventions[c.index as usize].hint,
        Origin::GrantedPrevention(timestamp) => match cx.s().effects.iter().find(|e| e.timestamp == timestamp).map(|e| e.kind) {
            Some(EffectKind::Prevention { card, index, .. }) => cx.cards().get(card).granted_preventions[index as usize].hint,
            _ => "",
        },
        Origin::PlayedTapped => PLAYED_TAPPED_HINT,
        Origin::Enters(card) => enters_hint(cx.cards().get(card)),
    }
}

/// Whether the effect prevents all the damage it applies to and uses nothing up: a static ability's prevention of all of
/// it, protection's, or a resolved effect that prevents all of it. Which of several such effects applies first changes nothing
/// but which one is recorded as having prevented it, so the affected player is not asked to order only these (CR 616.1).
fn prevents_all(cx: &Cx, c: &Candidate) -> bool {
    match c.origin {
        Origin::Protection => true,
        Origin::Prevention => cx.cards().get(card_of(cx, c.source)).preventions[c.index as usize].amount.is_none(),
        Origin::GrantedPrevention(timestamp) => cx
            .s()
            .effects
            .iter()
            .any(|e| e.timestamp == timestamp && matches!(e.kind, EffectKind::Prevention { shield: Shield::All, .. })),
        _ => false,
    }
}

/// Asks the affected player which of several effects applies first (CR 616.1). When the question has no answer yet
/// the run is abandoned and replayed with it (see `Cx::ask_core`), so the first is taken to carry on to the end.
fn choose(cx: &mut Cx, proposal: &Proposal, found: &[Candidate]) -> usize {
    let ask = RequestKind::ChooseReplacement {
        subject: subject_of(proposal),
        life: match proposal {
            Proposal::LifeChange(l) => Some(l.amount),
            _ => None,
        },
        sources: found.iter().map(|c| c.source).collect(),
        hints: found.iter().map(|c| hint_of(cx, proposal, c)).collect(),
        damage: match proposal {
            Proposal::Damage(d) => Some(DamageAbout { source: d.source, to: d.target, amount: d.amount }),
            _ => None,
        },
        turn: false,
        counters: match proposal {
            Proposal::Counters(c) => CounterKind::ALL.into_iter().zip(c.counts).filter(|&(_, n)| n > 0).collect(),
            _ => Vec::new(),
        },
        draw: matches!(proposal, Proposal::Draw(_)),
    };
    match cx.ask_core(affected(cx, proposal), ask, false) {
        Ok(Answer::Index(i)) if (i as usize) < found.len() => i as usize,
        _ => 0,
    }
}

/// Whether `you` take an optional effect ("you may ... instead", CR 614.1a) as it would apply: the effect's "you", the
/// controller of its source (CR 109.5) - or for one functioning from a graveyard, its owner - who need not be the affected
/// player who orders the effects (CR 616.1): "if a creature an opponent controls would die, you may exile it instead" is the
/// effect's controller's choice. When the question has no answer yet the run goes on as if they did and is replayed with the
/// answer (see `Cx::ask_core`).
fn takes(cx: &mut Cx, you: PlayerId, source: ObjectId, question: &'static Question) -> bool {
    !matches!(cx.ask_core(you, RequestKind::May { source, purpose: Purpose::Ask(question) }, false), Ok(Answer::No))
}

/// Runs what an effect does to the game as it applies (`ReplacementThen`): the modified event, every part of which meets only
/// the effects that have not yet had their opportunity at the event (CR 614.5), `applied`. What it does, the effect's
/// controller does (CR 609.1): the counters it puts are theirs (`Cx::doer`).
fn run_then(cx: &mut Cx, applied: &[Applied], controller: PlayerId, then: impl FnOnce(&mut Cx)) {
    let outer = std::mem::replace(&mut cx.lineage, applied.to_vec());
    cx.acting_as(controller, then);
    cx.lineage = outer;
}

/// The counters a replacement effect gives a card arriving in a zone other than the battlefield (the void counter of
/// Dauthi Voidwalker) are put by the effect's controller (CR 609.1); those a permanent enters with stay its controller's
/// unless the effect names another (CR 122.6a).
fn note_counters_by(before: &Proposal, after: &mut Proposal, controller: PlayerId) {
    if let (Proposal::ZoneChange(b), Proposal::ZoneChange(a)) = (before, after)
        && a.counters != b.counters
        && !a.to.is(ZoneKind::Battlefield)
    {
        a.counters_by = controller;
    }
}

/// CR 614.9: an invalid redirection does nothing; it does not replace valid damage with no damage.
/// Replacement functions are pure, so an invalid effect is neither offered nor consumed nor followed by its `then`.
fn valid_redirection(q: &dyn Query, before: &Proposal, apply: impl FnOnce(&mut Proposal)) -> bool {
    let Proposal::Damage(original) = before else { return true };
    let mut after = *before;
    apply(&mut after);
    let Proposal::Damage(changed) = after else { unreachable!("a replacement cannot change the event kind") };
    original.target == changed.target || damage_recipient(q, original.target) && damage_recipient(q, changed.target)
}

fn damage_recipient(q: &dyn Query, target: Target) -> bool {
    match target {
        Target::Player(p) => q.state().players.get(p.index()).is_some_and(|p| !p.lost),
        Target::Object(id) if id.index() < q.state().objects.len() && live(q.state(), id) && q.is_on_battlefield(id) => {
            let chars = q.chars(id);
            chars.is(CardType::Creature) || chars.is(CardType::Planeswalker) || chars.is(CardType::Battle)
        }
        _ => false,
    }
}

/// Applies the effects to the proposal, one at a time: each is judged again after the one before it has changed the
/// event (CR 616.1f), and none applies twice (CR 614.5), nor to an event that is part of what one of them made of an event
/// it applied to already (`Cx::lineage`).
fn apply(cx: &mut Cx, proposal: &mut Proposal) {
    apply_to(cx, proposal, None);
}

/// `apply`, to a permanent's entry with its own replacements still to apply (`Entering::own`) where `entry` is given.
fn apply_to(cx: &mut Cx, proposal: &mut Proposal, mut entry: Option<&mut Entering>) {
    let mut applied: Vec<Applied> = cx.lineage.clone();
    loop {
        // An entering permanent is judged as it would exist with the changes of the replacements applied so far (CR 614.12).
        if let Proposal::ZoneChange(z) = *proposal {
            crate::ops::rejudge_entering(cx, &z);
        }
        let found = candidates(cx, proposal, &applied, entry.as_deref());
        if found.is_empty() {
            return;
        }
        let pick = if found.len() > 1 && !found.iter().all(|c| prevents_all(cx, c)) { choose(cx, proposal, &found) } else { 0 };
        let Candidate { source, index, origin } = found[pick];
        applied.push((source, index, origin));
        let start = *proposal;
        match origin {
            Origin::Printed => {
                // Declined, it has had its opportunity, and the event goes on as it was (CR 614.5, 616.1).
                if !printed_taken(cx, source, index, proposal) {
                    continue;
                }
                apply_printed(cx, proposal, source, index, &applied);
            }
            Origin::Granted(timestamp) => {
                let Some((&e, def)) = granted(cx, timestamp) else { continue };
                apply_granted(cx, proposal, source, e, def, &applied, true);
            }
            // "That player discards it, but exiles it instead of putting it into their graveyard" (CR 702.35a): the
            // graveyard is its owner's (CR 400.3), and so is the exile it goes to.
            Origin::Madness => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.to = Zone::exile(z.to.player);
                    z.madness = true;
                }
            }
            // Its own (CR 614.1c-d), to the entry as it now is: whether it enters tapped, and under whom, are the event's;
            // the rest it changes is the entry's. It is judged again as it would exist with it applied (CR 614.12), or, sent
            // elsewhere instead, is no entry at all.
            Origin::Enters(_) => {
                if let (Some(e), Proposal::ZoneChange(z)) = (entry.as_deref_mut(), &mut *proposal) {
                    crate::ops::apply_own_entering(cx, e, z.controller, z.tapped);
                    z.controller = e.controller;
                    z.tapped = e.tapped;
                    if let Some(instead) = e.instead {
                        *z = sent_instead(cx, *z, instead);
                        cx.entering = None;
                    } else {
                        cx.entering = Some(crate::ops::as_it_would_exist(cx, e));
                    }
                }
            }
            // CR 702.34a, 614.1a: exiled instead, in its owner's exile (CR 400.3).
            Origin::Flashback | Origin::ExileInstead | Origin::Unearth | Origin::Finality => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.to = Zone::exile(cx.s().obj(z.object).owner);
                    // The dream counter goes on as the card arrives (CR 122.2: counters do not survive a zone change), put by the
                    // spell's controller whose ability had it exiled (CR 609.1).
                    if matches!(origin, Origin::ExileInstead) && z.exile_instead == ExileInstead::Dream {
                        z.counters[CounterKind::Dream as usize] += 1;
                        z.counters_by = cx.s().obj(z.object).controller;
                    }
                }
            }
            // CR 715.3d: its controller exiles it, in its owner's exile, and may play the card from there.
            Origin::Adventure => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.to = Zone::exile(cx.s().obj(z.object).owner);
                    z.on_adventure = true;
                }
            }
            // CR 720.3d: shuffled into its owner's library.
            Origin::Omen => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.to = Zone::library(cx.s().obj(z.object).owner);
                    z.shuffle = true;
                }
            }
            // CR 702.88a: exiled, and cast again at its controller's next upkeep.
            Origin::Rebound => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.to = Zone::exile(cx.s().obj(z.object).owner);
                    z.rebounding = true;
                }
            }
            // CR 701.19a: the destruction never happens; the permanent is regenerated instead (`ops::move_together`).
            Origin::Regeneration => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.regenerated = true;
                }
            }
            // CR 122.1c: the destruction never happens (CR 614.6); a shield counter is removed from the permanent instead
            // (`ops::move_together`). Or the damage to it is prevented, if it can be (CR 615.12), and a shield counter is
            // removed from it - once for the simultaneous damage event, however many sources deal it damage (CR 120.4).
            Origin::ShieldCounter => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.shielded = true;
                } else if let Proposal::Damage(d) = proposal {
                    if damage_can_be_prevented(cx, d) {
                        prevent(cx, d, d.amount, source);
                    }
                    if d.shielded != Some(source) {
                        d.shielded = Some(source);
                        crate::counters::remove(cx, source, CounterKind::Shield, 1);
                    }
                }
            }
            // CR 614.1d: it enters tapped.
            Origin::PlayedTapped => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.tapped = true;
                }
            }
            // CR 702.16e, 615.1: the damage is prevented, all of it.
            Origin::Protection => {
                if let Proposal::Damage(d) = proposal {
                    prevent(cx, d, d.amount, source);
                }
            }
            // CR 615.1: a static ability's prevention, of all the damage or of as much as it says, of the damage as it now is
            // (CR 616.1f).
            Origin::Prevention => {
                let asker = Asker { source, controller: controller_of(cx, source, proposal), by: None, spell: false };
                if let Proposal::Damage(d) = proposal {
                    let def = &cx.cards().get(card_of(cx, source)).preventions[index as usize];
                    let amount = def.amount.map_or(d.amount, |how_much| how_much(&*cx, &asker, d).min(d.amount));
                    prevent(cx, d, amount, source);
                }
            }
            Origin::GrantedPrevention(timestamp) => {
                if let Proposal::Damage(d) = proposal {
                    let at = cx.s().effects.iter().position(|e| e.timestamp == timestamp).expect("the effect that applies");
                    let mut e = cx.s().effects[at];
                    let EffectKind::Prevention { card, index, to, shield } = e.kind else { unreachable!("a prevention effect") };
                    match shield {
                        Shield::All => prevent(cx, d, d.amount, source),
                        // CR 615.7: each 1 damage prevented reduces the shield by 1; once it is 0 the rest is dealt, and
                        // the effect is used up.
                        Shield::Next(left) => {
                            let prevented = left.min(d.amount);
                            prevent(cx, d, prevented, source);
                            if prevented == left {
                                let effects_before = cx.s().effects.len();
                                cx.st().effects_retain(|f| f.timestamp != timestamp);
                                // An effect that ends changes the game with no event (CR 603.8).
                                if cx.s().effects.len() < effects_before {
                                    cx.state_changed();
                                }
                            } else {
                                e.kind = EffectKind::Prevention { card, index, to, shield: Shield::Next(left - prevented) };
                                cx.st().effect_set(at, e);
                            }
                        }
                    }
                }
            }
        }
        unlink_if_sent_elsewhere(&start, proposal);
    }
}

/// The player whose "you" a source's printed effect has: its controller - for the permanent entering, the player it enters
/// under - or, for one functioning from a graveyard, its owner (CR 108.4a).
fn printed_controller(cx: &Cx, source: ObjectId, index: u8, proposal: &Proposal) -> PlayerId {
    let def = &cx.cards().get(source_card(cx, source)).replacements[index as usize];
    if def.in_graveyard { cx.s().obj(source).owner } else { controller_of(cx, source, proposal) }
}

/// Whether a source's printed effect is taken as it applies: one that is not optional always is; an optional one ("you
/// may ... instead", CR 614.1a) is its controller's to take (`takes`).
fn printed_taken(cx: &mut Cx, source: ObjectId, index: u8, proposal: &Proposal) -> bool {
    let Some(question) = cx.cards().get(source_card(cx, source)).replacements[index as usize].optional else { return true };
    let controller = printed_controller(cx, source, index, proposal);
    takes(cx, controller, source, question)
}

/// A source's printed effect, taken, applied to the event: what it changes of it, unless that is a redirection that can't be
/// (CR 614.9), and what it does to the game as it applies (`run_then`).
fn apply_printed(cx: &mut Cx, proposal: &mut Proposal, source: ObjectId, index: u8, applied: &[Applied]) {
    let def = &cx.cards().get(source_card(cx, source)).replacements[index as usize];
    let (apply, then) = (def.apply, def.then);
    let controller = printed_controller(cx, source, index, proposal);
    let asker = Asker { source, controller, by: None, spell: false };
    let before = *proposal;
    if !valid_redirection(&*cx, &before, |after| apply(&*cx, &asker, after)) {
        return;
    }
    apply(&*cx, &asker, proposal);
    note_counters_by(&before, proposal, controller);
    if let Some(then) = then {
        let after = *proposal;
        run_then(cx, applied, controller, |cx| then(cx, &asker, &before, &after));
    }
}

/// A lasting effect a spell or ability made (CR 611.2), `e` with its definition `def`, applied to the event as a printed one
/// is (`apply_printed`). One that applies "the next time" is used up by the event (CR 614.1a): here, where `use_up`; by the
/// caller once it has applied to every part of the event, where not. Whether it applied.
fn apply_granted(
    cx: &mut Cx,
    proposal: &mut Proposal,
    source: ObjectId,
    e: ContinuousEffect,
    def: &'static GrantedReplacementDef,
    applied: &[Applied],
    use_up: bool,
) -> bool {
    let (apply, then) = (def.apply, def.then);
    let asker = Asker { source, controller: e.controller, by: None, spell: false };
    let before = *proposal;
    if !valid_redirection(&*cx, &before, |after| apply(&*cx, &asker, e.affected, after)) {
        return false;
    }
    apply(&*cx, &asker, e.affected, proposal);
    if use_up && def.once {
        used_up(cx, e.timestamp);
    }
    note_counters_by(&before, proposal, e.controller);
    if let Some(then) = then {
        let after = *proposal;
        run_then(cx, applied, e.controller, |cx| then(cx, &asker, e.affected, &before, &after));
    }
    true
}

/// A "the next time" effect, used up by the event it applied to (CR 614.1a), ends.
fn used_up(cx: &mut Cx, timestamp: u32) {
    let effects_before = cx.s().effects.len();
    cx.st().effects_retain(|f| f.timestamp != timestamp);
    // An effect that ends changes the game with no event (CR 603.8).
    if cx.s().effects.len() < effects_before {
        cx.state_changed();
    }
}

/// Applies the effects to one event that is several proposals - the groups of a token creation under one player, a
/// definition and copy each (`token_batch`) - as the one event it is (CR 614.1a). At each step the candidates are the effects
/// that apply to any of the groups and have not yet applied to the event; the affected player, the one whose tokens they
/// are, chooses one where there are several (CR 616.1); its "you may" is asked once; and it applies to every group it
/// applies to then, after which it has applied to the whole event and applies to none of it again (CR 614.5).
fn apply_to_parts(cx: &mut Cx, parts: &mut [Proposal]) {
    let mut applied: Vec<Applied> = cx.lineage.clone();
    loop {
        let found: Vec<Vec<Candidate>> = parts.iter().map(|p| candidates(cx, p, &applied, None)).collect();
        let mut all: Vec<Candidate> = found.iter().flatten().copied().collect();
        all.sort_by_key(|c| (c.source, c.index, c.origin));
        all.dedup();
        let Some(&first) = all.first() else { return };
        let Candidate { source, index, origin } =
            if all.len() > 1 && !all.iter().all(|c| prevents_all(cx, c)) { all[choose(cx, &parts[0], &all)] } else { first };
        applied.push((source, index, origin));
        let meets = |j: usize| found[j].iter().any(|c| (c.source, c.index, c.origin) == (source, index, origin));
        match origin {
            Origin::Printed => {
                let Some(at) = (0..parts.len()).find(|&j| meets(j)) else { continue };
                if !printed_taken(cx, source, index, &parts[at]) {
                    continue;
                }
                for j in (0..parts.len()).filter(|&j| meets(j)) {
                    apply_printed(cx, &mut parts[j], source, index, &applied);
                }
            }
            Origin::Granted(timestamp) => {
                let Some((&e, def)) = granted(cx, timestamp) else { continue };
                let mut took = false;
                for j in (0..parts.len()).filter(|&j| meets(j)) {
                    took |= apply_granted(cx, &mut parts[j], source, e, def, &applied, false);
                }
                if took && def.once {
                    used_up(cx, timestamp);
                }
            }
            // Only a printed or a lasting effect replaces anything but a zone change or damage.
            _ => {}
        }
    }
}

/// A card is exiled with a replacement effect's source only as the direct result of that effect (CR 607.2b): an effect that
/// applies afterwards and changes where it goes, without linking it to a source of its own, leaves it linked to none.
fn unlink_if_sent_elsewhere(before: &Proposal, after: &mut Proposal) {
    if let (Proposal::ZoneChange(b), Proposal::ZoneChange(a)) = (before, after)
        && a.to != b.to
        && a.exiled_with == b.exiled_with
    {
        a.exiled_with = None;
    }
}

/// What one effect does to the event, and nothing else: its `apply`, with the "you" it has (`apply` does this and the rest).
/// For the outcomes an event could have (`mana_outcomes`), which do nothing to the game.
fn reshape(cx: &Cx, c: &Candidate, proposal: &mut Proposal) {
    match c.origin {
        Origin::Printed => {
            let def = &cx.cards().get(source_card(cx, c.source)).replacements[c.index as usize];
            let asker = Asker { source: c.source, controller: controller_of(cx, c.source, proposal), by: None, spell: false };
            (def.apply)(cx, &asker, proposal);
        }
        Origin::Granted(timestamp) => {
            let Some((e, def)) = granted(cx, timestamp) else { return };
            (def.apply)(cx, &Asker { source: c.source, controller: e.controller, by: None, spell: false }, e.affected, proposal);
        }
        // No other kind of effect applies to the mana a permanent produces.
        _ => {}
    }
}

/// Whether a replacement effect could apply to the mana a permanent tapped for mana produces (CR 106.12b): a permanent on the
/// battlefield whose card has one (`CardTable::mana_replaces`), or one a spell or ability made, still lasting. The guard that
/// keeps a tap for mana, and the advice on what one would produce, free of the production's proposal on nearly every board.
pub(crate) fn mana_may_be_replaced(cx: &Cx) -> bool {
    mana_may_be_replaced_in(cx.s(), cx.cards())
}

/// `mana_may_be_replaced`, of a position.
pub(crate) fn mana_may_be_replaced_in(s: &GameState, cards: &CardTable) -> bool {
    s.battlefield().iter().any(|&id| cards.mana_replaces[cards.current(s.obj(id)).0 as usize])
        || s.effects.iter().any(|e| match e.kind {
            EffectKind::Replacement { card, index } | EffectKind::ReplacementRule { card, index } => {
                cards.get(card).granted_replacements[index as usize].kind == ReplacementKind::ManaProduction
            }
            _ => false,
        })
}

/// The mana a permanent tapped for mana produces (CR 106.12b): what is left of `production` once every replacement effect that
/// applies has, in the order its player chooses (CR 616.1), each once (CR 614.5).
pub(crate) fn mana_production(cx: &mut Cx, production: ManaProduction) -> ManaProduction {
    let mut proposal = Proposal::ManaProduction(production);
    apply(cx, &mut proposal);
    match proposal {
        Proposal::ManaProduction(production) => production,
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// Every outcome the replacement effects could leave of the mana a permanent tapped for mana would produce (CR 106.12b,
/// 616.1): each order its player could apply those that apply in, each judged again on the event as the one before left it
/// (CR 616.1f), and each "you may" taken or declined - `production` itself where none applies. "Taking into account any
/// applicable replacement effects in any possible order" is what a permanent could produce (CR 106.7). Judged on the board as
/// it is, as `mana_production` would judge it, and doing nothing: an effect's `then` is not run. What the player has to pay
/// with (`actions::available`), the types a permanent could produce, and what a mana ability's row says it adds are read from
/// it, so they agree with the mana the tap produces.
pub(crate) fn mana_outcomes(cx: &Cx, production: ManaProduction) -> Vec<ManaProduction> {
    fn walk(cx: &Cx, proposal: Proposal, applied: &mut Vec<Applied>, seen: &mut Vec<(Vec<Applied>, Proposal)>, out: &mut Vec<ManaProduction>) {
        // The same effects applied, whatever their order, to the same event: the same outcomes from here (doublers alike).
        let mut key = applied.clone();
        key.sort();
        if seen.iter().any(|(a, p)| *a == key && *p == proposal) {
            return;
        }
        seen.push((key, proposal));
        let found = candidates(cx, &proposal, applied, None);
        if found.is_empty() {
            if let Proposal::ManaProduction(m) = proposal
                && !out.contains(&m)
            {
                out.push(m);
            }
            return;
        }
        for c in found {
            applied.push((c.source, c.index, c.origin));
            // Declined, it has had its opportunity, and the event goes on as it was (CR 614.5, 616.1).
            let optional = match c.origin {
                Origin::Printed => cx.cards().get(source_card(cx, c.source)).replacements[c.index as usize].optional.is_some(),
                _ => false,
            };
            if optional {
                walk(cx, proposal, applied, seen, out);
            }
            let mut next = proposal;
            reshape(cx, &c, &mut next);
            walk(cx, next, applied, seen, out);
            applied.pop();
        }
    }
    let mut out = Vec::new();
    walk(cx, Proposal::ManaProduction(production), &mut cx.lineage.clone(), &mut Vec::new(), &mut out);
    out
}

/// Whether a replacement effect might change damage about to be dealt ("it deals that much damage plus 1 instead"): one a
/// spell or ability made, or a permanent's on the battlefield with a damage replacement. Asked before the damage is
/// proposed, so it only says the amounts may not stay as they are.
pub(crate) fn damage_may_change(cx: &Cx) -> bool {
    let cards = cx.cards();
    cx.s().effects.iter().any(|e| match e.kind {
        EffectKind::Replacement { card, index } | EffectKind::ReplacementRule { card, index } => {
            cards.get(card).granted_replacements[index as usize].kind == ReplacementKind::Damage
        }
        _ => false,
    }) || cx.s().battlefield().iter().any(|&id| {
        live(cx.s(), id)
            && !cx.lost_all_abilities(id)
            && cards.get(card_of(cx, id)).replacements.iter().any(|r| r.kind == ReplacementKind::Damage)
    })
}

/// The prevention shields that would apply to this damage (CR 615.7), taken together: the source of the first, and how much
/// they have left to prevent between them. None where no shield applies, or the damage can't be prevented (CR 615.12).
pub(crate) fn shields(cx: &Cx, d: &DamageEvent) -> Option<(ObjectId, u16)> {
    if !cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::Prevention { shield: Shield::Next(_), .. })) {
        return None;
    }
    if !crate::ops::damage_can_be_prevented(cx, d.source, d.combat) {
        return None;
    }
    let mut found: Option<(ObjectId, u16)> = None;
    for e in &cx.s().effects {
        let EffectKind::Prevention { card, index, to, shield: Shield::Next(left) } = e.kind else { continue };
        if !(to == Target::None || to == d.target) || (to.object().is_some() && !live(cx.s(), e.affected)) {
            continue;
        }
        let def = &cx.cards().get(card).granted_preventions[index as usize];
        if (def.applies)(cx, &Asker { source: e.source, controller: e.controller, by: None, spell: false }, e.affected, d) {
            found = Some(found.map_or((e.source, left), |(s, n)| (s, n.saturating_add(left))));
        }
    }
    found
}

/// Whether a prevention effect may prevent this damage (CR 615.12).
fn damage_can_be_prevented(cx: &Cx, d: &DamageEvent) -> bool {
    crate::ops::damage_can_be_prevented(cx, d.source, d.combat)
}

/// Prevents `amount` of the damage (CR 615.1) by the effect of `by`, and records it (CR 615.6, 615.13).
fn prevent(cx: &mut Cx, d: &mut DamageEvent, amount: u16, by: ObjectId) {
    if amount == 0 {
        return;
    }
    d.amount -= amount;
    cx.emit(Event::DamagePrevented { source: d.source, target: d.target, amount, combat: d.combat, by });
}

/// An object about to change zones (CR 614.1a, 614.1c): what remains of the move once every effect has applied.
pub fn zone_change(cx: &mut Cx, change: ZoneChange) -> ZoneChange {
    if !zone_change_possible(cx, &change) {
        return change;
    }
    let mut proposal = Proposal::ZoneChange(change);
    apply(cx, &mut proposal);
    match proposal {
        Proposal::ZoneChange(change) => change,
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// Counters that would be put on a permanent, or that one enters with (CR 122.6, 614.1a): how many of each kind
/// remain once every effect has applied. The permanent is judged as it is now, or as it would exist on the
/// battlefield if it is entering (CR 614.12). `putter` puts them (CR 122.6a). `effect` says an effect puts them, not a
/// cost or a turn-based action (`CounterPlacement::effect`). Called by the put path, `crate::counters`, alone.
pub(crate) fn counters(
    cx: &mut Cx,
    object: ObjectId,
    controller: PlayerId,
    putter: PlayerId,
    entering: bool,
    effect: bool,
    counts: [i16; CounterKind::COUNT],
) -> [i16; CounterKind::COUNT] {
    if counts.iter().all(|&n| n <= 0) {
        return counts;
    }
    // CR 101.2, 122.6: counters of a kind the permanent can't have put on it are not put on it (Melira, Sylvok Outcast's
    // "creatures you control can't have -1/-1 counters put on them"), entering with them included; no replacement effect sees
    // them, and none adds them back.
    let counts = without_banned(cx, object, controller, counts);
    if !possible(cx, Some(object)) || counts.iter().all(|&n| n <= 0) {
        return counts;
    }
    let mut proposal = Proposal::Counters(CounterPlacement { object, controller, entering, counts, putter, effect });
    apply(cx, &mut proposal);
    match proposal {
        Proposal::Counters(placement) => without_banned(cx, object, controller, placement.counts),
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// `counts` less every kind `object`, controlled by `controller`, can't have put on it (CR 101.2, 122.6).
fn without_banned(cx: &Cx, object: ObjectId, controller: PlayerId, mut counts: [i16; CounterKind::COUNT]) -> [i16; CounterKind::COUNT] {
    for (i, n) in counts.iter_mut().enumerate() {
        if *n > 0 && crate::static_rules::counters_banned(cx, cx.cards(), object, controller, CounterKind::ALL[i]) {
            *n = 0;
        }
    }
    counts
}

/// Counters `putter` would put on a player (CR 122.6, 122.6a): how many of each kind remain once every effect has applied.
/// Called by the put path, `crate::counters`, alone.
pub(crate) fn player_counters(cx: &mut Cx, player: PlayerId, putter: PlayerId, counts: crate::counters::PlayerCounts) -> crate::counters::PlayerCounts {
    if !possible(cx, None) {
        return counts;
    }
    let crate::counters::PlayerCounts { poison, energy, experience } = counts;
    let mut proposal = Proposal::PlayerCounters(PlayerCounters { player, putter, poison, energy, experience });
    apply(cx, &mut proposal);
    match proposal {
        Proposal::PlayerCounters(c) => crate::counters::PlayerCounts { poison: c.poison, energy: c.energy, experience: c.experience },
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// An instruction to draw cards (CR 121.2a): how many are drawn once every effect that changes the number has
/// applied, before any individual draw.
pub fn draw_count(cx: &mut Cx, player: PlayerId, count: u32) -> u32 {
    if !possible(cx, None) {
        return count;
    }
    let mut proposal = Proposal::DrawInstruction(DrawInstruction { player, count });
    apply(cx, &mut proposal);
    match proposal {
        Proposal::DrawInstruction(d) => d.count,
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// One card about to be drawn (CR 121.1, 121.6): whether an effect has replaced the draw, so that nothing is drawn. What
/// the effect does instead is done as it applies, before the draws of an instruction to draw several go on (CR 121.6b).
pub fn draw_replaced(cx: &mut Cx, player: PlayerId) -> bool {
    if !possible(cx, None) && !graveyard_replaces_draws(cx, player) {
        return false;
    }
    let mut proposal = Proposal::Draw(CardDraw { player, replaced: false });
    apply(cx, &mut proposal);
    matches!(proposal, Proposal::Draw(CardDraw { replaced: true, .. }))
}

/// Apply token-creation replacements to a creation, grouped by definition, copy and controller (CR 614.1a). Return the
/// resulting counts and token types. The creation is one event: the tokens each player would create are replaced together,
/// that player ordering the effects (CR 616.1), each asked once and applied once to all of them (`apply_to_parts`).
pub fn token_batch(cx: &mut Cx, tokens: &[(CardId, Option<Copiable>, PlayerId)]) -> Option<Vec<(CardId, Option<Copiable>, PlayerId)>> {
    if !possible(cx, None) {
        return None;
    }
    // The runs of one definition, copy and controller, in the order given.
    let mut groups: Vec<TokenCreation> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let (token, copy, controller) = tokens[i];
        let mut count = 0u32;
        while i < tokens.len() && tokens[i] == (token, copy, controller) {
            count += 1;
            i += 1;
        }
        groups.push(TokenCreation { controller, token, copy, count });
    }
    let mut after = groups.clone();
    // Each player's, the affected player's (CR 616.1), in the order they first appear.
    let mut done: Vec<PlayerId> = Vec::new();
    for g in &groups {
        if done.contains(&g.controller) {
            continue;
        }
        done.push(g.controller);
        let theirs: Vec<usize> = (0..groups.len()).filter(|&j| groups[j].controller == g.controller).collect();
        let mut parts: Vec<Proposal> = theirs.iter().map(|&j| Proposal::TokenCreation(groups[j])).collect();
        apply_to_parts(cx, &mut parts);
        for (&j, part) in theirs.iter().zip(parts) {
            let Proposal::TokenCreation(t) = part else { unreachable!("an effect cannot change the kind of the event") };
            after[j] = t;
        }
    }
    // Keep the final replacement’s token definition and copy values (CR 614.1a).
    let changed = groups.iter().zip(&after).any(|(b, a)| a.count != b.count || a.token != b.token || a.copy != b.copy);
    let mut out: Vec<(CardId, Option<Copiable>, PlayerId)> = Vec::with_capacity(tokens.len());
    for (b, a) in groups.iter().zip(&after) {
        out.extend(std::iter::repeat((a.token, a.copy, b.controller)).take(a.count as usize));
    }
    changed.then_some(out)
}

/// Damage about to be dealt (CR 614.1a, 120.3): what remains of it once every effect has applied - how much, and whom it
/// is dealt to, which a redirection ("is dealt to this creature instead") changes, after which the effects not yet
/// applied are judged again against the new recipient and its controller orders them (CR 614.6, 616.1f).
pub fn damage(cx: &mut Cx, event: DamageEvent) -> DamageEvent {
    if !possible(cx, None) && shield_damage_applies(cx, &event).is_none() {
        return event;
    }
    let mut proposal = Proposal::Damage(event);
    apply(cx, &mut proposal);
    match proposal {
        Proposal::Damage(event) => event,
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// Life `player` is about to gain (`amount` above zero) or lose (below zero) (CR 119.3, 614.1a): what remains of it once
/// every effect has applied, in the order the player chooses (CR 616.1), each judged again on the event as the one
/// before it left it (CR 616.1f), so a gain turned into a loss meets the effects on losing life. Zero is no event
/// (CR 119.10).
pub fn life_change(cx: &mut Cx, player: PlayerId, amount: i32) -> i32 {
    if !possible(cx, None) {
        return amount;
    }
    let mut proposal = Proposal::LifeChange(LifeChange { player, amount });
    apply(cx, &mut proposal);
    match proposal {
        Proposal::LifeChange(l) => l.amount,
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Game;
    use cantrip_state::{Frame, FrameKind};

    const ME: PlayerId = PlayerId(0);

    fn you_would_draw(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
        matches!(proposal, Proposal::Draw(d) if d.player == asker.controller)
    }

    fn instead(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
        if let Proposal::Draw(d) = proposal {
            d.replaced = true;
        }
    }

    fn draw_two(g: &mut dyn cantrip_api::Game, asker: &Asker, _: &Proposal, _: &Proposal) {
        g.draw(asker.controller, 2);
    }

    fn gain_one(g: &mut dyn cantrip_api::Game, asker: &Asker, _: &Proposal, _: &Proposal) {
        g.gain_life(asker.controller, 1);
    }

    /// "Each player can't draw more than one card each turn."
    fn one_each_turn(q: &dyn Query, _: &Asker, player: PlayerId, n: u32) -> bool {
        q.history().drawn(player) as u32 + n > 1
    }

    static FILLER: CardDef = CardDef { name: "Filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
    /// "If you would draw a card, draw two cards instead."
    static DOUBLER: CardDef = CardDef {
        name: "Doubler",
        types: TypeSet::of(&[CardType::Enchantment]),
        replacements: &[replacement(ReplacementKind::Draw, you_would_draw, instead, "draws two cards instead").then(draw_two)],
        ..CardDef::BLANK
    };
    /// "If you would draw a card, you gain 1 life instead."
    static GAINER: CardDef = CardDef {
        name: "Gainer",
        types: TypeSet::of(&[CardType::Enchantment]),
        replacements: &[replacement(ReplacementKind::Draw, you_would_draw, instead, "gains 1 life instead").then(gain_one)],
        ..CardDef::BLANK
    };
    static ASK: Question = Question::new("Gain", "gain 1 life instead?", "", "Gain", "Draw");
    /// "If you would draw a card, you may gain 1 life instead", functioning in its owner's graveyard alone, as dredge does.
    static GRAVE_GAINER: CardDef = CardDef {
        name: "Grave Gainer",
        types: TypeSet::of(&[CardType::Creature]),
        replacements: &[replacement(ReplacementKind::Draw, you_would_draw, instead, "may gain 1 life instead")
            .then(gain_one)
            .optional(&ASK)
            .in_graveyard()],
        ..CardDef::BLANK
    };
    fn an_opponent_would_draw(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
        matches!(proposal, Proposal::Draw(d) if d.player != asker.controller)
    }
    static SKIP: Question = Question::new("Skip", "have that draw not happen?", "", "Skip", "Let it");
    /// "If an opponent would draw a card, you may have them draw nothing instead."
    static DENIER: CardDef = CardDef {
        name: "Denier",
        types: TypeSet::of(&[CardType::Enchantment]),
        replacements: &[replacement(ReplacementKind::Draw, an_opponent_would_draw, instead, "draws nothing instead").optional(&SKIP)],
        ..CardDef::BLANK
    };
    fn goes_to_the_graveyard(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
        matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Graveyard))
    }
    fn nothing_changes(_: &dyn Query, _: &Asker, _: &mut Proposal) {}
    /// A replacement effect of something else than draws, as Rest in Peace's.
    static PEACE: CardDef = CardDef {
        name: "Peace",
        types: TypeSet::of(&[CardType::Enchantment]),
        replacements: &[replacement(ReplacementKind::ZoneChange, goes_to_the_graveyard, nothing_changes, "changes nothing")],
        ..CardDef::BLANK
    };
    static LIMITER: CardDef =
        CardDef { name: "Limiter", types: TypeSet::of(&[CardType::Enchantment]), cant_draw: &[one_each_turn], ..CardDef::BLANK };

    /// A game whose first player has each of `cards` in the zone beside it, a hand of seven Fillers and ten more in the
    /// library, and a frame whose run is given `answers`.
    fn game(cards: &[(&'static CardDef, Zone)], answers: Vec<Answer>) -> Game {
        let table: &'static CardTable =
            Box::leak(Box::new(CardTable::new([&FILLER, &DOUBLER, &GAINER, &GRAVE_GAINER, &LIMITER, &DENIER, &PEACE])));
        let deck: Vec<CardId> = std::iter::repeat_n(table.id_of(&FILLER), 17).collect();
        let mut game = Game::new(table, &crate::Setup { decks: vec![deck, Vec::new()], ..Default::default() });
        for &(def, to) in cards {
            let id = game.state.push_obj(Obj::new(table.id_of(def), ME, to));
            game.state.zone_push(to, id);
        }
        game.state.frame_push(Frame::new(FrameKind::Settle));
        let at = game.state.agenda.len() - 1;
        game.state.update_frame(at, |f| f.answers = answers.into_iter().map(|a| (None, a)).collect());
        game
    }

    /// Has the first player draw `count` cards: their hand, library and life afterwards.
    fn draw(game: &mut Game, count: u32) -> (usize, usize, i32) {
        let frame = game.state.agenda.len() - 1;
        let mut cx = Cx { g: game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        crate::ops::draw_cards(&mut cx, ME, count);
        assert!(cx.s().agenda[frame].pending.is_none(), "every question was answered");
        let s = cx.s();
        (s.zone(Zone::hand(ME)).len(), s.zone(Zone::library(ME)).len(), s.player(ME).life)
    }

    /// CR 614.5: each of two "draw two cards instead" effects has one opportunity at a draw and at the draws it is replaced
    /// by. The first replaces the draw with two; the second replaces each of those with two more, and neither applies again,
    /// so one draw is four cards (Thought Reflection's ruling). The player is asked once, at the first draw.
    #[test]
    fn two_effects_that_draw_two_instead_make_one_draw_four_cards() {
        let mut game = game(&[(&DOUBLER, Zone::BATTLEFIELD), (&DOUBLER, Zone::BATTLEFIELD)], vec![Answer::Index(0)]);
        assert_eq!(draw(&mut game, 1), (7 + 4, 10 - 4, 20));
    }

    /// CR 614.6: a replaced draw never happens, so no other effect replaces it as well: of two "gain 1 life instead"
    /// effects only the one chosen applies.
    #[test]
    fn a_draw_replaced_is_not_replaced_again() {
        let mut game = game(&[(&GAINER, Zone::BATTLEFIELD), (&GAINER, Zone::BATTLEFIELD)], vec![Answer::Index(1)]);
        assert_eq!(draw(&mut game, 1), (7, 10, 21));
    }

    /// CR 121.2b, 614.17c: a card the player can't draw is not drawn and its draw is not replaced. The first draw of the
    /// turn is replaced by two, of which only the first is drawn; with the life gain instead, the first draw is replaced
    /// and so not drawn (CR 614.6), the next is too, and once a card has been drawn the draw after it can't happen, so
    /// nothing replaces it.
    #[test]
    fn a_draw_that_cant_happen_is_not_replaced() {
        let mut game = game(&[(&LIMITER, Zone::BATTLEFIELD), (&DOUBLER, Zone::BATTLEFIELD)], Vec::new());
        assert_eq!(draw(&mut game, 1), (8, 9, 20));
        assert_eq!(draw(&mut game, 1), (8, 9, 20));
        let mut game = game_with(&[(&LIMITER, Zone::BATTLEFIELD), (&GAINER, Zone::BATTLEFIELD)]);
        assert_eq!(draw(&mut game, 2), (7, 10, 22));
    }

    /// CR 121.4, 121.6: "draw 2^32 or more cards" (Mathemagics) against an empty library returns at once with a replacement
    /// effect of another event (Rest in Peace) in play; only an effect that would replace this player's draw keeps it going.
    #[test]
    fn an_unrelated_replacement_effect_does_not_keep_an_empty_library_drawing() {
        let mut game = game_with(&[(&PEACE, Zone::BATTLEFIELD)]);
        assert_eq!(draw(&mut game, 10), (17, 0, 20));
        assert_eq!(draw(&mut game, u32::MAX), (17, 0, 20));
        assert!(game.state.player(ME).drew_from_empty);
    }

    fn game_with(cards: &[(&'static CardDef, Zone)]) -> Game {
        game(cards, Vec::new())
    }

    /// CR 702.52a, 614.1a: an ability that functions in its card's graveyard replaces its owner's draws from there, and only
    /// from there; "you may" asks, and declined, the draw goes on as it was (CR 616.1).
    #[test]
    fn a_graveyard_replacement_functions_only_in_the_graveyard_and_may_be_declined() {
        let mut game = game(&[(&GRAVE_GAINER, Zone::graveyard(ME))], vec![Answer::Yes]);
        assert_eq!(draw(&mut game, 1), (7, 10, 21));
        let mut game = game_with(&[(&GRAVE_GAINER, Zone::BATTLEFIELD)]);
        assert_eq!(draw(&mut game, 1), (8, 9, 20));
        let mut game = self::game(&[(&GRAVE_GAINER, Zone::graveyard(ME))], vec![Answer::No]);
        assert_eq!(draw(&mut game, 1), (8, 9, 20));
    }

    /// CR 109.5: "you may ... instead" is the choice of the effect's controller, not of the affected player (CR 616.1 gives
    /// that one only the order). Your effect on an opponent's draw asks you, and taken, they draw nothing.
    #[test]
    fn an_optional_replacement_is_chosen_by_its_controller_not_the_affected_player() {
        let mut game = game(&[(&DENIER, Zone::BATTLEFIELD)], Vec::new());
        let them = PlayerId(1);
        let frame = game.state.agenda.len() - 1;
        game.state.update_frame(frame, |f| f.answers = vec![(Some(ME), Answer::Yes)]);
        let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        crate::ops::draw_cards(&mut cx, them, 1);
        assert!(cx.s().agenda[frame].pending.is_none(), "the controller's answer was the one asked for");
        assert!(cx.s().zone(Zone::hand(them)).is_empty(), "the draw was replaced");
    }

    /// A card is exiled with a replacement effect's source only as that effect's direct result (CR 607.2b): an effect that
    /// applies afterwards and sends it elsewhere leaves it unlinked; one that changes something else, or that is the one
    /// linking it, keeps the link.
    #[test]
    fn a_later_effect_that_sends_the_card_elsewhere_unlinks_it() {
        let linked = Linked { object: ObjectId(5), text: CardId(3), acquired: Acquired::OWN };
        let exiling = ZoneChange {
            object: ObjectId(1),
            from: Zone::hand(ME),
            to: Zone::exile(ME),
            reason: ZoneReason::Other,
            controller: ME,
            tapped: false,
            attacking: false,
            counters: [0; CounterKind::COUNT],
            counters_by: ME,
            reveal: false,
            shuffle: false,
            is_card: true,
            madness: false,
            pays_characteristic: false,
            exiled_with: Some(linked),
            flashback: false,
            exile_instead: ExileInstead::Never,
            adventure: false,
            omen: false,
            rebound: false,
            on_adventure: false,
            rebounding: false,
            destroy: false,
            regenerated: false,
            shielded: false,
            replaced: false,
        };
        let link = |p: Proposal| match p {
            Proposal::ZoneChange(z) => z.exiled_with,
            _ => unreachable!(),
        };
        let before = Proposal::ZoneChange(exiling);
        let mut elsewhere = Proposal::ZoneChange(ZoneChange { to: Zone::library(ME), ..exiling });
        unlink_if_sent_elsewhere(&before, &mut elsewhere);
        assert_eq!(link(elsewhere), None, "sent on to a library, it is exiled with nothing");
        let mut revealed = Proposal::ZoneChange(ZoneChange { reveal: true, ..exiling });
        unlink_if_sent_elsewhere(&before, &mut revealed);
        assert_eq!(link(revealed), Some(linked), "still exiled, it stays linked");
        let to_graveyard = Proposal::ZoneChange(ZoneChange { to: Zone::graveyard(ME), exiled_with: None, ..exiling });
        let mut exiled = before;
        unlink_if_sent_elsewhere(&to_graveyard, &mut exiled);
        assert_eq!(link(exiled), Some(linked), "the effect exiling it links it");
    }
}
