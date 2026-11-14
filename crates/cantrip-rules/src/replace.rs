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
use crate::actions::WithWords;
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
    /// Commander owner's replacement of a move into hand or library (CR 903.9b).
    Commander,
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
    /// An instance of riot the entering permanent would have (CR 702.136a-b, `ops::apply_riot`): each is applied separately,
    /// offered while it would have more instances than have applied, and ordered with the others that modify how it enters
    /// (CR 616.1).
    Riot,
    /// "It enters with ... counters on it if it's a ..." of the effect putting the entering permanent onto the battlefield
    /// (CR 614.1c, `Entering::counters_if`): of the permanent itself, offered while it would be one of what it names, and
    /// ordered with the others that modify how it enters (CR 616.1).
    CountersIf,
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

/// What an instance of riot says it does, for the player who orders it with others (CR 616.1, 702.136a).
const RIOT_HINT: &str = "riot: it enters with an additional +1/+1 counter, or gains haste";

/// What the "it enters with ... counters on it if it's a ..." of the effect putting it onto the battlefield says it does
/// (CR 616.1, `Entering::counters_if`).
const COUNTERS_IF_HINT: &str = "it enters with the additional counters the effect putting it onto the battlefield gives it";

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
    change.put_by = None;
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
    apply_to(cx, &mut proposal, Some(entering), None);
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
        && !crate::static_rules::cant_be_regenerated(cx, cx.cards(), z.object)
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

/// The definitions `id`'s printed replacement and prevention effects come from (`source_card`), and, for a merged permanent,
/// each of its other components' (CR 702.140e, `CardTable::texts`): a printed effect's index counts through them all, in
/// this order.
fn source_text_ids<'a>(cx: &'a Cx, id: ObjectId) -> impl Iterator<Item = CardId> + 'a {
    let entering = matches!(cx.entering, Some(w) if w.object == id);
    let first = source_card(cx, id);
    let obj = cx.s().obj(id);
    let rest = if entering || obj.copy.is_none() { None } else { Some(cx.cards().texts(cx.s(), obj).skip(1)) };
    std::iter::once(first).chain(rest.into_iter().flatten())
}

/// The cards of `source_text_ids`, each with the link an effect printed on it is asked with (CR 607.1): none for the text the
/// object has, a merged permanent's other component's own (`CardTable::component_link`), so that component's effect reads
/// what its own card exiled or chose.
fn source_texts_linked<'a>(cx: &'a Cx, id: ObjectId) -> impl Iterator<Item = (&'static CardDef, Option<Linked>)> + 'a {
    let entering = matches!(cx.entering, Some(w) if w.object == id);
    let first = (cx.cards().get(source_card(cx, id)), None);
    // An object with no copiable values of its own (nearly every one) has no text but its card's: nothing more to walk.
    let rest = if entering || cx.s().obj(id).copy.is_none() { None } else { Some(cx.cards().texts_linked(cx.s(), id).skip(1)) };
    std::iter::once(first).chain(rest.into_iter().flatten().map(|(card, link)| (cx.cards().get(card), link)))
}

/// `id`'s printed replacement effects (`source_texts`), numbered as `Candidate::index` names them, each with its link.
fn printed_replacements<'a>(cx: &'a Cx, id: ObjectId) -> impl Iterator<Item = (&'static ReplacementDef, Option<Linked>)> + 'a {
    source_texts_linked(cx, id).flat_map(|(d, by)| d.replacements.iter().map(move |r| (r, by)))
}

/// `id`'s printed prevention effects, numbered through its definitions as `printed_replacements` are.
fn printed_preventions<'a>(cx: &'a Cx, id: ObjectId) -> impl Iterator<Item = (&'static PreventionDef, Option<Linked>)> + 'a {
    source_texts_linked(cx, id).flat_map(|(d, by)| d.preventions.iter().map(move |r| (r, by)))
}

fn nothing_applies(_: &dyn Query, _: &Asker, _: &Proposal) -> bool {
    false
}

fn nothing_done(_: &dyn Query, _: &Asker, _: &mut Proposal) {}

fn nothing_prevented(_: &dyn Query, _: &Asker, _: &DamageEvent) -> bool {
    false
}

/// What a printed effect a candidate named is, once its source no longer has it - one of a merged permanent's components
/// whose text an earlier part of the event changed: it applies to nothing and does nothing.
static NO_REPLACEMENT: ReplacementDef = ReplacementDef {
    fixed_mana: None,
    hint: "",
    kind: ReplacementKind::ZoneChange,
    own_everywhere: false,
    applies: nothing_applies,
    apply: nothing_done,
    then: None,
    damage: None,
    optional: None,
    in_graveyard: false,
};

static NO_PREVENTION: PreventionDef = PreventionDef { hint: "", applies: nothing_prevented, amount: PreventionAmount::All };

/// `id`'s printed replacement effect `index` (`printed_replacements`), and the link it is asked with.
fn printed_replacement(cx: &Cx, id: ObjectId, index: u8) -> &'static ReplacementDef {
    printed_replacements(cx, id).nth(index as usize).map_or(&NO_REPLACEMENT, |(r, _)| r)
}

/// The link `id`'s printed replacement effect `index` is asked with (`printed_replacements`).
fn printed_link(cx: &Cx, id: ObjectId, index: u8) -> Option<Linked> {
    printed_replacements(cx, id).nth(index as usize).and_then(|(_, by)| by)
}

/// `id`'s printed prevention effect `index` (`printed_preventions`), and the link it is asked with.
fn printed_prevention(cx: &Cx, id: ObjectId, index: u8) -> (&'static PreventionDef, Option<Linked>) {
    printed_preventions(cx, id).nth(index as usize).unwrap_or((&NO_PREVENTION, None))
}

/// Whether any effect could apply to an event about `subject`: a permanent with replacements, or the subject's own
/// card, which has some that function from every zone. The guard that keeps the hot path free of replacement work.
pub(crate) fn possible(cx: &Cx, subject: Option<ObjectId>) -> bool {
    if cx.s().commander && subject.is_some_and(|id| cx.s().obj(id).commander) {
        return true;
    }
    // An effect a spell or ability made, which lasts (CR 611.2).
    if cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::Replacement { .. } | EffectKind::ReplacementRule { .. } | EffectKind::Prevention { .. })) {
        return true;
    }
    let cards = cx.cards();
    if !cards.any_replaces {
        return false;
    }
    subject.is_some_and(|id| cards.replaces[source_card(cx, id).0 as usize])
        || cx.s().battlefield().iter().any(|&id| cards.texts(cx.s(), cx.s().obj(id)).any(|c| cards.replaces[c.0 as usize]))
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
        && (cx.s().player(player).drew_from_empty || crate::static_rules::cant_draw(cx, cx.cards(), player, 1))
        && !graveyard_replaces_draws(cx, player)
        && (!possible(cx, None) || candidates(cx, &Proposal::Draw(CardDraw { player, replaced: false }), &[], None, [None; 8]).is_empty())
}

fn subject_of(proposal: &Proposal) -> Option<ObjectId> {
    match proposal {
        Proposal::ZoneChange(z) => Some(z.object),
        Proposal::Counters(c) => Some(c.object),
        // The permanent tapped for mana. Its own `own_everywhere` effects are looked at where a cost has moved it (a
        // sacrificed Treasure's): no printed card has one of this kind, which would function from its new zone here.
        Proposal::ManaProduction(m) => Some(m.permanent),
        Proposal::Connive(c) => Some(c.creature),
        Proposal::DrawInstruction(_)
        | Proposal::Draw(_)
        | Proposal::Mill(_)
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
        Proposal::Mill(m) => m.player,
        // Whoever controls the conniving creature (last controlled, CR 701.50b), who is the one that draws and discards.
        Proposal::Connive(c) => cx.s().obj(c.creature).controller,
        Proposal::TokenCreation(t) => t.controller,
        Proposal::LifeChange(l) => l.player,
        // "If multiple replacement effects would modify the mana an ability you control produces, choose one to apply"
        // (Damping Sphere's ruling): the player who tapped the permanent for it.
        Proposal::ManaProduction(m) => m.player,
    }
}

/// Every effect that applies to the event and has not been applied to it yet (CR 614.5), in a deterministic order:
/// by source, then by the order the card lists them.
fn candidates(cx: &Cx, proposal: &Proposal, applied: &[Applied], entry: Option<&Entering>, commander_declined_to: [Option<Zone>; 8]) -> Vec<Candidate> {
    candidates_for(cx, proposal, applied, entry, commander_declined_to, &|_| Some(*proposal))
}

/// Formation may retain independent replacement histories within one source/recipient event (CR 614.5, 700.1).
/// Each effect is tested against exactly the quantity whose lineage still permits that effect. Other proposal kinds
/// use the original proposal unchanged; availability, source text, redirection legality and candidate order stay here.
fn candidates_for(cx: &Cx, proposal: &Proposal, applied: &[Applied], entry: Option<&Entering>, commander_declined_to: [Option<Zone>; 8], select: &dyn Fn(Applied) -> Option<Proposal>) -> Vec<Candidate> {
    // Each effect's own text, its source's now (`Cx::own_text`).
    let _others = cx.others_text();
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
        // A permanent that has lost all its abilities has no replacement effects (CR 613.1f).
        if !source_text_ids(cx, id).any(|c| cards.replaces[c.0 as usize]) || cx.lost_all_abilities(id) {
            return;
        }
        // "You" for an ability of a card in a graveyard is its owner (CR 108.4a: it has no controller).
        let controller = if at == At::Graveyard { cx.s().obj(id).owner } else { controller_of(cx, id, proposal) };
        for (index, (def, by)) in printed_replacements(cx, id).enumerate() {
            let index = index as u8;
            let asker = Asker { instance: None, source: id, controller, by, spell: false };
            let functions = match at {
                At::Field => !def.in_graveyard,
                At::Own => def.own_everywhere,
                At::Graveyard => def.in_graveyard,
            };
            if def.kind != kind || !functions || applied.contains(&(id, index, Origin::Printed)) {
                continue;
            }
            let Some(selected) = select((id, index, Origin::Printed)) else { continue };
            if def.applies_to(cx, &asker, &selected)
                && valid_redirection(cx, &selected, |after| def.reshape(cx, &asker, after))
            {
                found.push(Candidate { source: id, index, origin: Origin::Printed });
            }
        }
        // Its static abilities' prevention effects function only while it is on the battlefield (CR 604.1).
        if let Proposal::Damage(_) = proposal
            && preventable
            && at == At::Field
        {
            for (index, (def, by)) in printed_preventions(cx, id).enumerate() {
                let index = index as u8;
                let asker = Asker { instance: None, source: id, controller, by, spell: false };
                if !applied.contains(&(id, index, Origin::Prevention))
                    && let Some(Proposal::Damage(selected)) = select((id, index, Origin::Prevention))
                    && (def.applies)(cx, &asker, &selected) {
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
                    let asker = Asker { instance: None, source: e.source, controller: e.controller, by: None, spell: false };
                    let reads = WithWords { q: cx, source: e.source, words: e.words };
                    if let Some(Proposal::Damage(selected)) = select((e.source, index, Origin::GrantedPrevention(e.timestamp)))
                        && (def.applies)(&reads, &asker, e.affected, &selected) {
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
        let asker = Asker { instance: None, source: e.source, controller: e.controller, by: None, spell: false };
        let reads = WithWords { q: cx, source: e.source, words: e.words };
        let Some(selected) = select((e.source, index, Origin::Granted(e.timestamp))) else { continue };
        if (def.applies)(&reads, &asker, e.affected, &selected)
            && valid_redirection(cx, &selected, |after| (def.apply)(&reads, &asker, e.affected, after))
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
        let after_own = usize::from(own.is_some());
        found.splice(0..0, own);
        // An instance of riot it would have and that has not applied yet (CR 702.136b), judged as it would exist with those
        // applied so far (CR 616.1f): one its own "as this enters" gives it applies after that one. Offered after its own.
        if let Some(w) = cx.entering.filter(|w| w.object == e.object) {
            let applied_riot = applied.iter().filter(|&&(source, _, origin)| source == e.object && origin == Origin::Riot).count();
            if usize::from(w.chars.riot) > applied_riot {
                found.insert(after_own, Candidate { source: e.object, index: 0, origin: Origin::Riot });
            }
        }
    }
    // The counters the effect putting it onto the battlefield gives it if it's one of what that effect names (CR 614.1c,
    // `Entering::counters_if`), ordered with the rest (CR 616.1): offered while, as it would exist with those applied so far
    // (CR 614.12, 616.1f), it would get some - a copy effect it enters with already over it (CR 616.1c). Not an ability of
    // its own, so one it would have lost its abilities there still has it.
    if let (Some(e), Proposal::ZoneChange(z)) = (entry, proposal)
        && z.object == e.object
        && z.to.is(ZoneKind::Battlefield)
        && let Some(counters) = e.counters_if
        && let Some(w) = cx.entering.filter(|w| w.object == e.object)
        && counters.counters(&w.chars).iter().any(|&n| n > 0)
    {
        found.push(Candidate { source: e.object, index: 0, origin: Origin::CountersIf });
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
        if cx.s().commander && (z.to.is(ZoneKind::Hand) || z.to.is(ZoneKind::Library)) {
            for (index, (identity, _)) in commander_components(cx, z.object).into_iter().enumerate().take(8) {
                if commander_declined_to[index] != Some(z.to)
                    && !z.commander_components.contains(&identity)
                {
                    found.insert(0, Candidate { source: z.object, index: index as u8, origin: Origin::Commander });
                }
            }
        }
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
        && select((id, 0, Origin::ShieldCounter)).is_some()
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
        && select((by, 0, Origin::Protection)).is_some()
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

/// The physical commander cards among a permanent's components, in component order. Replacement choices index this
/// filtered list, so any number of ordinary components before them cannot hide a commander from the replacement queue.
fn commander_components(cx: &Cx, object: ObjectId) -> Vec<(ObjectId, bool)> {
    match crate::merge::components(cx, object) {
        Some(parts) => parts.into_iter().filter_map(|part| part.commander_id.map(|id| (id, part.top))).collect(),
        None => cx.s().obj(object).commander_id.map(|id| vec![(id, true)]).unwrap_or_default(),
    }
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
        Origin::Printed => printed_replacement(cx, c.source, c.index).hint,
        Origin::Granted(timestamp) => granted(cx, timestamp).map_or("", |(_, def)| def.hint),
        Origin::Madness => MADNESS_HINT,
        Origin::Commander => "put this commander into the command zone instead",
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
        Origin::Prevention => printed_prevention(cx, c.source, c.index).0.hint,
        Origin::GrantedPrevention(timestamp) => match cx.s().effects.iter().find(|e| e.timestamp == timestamp).map(|e| e.kind) {
            Some(EffectKind::Prevention { card, index, .. }) => cx.cards().get(card).granted_preventions[index as usize].hint,
            _ => "",
        },
        Origin::PlayedTapped => PLAYED_TAPPED_HINT,
        Origin::Enters(card) => enters_hint(cx.cards().get(card)),
        Origin::Riot => RIOT_HINT,
        Origin::CountersIf => COUNTERS_IF_HINT,
    }
}

/// Whether the effect prevents all the damage it applies to and uses nothing up: a static ability's prevention of all of
/// it, protection's, or a resolved effect that prevents all of it. Which of several such effects applies first changes nothing
/// but which one is recorded as having prevented it, so the affected player is not asked to order only these (CR 616.1).
fn prevents_all(cx: &Cx, c: &Candidate) -> bool {
    match c.origin {
        Origin::Protection => true,
        Origin::Prevention => matches!(printed_prevention(cx, c.source, c.index).0.amount, PreventionAmount::All),
        // One with more to do (CR 615.5) is told what it prevented, so which one applies first changes what happens.
        Origin::GrantedPrevention(timestamp) => cx.s().effects.iter().any(|e| {
            e.timestamp == timestamp
                && matches!(e.kind, EffectKind::Prevention { card, index, shield: Shield::All, .. }
                    if cx.cards().get(card).granted_preventions[index as usize].then.is_none())
        }),
        _ => false,
    }
}

/// Asks the affected player which of several effects applies first (CR 616.1). When the question has no answer yet
/// the run is abandoned and replayed with it (see `Cx::ask_core`), so the first is taken to carry on to the end.
fn choose(cx: &mut Cx, proposal: &Proposal, found: &[Candidate]) -> Option<usize> {
    // A concession at this question may remove its candidates (CR 104.3a, 611.3b); rebuild before asking.
    if cx.concessions_here() {
        return None;
    }
    let ask = RequestKind::ChooseReplacement {
        subject: subject_of(proposal),
        life: match proposal {
            Proposal::LifeChange(l) => Some((l.magnitude, l.negative)),
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
        Ok(Answer::Index(i)) if (i as usize) < found.len() => Some(i as usize),
        _ => Some(0),
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
fn run_then(cx: &mut Cx, applied: &[Applied], source: ObjectId, controller: PlayerId, then: impl FnOnce(&mut Cx)) {
    let outer = std::mem::replace(&mut cx.lineage, applied.to_vec());
    callback_instructions(cx, source, |cx| cx.acting_as(controller, then));
    cx.lineage = outer;
}

/// A replacement's additional instructions are its own card code (CR 614.6),
/// not extra instructions of the spell whose event is being changed. Check that
/// code at its own resolution depth: grouping a core event must not silently
/// permit arbitrary unannounced changes inside its callbacks. Carry pending
/// changes into it so an event or explicit sentence boundary judges the whole
/// game, then restore the enclosing core instruction's bookkeeping.
fn callback_instructions(cx: &mut Cx, source: ObjectId, callback: impl FnOnce(&mut Cx)) {
    let outer = cx.g.check;
    cx.g.check = crate::StateCheck { depth: 1, resolving: Some(source), due: outer.due, open: false, done: outer.done, judged: false };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(cx)));
    let checked = std::mem::replace(&mut cx.g.check, outer);
    if let Err(panic) = result { std::panic::resume_unwind(panic); }
    // A callback may judge the caller's pending change, then make another. Its
    // final `due` alone cannot tell whether that earlier change was judged.
    if checked.judged { cx.state_judged(); }
    if checked.due {
        cx.g.check.due = true;
        if checked.open || checked.done { cx.g.check.open = true; }
    }
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

/// Redirecting a move onto the battlefield is that replacement's instruction (CR 609.1, 614.1a), unless it names
/// another player explicitly. Merely changing an entering permanent's controller does not change its putter.
fn note_put_by(before: &Proposal, after: &mut Proposal, controller: PlayerId) {
    if let (Proposal::ZoneChange(before), Proposal::ZoneChange(after)) = (before, after) {
        if !after.to.is(ZoneKind::Battlefield) {
            after.put_by = None;
        } else if !before.to.is(ZoneKind::Battlefield) && after.put_by.is_none() {
            after.put_by = Some(controller);
        }
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

/// Partial redirection obeys the same recipient authority as whole redirection
/// (CR 614.9). An impossible redirection neither reduces the original damage nor
/// consumes its once-only effect.
fn valid_damage_components(q: &dyn Query, before: &Proposal, components: &[DamageComponent]) -> bool {
    let Proposal::Damage(original) = before else { return components.is_empty() };
    components.iter().all(|c| match *c {
        DamageComponent::Redirected { target, amount } if amount > 0 => damage_recipient(q, original.target) && damage_recipient(q, target),
        _ => true,
    })
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
    apply_to(cx, proposal, None, None);
}

/// `apply`, to a permanent's entry with its own replacements still to apply (`Entering::own`) where `entry` is given; to
/// damage, noting in `uses` the prevention effects that have more to do once its event has been dealt (`PreventionUse`).
/// Each effect is its own text, its source's now, though a resolving ability's instructions made the event (`Cx::own_text`).
fn apply_to(cx: &mut Cx, proposal: &mut Proposal, entry: Option<&mut Entering>, uses: Option<&mut Vec<PreventionUse>>) {
    // Damage is formed with its whole event (`damage_event_inner`); what an effect adds to damage reaches here never.
    let _ = cx.instruction(|cx| cx.own_text(false, |cx| apply_each(cx, proposal, entry, uses, false, None)), |_| true);
}

enum DamageProgress {
    Expanded { added: Vec<DamageComponent>, applied: Vec<Applied> },
}

fn apply_each(cx: &mut Cx, proposal: &mut Proposal, mut entry: Option<&mut Entering>, mut uses: Option<&mut Vec<PreventionUse>>, group_once: bool, mut selected: Option<Candidate>) -> Option<DamageProgress> {
    let mut applied: Vec<Applied> = cx.lineage.clone();
    let mut commander_declined_to: [Option<Zone>; 8] = [None; 8];
    loop {
        if let Proposal::ZoneChange(z) = proposal && !z.to.is(ZoneKind::Battlefield) {
            z.put_by = None;
        }
        // A card's replacement can ask to regenerate directly (701.19b), as
        // Clergy of the Holy Nimbus does. "Can't" also defeats that request.
        // Clear it before looking for further replacements, so a shield counter
        // can still replace destruction and the event remains a destruction.
        if let Proposal::ZoneChange(z) = proposal {
            if z.regenerated && (!z.destroy || crate::static_rules::cant_be_regenerated(cx, cx.cards(), z.object)) {
                z.regenerated = false;
                z.regeneration_shield = false;
            }
        }
        // An entering permanent is judged as it would exist with the changes of the replacements applied so far (CR 614.12).
        if let Proposal::ZoneChange(z) = *proposal {
            crate::ops::rejudge_entering(cx, &z);
        }
        if let Proposal::Counters(placement) = proposal {
            // A non-entering recipient's controller is live information, not the controller from when its first
            // replacement choice was built (CR 616.1, 611.3a). A concession or a continuation can change it here.
            // An entry instead keeps its prospective controller, even while its object is still in another zone
            // (CR 614.12). Neither case changes the damage/effect's putter.
            if !placement.entering {
                if !live(cx.s(), placement.object) || !cx.s().obj(placement.object).zone.is(ZoneKind::Battlefield) {
                    placement.counts = [0; CounterKind::COUNT];
                    return None;
                }
                placement.controller = cx.s().obj(placement.object).controller;
            }
            placement.counts = without_banned(cx, placement.object, placement.controller, placement.counts);
            if placement.counts.iter().all(|&n| n <= 0) {
                return None;
            }
        }
        let found = match selected.take() {
            // The effect chosen for the part it is still eligible for applies only if it still applies to it as that part
            // is now; otherwise it does nothing to it.
            Some(candidate) if candidates(cx, proposal, &applied, entry.as_deref(), commander_declined_to).contains(&candidate) => vec![candidate],
            Some(_) => return None,
            None => candidates(cx, proposal, &applied, entry.as_deref(), commander_declined_to),
        };
        if found.is_empty() {
            return None;
        }
        let pick = if found.len() > 1 && !found.iter().all(|c| prevents_all(cx, c)) {
            let Some(pick) = choose(cx, proposal, &found) else { continue };
            pick
        } else { 0 };
        let chosen = found[pick];
        if chosen.origin == Origin::Printed
            && printed_replacement(cx, chosen.source, chosen.index).optional.is_some()
            && cx.concessions_here()
        {
            // Re-enter the loop to refresh both the proposal and its candidates, even if the old candidate would
            // still match the stale proposal (for example, counters on a creature that just changed controller).
            continue;
        }
        let Candidate { source, index, origin } = chosen;
        applied.push((source, index, origin));
        let start = *proposal;
        let mut added = Vec::new();
        match origin {
            Origin::Printed => {
                // Declined, it has had its opportunity, and the event goes on as it was (CR 614.5, 616.1).
                if !printed_taken(cx, source, index, proposal) {
                    if group_once { return Some(DamageProgress::Expanded { added, applied }); }
                    continue;
                }
                apply_printed(cx, proposal, source, index, &applied, &mut added);
            }
            Origin::Granted(timestamp) => {
                let Some((&e, def)) = granted(cx, timestamp) else { continue };
                apply_granted(cx, proposal, source, e, def, &applied, true, &mut added);
            }
            // "That player discards it, but exiles it instead of putting it into their graveyard" (CR 702.35a): the
            // graveyard is its owner's (CR 400.3), and so is the exile it goes to.
            Origin::Madness => {
                if let Proposal::ZoneChange(z) = proposal {
                    z.to = Zone::exile(z.to.player);
                    z.madness = true;
                }
            }
            Origin::Commander => {
                if matches!(cx.ask_core(cx.s().obj(source).owner, RequestKind::May { source, purpose: Purpose::Optional }, false), Ok(Answer::No)) {
                    commander_declined_to[index as usize] = match *proposal { Proposal::ZoneChange(z) => Some(z.to), _ => None };
                } else if let Proposal::ZoneChange(z) = proposal {
                    if let Some((identity, top)) = commander_components(cx, source).get(index as usize).copied() {
                        if top {
                            z.to = Zone::COMMAND;
                        } else if let Some(slot) = z.commander_components.iter_mut().find(|slot| **slot == ObjectId::NONE) {
                            *slot = identity;
                        }
                    }
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
            // An instance of riot (CR 702.136a), its choice made by the player it now enters under (CR 614.12a): the counter
            // or the haste is the entry's. It is judged again as it would exist with it applied (CR 614.12).
            Origin::Riot => {
                if let (Some(e), Proposal::ZoneChange(z)) = (entry.as_deref_mut(), &mut *proposal) {
                    e.controller = z.controller;
                    e.tapped = z.tapped;
                    crate::ops::apply_riot(cx, e);
                    cx.entering = Some(crate::ops::as_it_would_exist(cx, e));
                }
            }
            // The counters the effect putting it there gives it for what it now would be (CR 614.1c, 614.12), each kind it
            // names, more it enters with (CR 122.6). Applied once: it is emptied. It is judged again as it would exist with
            // them (CR 614.12).
            Origin::CountersIf => {
                if let (Some(e), Proposal::ZoneChange(z)) = (entry.as_deref_mut(), &mut *proposal)
                    && let Some(counters) = e.counters_if.take()
                    && let Some(w) = cx.entering.filter(|w| w.object == e.object)
                {
                    e.controller = z.controller;
                    e.tapped = z.tapped;
                    for (n, with) in e.counters.iter_mut().zip(counters.counters(&w.chars)) {
                        *n += with;
                    }
                    cx.entering = Some(crate::ops::as_it_would_exist(cx, e));
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
                    z.regeneration_shield = true;
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
                let (def, by) = printed_prevention(cx, source, index);
                let asker = Asker { instance: None, source, controller: controller_of(cx, source, proposal), by, spell: false };
                if let Proposal::Damage(d) = proposal {
                    let scope = DamageScope::of(d);
                    let key = (source, index, origin);
                    let amount = match def.amount {
                        PreventionAmount::All => d.amount,
                        PreventionAmount::Relative(how_much) => how_much(&*cx, &asker, d),
                        PreventionAmount::Specific(how_much) => {
                            let previous = cx.damage.as_ref().and_then(|c| c.allowance(key, scope));
                            let (capacity, spent) =
                                previous.unwrap_or_else(|| (how_much(&*cx, &asker, d).max(0), 0));
                            if let Some(context) = cx.damage.as_mut() {
                                context.freeze(key, scope, capacity);
                            }
                            capacity - spent
                        }
                    };
                    prevent(cx, d, amount, source);
                }
            }
            Origin::GrantedPrevention(timestamp) => {
                if let Proposal::Damage(d) = proposal {
                    let at = cx.s().effects.iter().position(|e| e.timestamp == timestamp).expect("the effect that applies");
                    let mut e = cx.s().effects[at];
                    let as_applied = e;
                    let EffectKind::Prevention { card, index, to, shield } = e.kind else { unreachable!("a prevention effect") };
                    let prevented = match shield {
                        Shield::All | Shield::NextTime => d.amount,
                        Shield::Next(left) => left.min(d.amount),
                    };
                    let part = Prevented { source: d.source, target: d.target, amount: prevented, combat: d.combat };
                    prevent(cx, d, prevented, source);
                    match shield {
                        // CR 615.8: "the next time" prevents all of the one event, and is used up once the event has been
                        // dealt (`rest_of_preventions`): the event's other parts it applies to are prevented too.
                        Shield::All | Shield::NextTime => {}
                        // CR 615.7: each 1 damage prevented reduces the shield by 1; once it is 0 the rest is dealt, and
                        // the effect is used up.
                        Shield::Next(left) => {
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
                    // CR 615.5: the rest of the effect, once the event has been dealt; and the use of a "next time" (CR 615.8).
                    let then = cx.cards().get(card).granted_preventions[index as usize].then;
                    if prevented > 0
                        && (then.is_some() || shield == Shield::NextTime)
                        && let Some(uses) = uses.as_deref_mut()
                    {
                        match uses.iter_mut().find(|u| u.effect.timestamp == timestamp) {
                            Some(u) => u.parts.push(part),
                            None => uses.push(PreventionUse { effect: as_applied, parts: vec![part] }),
                        }
                    }
                }
            }
        }
        if is_prevention(origin)
            && let (Proposal::Damage(before), Proposal::Damage(after)) = (start, *proposal)
        {
            let spent = if origin == Origin::Prevention && matches!(printed_prevention(cx, source, index).0.amount, PreventionAmount::Specific(_)) { before.amount - after.amount.min(before.amount) } else { 0 };
            if spent > 0 && let Some(context) = cx.damage.as_mut() { context.applied((source, index, origin), DamageScope::of(&before), spent); }
        }
        unlink_if_sent_elsewhere(&start, proposal);
        if let Proposal::ZoneChange(z) = &mut *proposal {
            for declined in &mut commander_declined_to {
                if *declined != Some(z.to) {
                    *declined = None;
                }
            }
        }
        if group_once || !added.is_empty() { return Some(DamageProgress::Expanded { added, applied }); }
    }
}

/// The player whose "you" a source's printed effect has: its controller - for the permanent entering, the player it enters
/// under - or, for one functioning from a graveyard, its owner (CR 108.4a).
fn printed_controller(cx: &Cx, source: ObjectId, index: u8, proposal: &Proposal) -> PlayerId {
    let def = printed_replacement(cx, source, index);
    if def.in_graveyard { cx.s().obj(source).owner } else { controller_of(cx, source, proposal) }
}

/// Whether a source's printed effect is taken as it applies: one that is not optional always is; an optional one ("you
/// may ... instead", CR 614.1a) is its controller's to take (`takes`).
fn printed_taken(cx: &mut Cx, source: ObjectId, index: u8, proposal: &Proposal) -> bool {
    let Some(question) = printed_replacement(cx, source, index).optional else { return true };
    let controller = printed_controller(cx, source, index, proposal);
    takes(cx, controller, source, question)
}

/// Damage quantities are nonnegative even though their expression domain is
/// signed (CR 107.1b). Normalize before expansion/continuation reads the result.
fn normalize_damage(proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal { d.amount = d.amount.max(0); }
}

/// A source's printed effect, taken, applied to the event: what it changes of it, unless that is a redirection that can't be
/// (CR 614.9), and what it does to the game as it applies (`run_then`).
fn apply_printed(cx: &mut Cx, proposal: &mut Proposal, source: ObjectId, index: u8, applied: &[Applied], added: &mut Vec<DamageComponent>) {
    let def = printed_replacement(cx, source, index);
    let then = def.then;
    let controller = printed_controller(cx, source, index, proposal);
    let asker = Asker { instance: None, source, controller, by: printed_link(cx, source, index), spell: false };
    let before = *proposal;
    let mut after = before;
    def.reshape(&*cx, &asker, &mut after);
    normalize_damage(&mut after);
    if !valid_redirection(&*cx, &before, |p| *p = after) { return; }
    let components = if let (Some(damage), Proposal::Damage(before), Proposal::Damage(after)) = (def.damage, before, after) {
        damage(&*cx, &asker, &before, &after)
    } else { Vec::new() };
    if !valid_damage_components(&*cx, &before, &components) { return; }
    *proposal = after;
    note_counters_by(&before, proposal, controller);
    note_put_by(&before, proposal, controller);
    added.extend(components);
    if let Some(then) = then {
        let after = *proposal;
        run_then(cx, applied, source, controller, |cx| then(cx, &asker, &before, &after));
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
    added: &mut Vec<DamageComponent>,
) -> bool {
    let (apply, then) = (def.apply, def.then);
    let asker = Asker { instance: None, source, controller: e.controller, by: None, spell: false };
    let before = *proposal;
    let reads = WithWords { q: &*cx, source: e.source, words: e.words };
    let mut after = before;
    apply(&reads, &asker, e.affected, &mut after);
    normalize_damage(&mut after);
    if !valid_redirection(&*cx, &before, |p| *p = after) { return false; }
    let components = if let (Some(damage), Proposal::Damage(before), Proposal::Damage(after)) = (def.damage, before, after) {
        damage(&reads, &asker, e.affected, &before, &after)
    } else { Vec::new() };
    if !valid_damage_components(&*cx, &before, &components) { return false; }
    *proposal = after;
    if use_up && def.once { used_up(cx, e.timestamp); }
    note_counters_by(&before, proposal, e.controller);
    note_put_by(&before, proposal, e.controller);
    added.extend(components);
    if let Some(then) = then {
        let after = *proposal;
        cx.effect_text(&e, |cx| run_then(cx, applied, e.source, e.controller, |cx| then(cx, &asker, e.affected, &before, &after)));
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
fn apply_to_parts(cx: &mut Cx, parts: &mut Vec<Proposal>) {
    cx.instruction(|cx| cx.own_text(false, |cx| apply_to_each_part(cx, parts)), |_| true);
}

/// The token an effect that just applied added to the event ("those tokens plus a [token]", `TokenCreation::plus`), as a
/// group of its own: one, however many of the event's groups set it, each mark cleared.
fn take_plus(parts: &mut Vec<Proposal>) {
    let mut added: Option<TokenCreation> = None;
    for part in parts.iter_mut() {
        if let Proposal::TokenCreation(t) = part
            && let Some((token, copy)) = t.plus.take()
        {
            added.get_or_insert(TokenCreation { controller: t.controller, token, count: 1, copy, plus: None });
        }
    }
    parts.extend(added.map(Proposal::TokenCreation));
}

fn apply_to_each_part(cx: &mut Cx, parts: &mut Vec<Proposal>) {
    let mut applied: Vec<Applied> = cx.lineage.clone();
    loop {
        let mut found: Vec<Vec<Candidate>> = parts.iter().map(|p| candidates(cx, p, &applied, None, [None; 8])).collect();
        let mut all: Vec<Candidate> = found.iter().flatten().copied().collect();
        all.sort_by_key(|c| (c.source, c.index, c.origin));
        all.dedup();
        let Some(&first) = all.first() else { return };
        let chosen = if all.len() > 1 && !all.iter().all(|c| prevents_all(cx, c)) {
            let Some(pick) = choose(cx, &parts[0], &all) else { continue };
            all[pick]
        } else { first };
        if chosen.origin == Origin::Printed
            && printed_replacement(cx, chosen.source, chosen.index).optional.is_some()
            && cx.concessions_here()
        {
            found = parts.iter().map(|p| candidates(cx, p, &applied, None, [None; 8])).collect();
            if !found.iter().flatten().any(|c| *c == chosen) {
                continue;
            }
        }
        let Candidate { source, index, origin } = chosen;
        applied.push((source, index, origin));
        let meets = |j: usize| found[j].iter().any(|c| (c.source, c.index, c.origin) == (source, index, origin));
        match origin {
            Origin::Printed => {
                let Some(at) = (0..parts.len()).find(|&j| meets(j)) else { continue };
                if !printed_taken(cx, source, index, &parts[at]) {
                    continue;
                }
                for j in (0..parts.len()).filter(|&j| meets(j)) {
                    let mut added = Vec::new();
                    // Only a damage replacement adds damage (`ReplacementDef::with_damage`); these are no damage.
                    apply_printed(cx, &mut parts[j], source, index, &applied, &mut added);
                }
                take_plus(parts);
            }
            Origin::Granted(timestamp) => {
                let Some((&e, def)) = granted(cx, timestamp) else { continue };
                let mut took = false;
                for j in (0..parts.len()).filter(|&j| meets(j)) {
                    let mut added = Vec::new();
                    took |= apply_granted(cx, &mut parts[j], source, e, def, &applied, false, &mut added);
                }
                if took && def.once {
                    used_up(cx, timestamp);
                }
                take_plus(parts);
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
    let _others = cx.others_text();
    match c.origin {
        Origin::Printed => {
            let def = printed_replacement(cx, c.source, c.index);
            let asker = Asker { instance: None, source: c.source, controller: controller_of(cx, c.source, proposal), by: printed_link(cx, c.source, c.index), spell: false };
            def.reshape(cx, &asker, proposal);
        }
        Origin::Granted(timestamp) => {
            let Some((e, def)) = granted(cx, timestamp) else { return };
            let reads = WithWords { q: cx, source: e.source, words: e.words };
            (def.apply)(&reads, &Asker { instance: None, source: c.source, controller: e.controller, by: None, spell: false }, e.affected, proposal);
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
    s.battlefield().iter().any(|&id| cards.texts(s, s.obj(id)).any(|c| cards.mana_replaces[c.0 as usize]))
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
        let found = candidates(cx, &proposal, applied, None, [None; 8]);
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
                Origin::Printed => printed_replacement(cx, c.source, c.index).optional.is_some(),
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

/// The types an announced-X color combination could produce through the replacement effects on its tap (CR 106.7):
/// each listed color, at the totals that can make a difference. A fixed-output replacement (`FixedManaReplacement`)
/// reads only whether the total reaches its threshold and the source, so [1, first threshold), [first, second), ...
/// each take one path - the color survives, or the first replacement leaves a fixed pool - and each interval's least
/// total stands for all of it, the last, unbounded one included. Any other mana replacement is tried at one mana and
/// at two: each printed one reads no amount (Ritual of Subdual's and Deep Water's "instead of any other type") or
/// multiplies it (Mana Reflection), and none reads more than whether there are two or more (Damping Sphere's). Mixed
/// colors add no distinct output type: every such replacement maps each unit, or the whole total, alike.
pub(crate) fn combination_types(cx: &Cx, permanent: ObjectId, player: PlayerId, colors: ColorSet,
    total: Option<u32>, restriction: Option<ManaRestriction>) -> u8 {
    if total == Some(0) { return 0; }
    let mut bounds = match total { Some(total) => vec![total], None => vec![1, 2] };
    if total.is_none() {
        for &id in cx.s().battlefield() {
            if !live(cx.s(), id) || cx.lost_all_abilities(id) { continue; }
            for (index, (def, by)) in printed_replacements(cx, id).enumerate() {
                if def.kind != ReplacementKind::ManaProduction || def.in_graveyard
                    || cx.lineage.contains(&(id, index as u8, Origin::Printed)) { continue; }
                let Some(rule) = def.fixed_mana else { continue };
                let asker = Asker { instance: None, source: id, controller: cx.controller(id), by, spell: false };
                if (rule.source)(cx, &asker, permanent, player) && rule.minimum > 1 {
                    bounds.push(rule.minimum);
                }
            }
        }
    }
    bounds.sort_unstable();
    bounds.dedup();
    let mut types = 0;
    for total in bounds {
        for color in colors.iter() {
            let mut mana = ManaPool::default();
            mana.0[color.mana() as usize] = total;
            for outcome in mana_outcomes(cx, ManaProduction { permanent, player, to: player, mana, restriction, lasts: ManaLasts::Step }) {
                for m in Mana::ALL {
                    if outcome.mana.get(m) > 0 { types |= 1 << m as u8; }
                }
            }
        }
    }
    types
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
            && cards.texts(cx.s(), cx.s().obj(id)).any(|c| cards.get(c).replacements.iter().any(|r| r.kind == ReplacementKind::Damage))
    })
}

/// The prevention shields that would apply to this damage (CR 615.7), taken together: the source of the first, and how much
/// they have left to prevent between them, in the published bounded u32 domain. None where no
/// shield applies, or the damage can't be prevented (CR 615.12).
pub(crate) fn shields(cx: &Cx, d: &DamageEvent) -> Option<(ObjectId, u32)> {
    if !cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::Prevention { shield: Shield::Next(_), .. })) {
        return None;
    }
    if !crate::ops::damage_can_be_prevented(cx, d.source, d.combat) {
        return None;
    }
    let mut found: Option<(ObjectId, u32)> = None;
    let _others = cx.others_text();
    for e in &cx.s().effects {
        let EffectKind::Prevention { card, index, to, shield: Shield::Next(left) } = e.kind else { continue };
        if !(to == Target::None || to == d.target) || (to.object().is_some() && !live(cx.s(), e.affected)) {
            continue;
        }
        let def = &cx.cards().get(card).granted_preventions[index as usize];
        let reads = WithWords { q: cx, source: e.source, words: e.words };
        if (def.applies)(&reads, &Asker { instance: None, source: e.source, controller: e.controller, by: None, spell: false }, e.affected, d) {
            found = Some(found.map_or((e.source, u32::from(left)), |(s, n)| (s, n + u32::from(left))));
        }
    }
    found
}

/// Whether a prevention effect may prevent this damage (CR 615.12).
fn damage_can_be_prevented(cx: &Cx, d: &DamageEvent) -> bool {
    crate::ops::damage_can_be_prevented(cx, d.source, d.combat)
}

/// A prevention effect a spell or ability made that has more to do once the damage event it prevented some of has been
/// dealt (CR 615.5, 615.8): the effect as it was when it first applied to the event, and what it prevented of each part.
/// Kept by the event (`ops::DamageBatch`) until then.
pub(crate) struct PreventionUse {
    effect: ContinuousEffect,
    parts: Vec<Prevented>,
}

/// The rest of the prevention effects that prevented some of a damage event, once it has been dealt and announced, in the
/// order they first applied to it (CR 615.5: "immediately afterward", before state-based actions and before the spell or
/// ability that dealt it goes on resolving): a "next time" effect is used up by the event (CR 615.8), and an effect with more
/// to do (`GrantedPreventionDef::then`) does it, its controller doing it (CR 609.1), told what it prevented.
pub(crate) fn rest_of_preventions(cx: &mut Cx, uses: Vec<PreventionUse>) {
    // Each effect's rest is its own text, its source's now, though a resolving ability's instructions dealt the damage
    // (`Cx::own_text`).
    cx.instruction(|cx| cx.own_text(false, |cx| rest_of_each(cx, uses)), |_| true);
}

fn rest_of_each(cx: &mut Cx, uses: Vec<PreventionUse>) {
    for PreventionUse { effect, parts } in uses {
        let EffectKind::Prevention { card, index, shield, .. } = effect.kind else { continue };
        if shield == Shield::NextTime {
            used_up(cx, effect.timestamp);
        }
        if let Some(then) = cx.cards().get(card).granted_preventions[index as usize].then {
            let asker = Asker { instance: None, source: effect.source, controller: effect.controller, by: None, spell: false };
            cx.effect_text(&effect, |cx| callback_instructions(cx, effect.source, |cx| cx.acting_as(effect.controller, |cx| then(cx, &asker, effect.affected, &parts))));
        }
    }
}

/// Prevents `amount` of the damage (CR 615.1) by the effect of `by`, and records it (CR 615.6, 615.13).
fn prevent(cx: &mut Cx, d: &mut DamageEvent, amount: u32, by: ObjectId) {
    let amount = amount.min(d.amount);
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
    counts: [i32; CounterKind::COUNT],
) -> [i32; CounterKind::COUNT] {
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
        Proposal::Counters(placement) => without_banned(cx, placement.object, placement.controller, placement.counts),
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// `counts` less every kind `object`, controlled by `controller`, can't have put on it (CR 101.2, 122.6).
pub(crate) fn without_banned(cx: &Cx, object: ObjectId, controller: PlayerId, mut counts: [i32; CounterKind::COUNT]) -> [i32; CounterKind::COUNT] {
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

/// An instruction to mill cards (CR 701.17a): how many are milled once every effect that changes the number has applied
/// (CR 614.1a), before any card moves.
pub fn mill_count(cx: &mut Cx, player: PlayerId, count: u64) -> u64 {
    if !possible(cx, None) {
        return count;
    }
    let mut proposal = Proposal::Mill(MillInstruction { player, count });
    apply(cx, &mut proposal);
    match proposal {
        Proposal::Mill(m) => m.count,
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// A permanent about to connive (CR 701.50): the effects that replace it apply, each once (CR 614.5), doing what they do
/// in its place ("instead you draw a card", their `then`); the connive then goes on as it is. Two of them draw two cards.
pub fn would_connive(cx: &mut Cx, creature: ObjectId, count: u32) {
    if possible(cx, None) {
        apply(cx, &mut Proposal::Connive(Connive { creature, count }));
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
///
/// Beside each token made is the token asked for that it is one of "those tokens" for (CR 614.1a: an effect that changes how
/// many or what kind of tokens are made still makes those tokens), as its place in `tokens`: a run's tokens are dealt out
/// over the places of its run in turn, so under Doubling Season each place asked for has two. A token an effect added to
/// the event ("those tokens plus a [token]") stands for none.
pub fn token_batch(cx: &mut Cx, tokens: &[(CardId, Option<Copiable>, PlayerId)]) -> Option<(Vec<(CardId, Option<Copiable>, PlayerId)>, Vec<Option<usize>>)> {
    if !possible(cx, None) {
        return None;
    }
    // The runs of one definition, copy and controller, in the order given, and where each starts.
    let mut groups: Vec<TokenCreation> = Vec::new();
    let mut starts: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let (token, copy, controller) = tokens[i];
        starts.push(i);
        let mut count = 0u32;
        while i < tokens.len() && tokens[i] == (token, copy, controller) {
            count += 1;
            i += 1;
        }
        groups.push(TokenCreation { controller, token, copy, count, plus: None });
    }
    let mut after = groups.clone();
    // The groups an effect added to the event ("those tokens plus a [token]").
    let mut added: Vec<TokenCreation> = Vec::new();
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
        let mut parts = parts.into_iter().map(|part| match part {
            Proposal::TokenCreation(t) => t,
            _ => unreachable!("an effect cannot change the kind of the event"),
        });
        for (&j, t) in theirs.iter().zip(parts.by_ref()) {
            after[j] = t;
        }
        added.extend(parts);
    }
    // Keep the final replacement’s token definition and copy values (CR 614.1a).
    let changed = !added.is_empty() || groups.iter().zip(&after).any(|(b, a)| a.count != b.count || a.token != b.token || a.copy != b.copy);
    if !changed {
        return None;
    }
    let mut out: Vec<(CardId, Option<Copiable>, PlayerId)> = Vec::with_capacity(tokens.len());
    let mut stands_for: Vec<Option<usize>> = Vec::with_capacity(tokens.len());
    for ((b, a), &start) in groups.iter().zip(&after).zip(&starts) {
        out.extend(std::iter::repeat((a.token, a.copy, b.controller)).take(a.count as usize));
        stands_for.extend((0..a.count as usize).map(|t| Some(start + t % b.count as usize)));
    }
    for a in &added {
        out.extend(std::iter::repeat((a.token, a.copy, a.controller)).take(a.count as usize));
        stands_for.extend(std::iter::repeat_n(None, a.count as usize));
    }
    Some((out, stands_for))
}

/// Damage about to be dealt (CR 614.1a, 120.3): what remains of it once every effect has applied - how much, and whom it
/// is dealt to, which a redirection ("is dealt to this creature instead") changes, after which the effects not yet
/// applied are judged again against the new recipient and its controller orders them (CR 614.6, 616.1f).
/// The prevention effects with more to do once the event has been dealt are noted in `uses` (`rest_of_preventions`).
#[cfg(test)]
pub(crate) fn damage(cx: &mut Cx, event: DamageEvent, uses: &mut Vec<PreventionUse>) -> DamageEvent {
    if !possible(cx, None) && shield_damage_applies(cx, &event).is_none() {
        return event;
    }
    let mut proposal = Proposal::Damage(event);
    apply_to(cx, &mut proposal, None, Some(uses));
    match proposal {
        Proposal::Damage(event) => event,
        _ => unreachable!("an effect cannot change the kind of the event"),
    }
}

/// One source's damage to one recipient after the entire event is formed.
/// Private provenance retains original returns without fragmenting the damage
/// record seen by prevention, toxic or damage triggers.
pub(crate) struct DamagePart {
    pub event: DamageEvent,
    pub origins: Vec<(Option<usize>, i64)>,
}

impl DamagePart {
    fn original(event: DamageEvent, index: usize) -> Self {
        Self { event: DamageEvent { amount: event.amount.max(0), ..event }, origins: vec![(Some(index), i64::from(event.amount))] }
    }

    /// Reductions consume the caller's stable assignment order. Increases keep
    /// each assignment's proportional share; remainders stay in that same order.
    /// Attribution never creates separate native damage events.
    fn resize(&mut self, total: i64) {
        let total = total.max(0);
        let before: i64 = self.origins.iter().map(|(_, n)| *n).sum();
        if total <= before {
            let mut remove = before - total;
            for (_, n) in &mut self.origins {
                let take = (*n).min(remove);
                *n -= take;
                remove -= take;
            }
        } else if before > 0 {
            let mut assigned = 0;
            for (_, n) in &mut self.origins {
                *n = (i128::from(*n) * i128::from(total) / i128::from(before)) as i64;
                assigned += *n;
            }
            let mut remainder = total - assigned;
            for (_, n) in &mut self.origins {
                if remainder == 0 { break; }
                *n += 1;
                remainder -= 1;
            }
        } else if total > 0 {
            self.origins.push((None, total));
        }
    }

    fn take(&mut self, mut amount: i64) -> Vec<(Option<usize>, i64)> {
        let mut taken = Vec::new();
        for (origin, n) in &mut self.origins {
            let part = (*n).min(amount);
            if part > 0 { taken.push((*origin, part)); }
            *n -= part;
            amount -= part;
        }
        taken
    }
}

struct PendingDamage {
    part: DamagePart,
    applied: Vec<Applied>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct DamageScope {
    source: ObjectId,
    target: Target,
    combat: bool,
}
impl DamageScope {
    fn of(d: &DamageEvent) -> Self {
        Self {
            source: d.source,
            target: d.target,
            combat: d.combat,
        }
    }
}
#[derive(Clone)]
struct PreventionReceipt {
    effect: Applied,
    scope: DamageScope,
    capacity: u32,
    spent: u32,
}
/// Ephemeral specific-allowance receipts for one native damage event. Pure replacement components share them;
/// nested damage calls are separate events at another time and save/restore an independent context (615.10).
/// No position, history, observation or per-card scratch field retains it.
#[derive(Clone, Default)]
pub(crate) struct DamageContext {
    receipts: Vec<PreventionReceipt>,
}
impl DamageContext {
    fn allowance(&self, effect: Applied, scope: DamageScope) -> Option<(u32, u32)> {
        self.receipts
            .iter()
            .find(|r| r.effect == effect && r.scope == scope)
            .map(|r| (r.capacity, r.spent))
    }
    fn freeze(&mut self, effect: Applied, scope: DamageScope, capacity: u32) {
        if self.allowance(effect, scope).is_none() {
            self.receipts.push(PreventionReceipt {
                effect,
                scope,
                capacity,
                spent: 0,
            });
        }
    }
    fn applied(&mut self, effect: Applied, scope: DamageScope, spent: u32) {
        // The allowance is frozen as the effect first applies to the event (`freeze`).
        let Some(r) = self.receipts.iter_mut().find(|r| r.effect == effect && r.scope == scope) else { return };
        r.spent += spent;
    }
}

fn same_damage(a: &DamageEvent, b: &DamageEvent) -> bool {
    DamageScope::of(a) == DamageScope::of(b)
}
fn is_prevention(origin: Origin) -> bool {
    matches!(
        origin,
        Origin::Protection
            | Origin::Prevention
            | Origin::GrantedPrevention(_)
            | Origin::ShieldCounter
    )
}
fn specific(cx: &Cx, key: Applied) -> Option<PreventionAmountFn> {
    if key.2 == Origin::Prevention
        && let PreventionAmount::Specific(amount) = printed_prevention(cx, key.0, key.1).0.amount
    {
        Some(amount)
    } else {
        None
    }
}
fn groups(parts: &[PendingDamage]) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        if let Some(group) = out
            .iter_mut()
            .find(|g| same_damage(&parts[g[0]].part.event, &p.part.event))
        {
            group.push(i);
        } else {
            out.push(vec![i]);
        }
    }
    out
}
fn aggregate(parts: &[PendingDamage], indices: &[usize]) -> DamagePart {
    let mut event = parts[indices[0]].part.event;
    event.amount = 0;
    let mut origins = Vec::new();
    for &i in indices {
        event.amount += parts[i].part.event.amount;
        event.heal_marked |= parts[i].part.event.heal_marked;
        event.shielded = event.shielded.or(parts[i].part.event.shielded);
        origins.extend_from_slice(&parts[i].part.origins);
    }
    origins.sort_by_key(|(origin, _)| origin.unwrap_or(usize::MAX));
    DamagePart { event, origins }
}
fn eligibility(cx: &Cx, parts: &[PendingDamage], group: &[usize], key: Applied) -> Vec<usize> {
    if specific(cx, key).is_some() {
        let scope = DamageScope::of(&parts[group[0]].part.event);
        // Freeze only when chosen; a zero first application is still an explicit allowance of zero.
        match cx.damage.as_ref().and_then(|c| c.allowance(key, scope)) {
            Some((capacity, spent)) if capacity <= spent => Vec::new(),
            _ => group.to_vec(),
        }
    } else {
        group
            .iter()
            .copied()
            .filter(|&i| !parts[i].applied.contains(&key))
            .collect()
    }
}

fn formed_candidates(cx: &Cx, parts: &[PendingDamage], group: &[usize]) -> Vec<Candidate> {
    let event = aggregate(parts, group).event;
    candidates_for(cx, &Proposal::Damage(event), &[], None, [None; 8], &|key| {
        let eligible = eligibility(cx, parts, group, key);
        // The part an effect has not had its opportunity at may be damage already prevented to 0, which is no event to
        // replace any more (CR 615.1, 614.6), whatever else the group holds.
        if eligible.is_empty() {
            return None;
        }
        let part = aggregate(parts, &eligible).event;
        (part.amount > 0).then_some(Proposal::Damage(part))
    })
}
fn ancestors(parts: &[PendingDamage], selected: &[usize], key: Applied) -> Vec<Applied> {
    let mut out = Vec::new();
    for &i in selected {
        for &a in &parts[i].applied {
            if a != key && !out.contains(&a) {
                out.push(a);
            }
        }
    }
    out
}
/// Use the established exact attribution policy over the selected quantity, without coalescing its effect histories.
fn resize_selected(parts: &mut [PendingDamage], selected: &[usize], total: i64) {
    let mut slots = Vec::new();
    for &i in selected {
        for (j, &(origin, n)) in parts[i].part.origins.iter().enumerate() {
            slots.push((origin.unwrap_or(usize::MAX), i, j, n));
        }
    }
    slots.sort_by_key(|&(origin, i, j, _)| (origin, i, j));
    let mut allocation = DamagePart {
        event: parts[selected[0]].part.event,
        origins: slots
            .iter()
            .enumerate()
            .map(|(k, s)| (Some(k), s.3))
            .collect(),
    };
    allocation.resize(total);
    for (slot, n) in allocation.origins {
        if let Some(slot) = slot {
            let (_, i, j, _) = slots[slot];
            parts[i].part.origins[j].1 = n;
        } else {
            parts[selected[0]].part.origins.push((None, n));
        }
    }
    for &i in selected {
        parts[i].part.event.amount = parts[i].part.origins.iter().map(|(_, n)| *n).sum::<i64>().max(0) as u32;
    }
}
fn distribute(
    parts: &mut Vec<PendingDamage>,
    selected: &[usize],
    after: DamageEvent,
    added: Vec<DamageComponent>,
    mut lineage: Vec<Applied>,
    key: Applied,
) {
    let redirected: i64 = added
        .iter()
        .map(|c| match c {
            DamageComponent::Redirected { amount, .. } => i64::from(*amount),
            _ => 0,
        })
        .sum();
    resize_selected(
        parts,
        selected,
        i64::from(after.amount) + redirected,
    );
    if !lineage.contains(&key) {
        lineage.push(key);
    }
    for &i in selected {
        let amount = parts[i].part.event.amount;
        parts[i].part.event = DamageEvent { amount, ..after };
        if !parts[i].applied.contains(&key) {
            parts[i].applied.push(key);
        }
    }
    for component in added {
        match component {
            DamageComponent::Additional(event) => {
                let event = DamageEvent {
                    amount: event.amount.max(0),
                    ..event
                };
                let applied = lineage.clone();
                parts.push(PendingDamage {
                    part: DamagePart {
                        event,
                        origins: vec![(None, i64::from(event.amount))],
                    },
                    applied,
                });
            }
            DamageComponent::Redirected { target, amount } => {
                let mut left = amount.max(0);
                let mut order = selected.to_vec();
                order.sort_by_key(|&i| {
                    (
                        parts[i]
                            .part
                            .origins
                            .iter()
                            .filter_map(|(o, n)| (*n > 0).then_some(*o).flatten())
                            .min()
                            .unwrap_or(usize::MAX),
                        i,
                    )
                });
                for i in order {
                    let take = left.min(parts[i].part.event.amount);
                    if take == 0 {
                        continue;
                    }
                    let origins = parts[i].part.take(i64::from(take));
                    parts[i].part.event.amount -= take;
                    let event = DamageEvent {
                        target,
                        amount: take,
                        heal_marked: false,
                        shielded: None,
                        ..after
                    };
                    // The redirected part is the same event modified (CR 614.5): every effect that has had its
                    // opportunity at it, a prevention effect among them, has had it at what it became.
                    let applied = parts[i].applied.clone();
                    parts.push(PendingDamage {
                        part: DamagePart { event, origins },
                        applied,
                    });
                    left -= take;
                }
            }
        }
    }
}
fn finish_damage(parts: &[PendingDamage]) -> Vec<DamagePart> {
    let mut out: Vec<_> = groups(parts).iter().map(|g| aggregate(parts, g)).collect();
    out.sort_by_key(|p| {
        p.origins
            .iter()
            .filter_map(|(origin, amount)| (*amount > 0).then_some(*origin).flatten())
            .min()
            .unwrap_or(usize::MAX)
    });
    out
}
struct DamagePlan {
    order: Vec<DamageScope>,
    established: Vec<DamageScope>,
    shields: Vec<u32>,
    choosers: Vec<(DamageScope, PlayerId)>,
    eligible: Vec<DamageScope>,
}
impl DamagePlan {
    fn new(cx: &mut Cx, events: &[DamageEvent]) -> Self {
        let plan = crate::ops::damage_source_order(cx, events);
        Self {
            order: plan
                .order
                .into_iter()
                .map(|i| DamageScope::of(&events[i]))
                .collect(),
            established: plan
                .established
                .into_iter()
                .map(|i| DamageScope::of(&events[i]))
                .collect(),
            shields: Self::shields(cx),
            choosers: events
                .iter()
                .map(|e| (DamageScope::of(e), affected(cx, &Proposal::Damage(*e))))
                .collect(),
            eligible: events
                .iter()
                .filter(|e| shields(cx, e).is_some())
                .map(DamageScope::of)
                .collect(),
        }
    }
    fn shields(cx: &Cx) -> Vec<u32> {
        cx.s()
            .effects
            .iter()
            .filter_map(|e| {
                matches!(
                    e.kind,
                    EffectKind::Prevention {
                        shield: Shield::Next(_),
                        ..
                    }
                )
                .then_some(e.timestamp)
            })
            .collect()
    }
    fn valid(&self, cx: &Cx, events: &[DamageEvent]) -> bool {
        events.iter().all(|e| {
            self.order.contains(&DamageScope::of(e))
                && self
                    .choosers
                    .contains(&(DamageScope::of(e), affected(cx, &Proposal::Damage(*e))))
        }) && Self::shields(cx).iter().all(|t| self.shields.contains(t))
            && events
                .iter()
                .filter(|e| shields(cx, e).is_some())
                .all(|e| self.eligible.contains(&DamageScope::of(e)))
    }
}
fn damage_event_inner(
    cx: &mut Cx,
    events: &[DamageEvent],
    uses: &mut Vec<PreventionUse>,
    shielded: &mut Vec<ObjectId>,
) -> Vec<DamagePart> {
    let mut parts: Vec<_> = events
        .iter()
        .enumerate()
        .map(|(i, &event)| {
            // A new native damage call is another event/time (615.10), even inside a replacement callback.
            // True replacement ancestor opportunities still inherit (614.5); prevention opportunities do not.
            let applied = cx
                .lineage
                .iter()
                .copied()
                .filter(|a| !is_prevention(a.2))
                .collect();
            PendingDamage {
                part: DamagePart::original(event, i),
                applied,
            }
        })
        .collect();
    let mut plan: Option<DamagePlan> = None;
    loop {
        for p in &mut parts {
            if let Some(&id) = shielded
                .iter()
                .find(|&&id| p.part.event.target == Target::Object(id))
            {
                p.part.event.shielded = Some(id);
            }
        }
        let grouped = groups(&parts);
        let events: Vec<_> = grouped.iter().map(|g| aggregate(&parts, g).event).collect();
        if plan.as_ref().is_none_or(|p| !p.valid(cx, &events)) {
            plan = Some(DamagePlan::new(cx, &events));
        }
        let Some(current) = plan.as_ref() else { break };
        let order: Vec<_> = current
            .order
            .iter()
            .filter_map(|scope| events.iter().position(|e| DamageScope::of(e) == *scope))
            .collect();
        let mut choice = None;
        let mut unasked = false;
        for at in order {
            let group = &grouped[at];
            let found = formed_candidates(cx, &parts, group);
            if !found.is_empty() {
                if !current.established.contains(&DamageScope::of(&events[at])) {
                    unasked = true;
                    break;
                }
                let proposal = Proposal::Damage(events[at]);
                let pick = if found.len() > 1 && !found.iter().all(|c| prevents_all(cx, c)) {
                    cx.instruction(
                        |cx| cx.own_text(false, |cx| choose(cx, &proposal, &found)),
                        |_| true,
                    )
                } else {
                    Some(0)
                };
                let Some(pick) = pick else {
                    // A concession changed the participants while choosing. Rebuild the event's APNAP plan.
                    unasked = true;
                    break;
                };
                choice = Some((group.clone(), found[pick]));
                break;
            }
            if events[at].amount > 0
                && let Some(by) = crate::ops::protection_prevents(
                    cx,
                    events[at].source,
                    events[at].target,
                    events[at].combat,
                )
            {
                let mut result = aggregate(&parts, group);
                prevent(cx, &mut result.event, events[at].amount, by);
                resize_selected(&mut parts, group, i64::from(result.event.amount));
            }
        }
        if unasked {
            plan = Some(DamagePlan::new(cx, &events));
            continue;
        }
        let Some((group, candidate)) = choice else {
            break;
        };
        let key = (candidate.source, candidate.index, candidate.origin);
        if let Origin::Granted(timestamp) = candidate.origin
            && granted(cx, timestamp).is_some_and(|(_, d)| d.once)
        {
            let Some((&effect, def)) = granted(cx, timestamp) else { break };
            // Eligibility is captured for the whole once-only event before it changes any original assignment.
            let snapshots: Vec<_> = groups(&parts)
                .iter()
                .filter(|g| formed_candidates(cx, &parts, g).contains(&candidate))
                .map(|g| eligibility(cx, &parts, g, key))
                .collect();
            let mut took = false;
            for selected in snapshots {
                let before = aggregate(&parts, &selected).event;
                let mut proposal = Proposal::Damage(before);
                let mut lineage = ancestors(&parts, &selected, key);
                lineage.push(key);
                let outer = std::mem::replace(&mut cx.lineage, lineage.clone());
                let mut added = Vec::new();
                took |= cx.instruction(
                    |cx| {
                        cx.own_text(false, |cx| {
                            apply_granted(
                                cx,
                                &mut proposal,
                                candidate.source,
                                effect,
                                def,
                                &lineage,
                                false,
                                &mut added,
                            )
                        })
                    },
                    |_| true,
                );
                cx.lineage = outer;
                let after = match proposal { Proposal::Damage(after) => after, _ => before };
                if let Some(id) = after.shielded
                    && !shielded.contains(&id)
                {
                    shielded.push(id);
                }
                distribute(&mut parts, &selected, after, added, lineage, key);
            }
            if took {
                used_up(cx, timestamp);
            }
        } else {
            let selected = eligibility(cx, &parts, &group, key);
            let before = aggregate(&parts, &selected).event;
            let mut proposal = Proposal::Damage(before);
            let outer = std::mem::replace(&mut cx.lineage, ancestors(&parts, &selected, key));
            let progress = cx.instruction(
                |cx| {
                    cx.own_text(false, |cx| {
                        apply_each(cx, &mut proposal, None, Some(uses), true, Some(candidate))
                    })
                },
                |_| true,
            );
            cx.lineage = outer;
            let after = match proposal { Proposal::Damage(after) => after, _ => before };
            if let Some(id) = after.shielded
                && !shielded.contains(&id)
            {
                shielded.push(id);
            }
            match progress {
                Some(DamageProgress::Expanded { added, applied }) => distribute(&mut parts, &selected, after, added, applied, key),
                // It no longer applied to its part: it has had its opportunity there (CR 614.5), and does nothing.
                None => {
                    let marked = if selected.is_empty() { &group } else { &selected };
                    for &i in marked {
                        if !parts[i].applied.contains(&key) {
                            parts[i].applied.push(key);
                        }
                    }
                }
            }
        }
    }
    finish_damage(&parts)
}

/// Forms replacement-introduced damage atomically (120.4b-d), retaining per-effect original lineage (614.5) and the
/// distinct source-event allowance of specific static prevention (615.10). Final records alone coalesce for triggers.
pub(crate) fn damage_event(
    cx: &mut Cx,
    events: &[DamageEvent],
    uses: &mut Vec<PreventionUse>,
    shielded: &mut Vec<ObjectId>,
) -> Vec<DamagePart> {
    if !possible(cx, None)
        && events
            .iter()
            .all(|d| shield_damage_applies(cx, d).is_none())
    {
        let parts: Vec<_> = events
            .iter()
            .enumerate()
            .map(|(i, &e)| PendingDamage {
                part: DamagePart::original(e, i),
                applied: Vec::new(),
            })
            .collect();
        let mut out = finish_damage(&parts);
        for p in &mut out {
            let d = p.event;
            if d.amount > 0
                && let Some(by) = crate::ops::protection_prevents(cx, d.source, d.target, d.combat)
            {
                prevent(cx, &mut p.event, d.amount, by);
                p.resize(i64::from(p.event.amount));
            }
        }
        return out;
    }
    let saved = cx.damage.take();
    let lineage = cx.lineage.clone();
    cx.damage = Some(Box::default());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        damage_event_inner(cx, events, uses, shielded)
    }));
    cx.damage = saved;
    if result.is_err() {
        cx.lineage = lineage;
    }
    match result {
        Ok(parts) => parts,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// Life `player` is about to gain (`amount` above zero) or lose (below zero) (CR 119.3, 614.1a): what remains of it once
/// every effect has applied, in the order the player chooses (CR 616.1), each judged again on the event as the one
/// before it left it (CR 616.1f), so a gain turned into a loss meets the effects on losing life. Zero is no event
/// (CR 119.10).
#[cfg(test)]
pub fn life_change(cx: &mut Cx, player: PlayerId, amount: i64) -> i64 {
    life_change_proposal(cx, LifeChange::from_signed(player, amount)).signed()
}

/// Apply replacements before narrowing a transient unsigned magnitude (CR 614.1a, 616.1).
pub(crate) fn life_change_proposal(cx: &mut Cx, change: LifeChange) -> LifeChange {
    if !possible(cx, None) {
        return change;
    }
    let mut proposal = Proposal::LifeChange(change);
    apply(cx, &mut proposal);
    match proposal {
        Proposal::LifeChange(l) => l,
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
        let mut cx = Cx { g: game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        crate::ops::draw_cards(&mut cx, ME, count);
        assert!(cx.s().agenda[frame].pending.is_none(), "every question was answered");
        let s = cx.s();
        (s.zone(Zone::hand(ME)).len(), s.zone(Zone::library(ME)).len(), s.player(ME).life)
    }

    /// A copy of a merged permanent carries exception texts on both its topmost and lower components. Replacement
    /// and prevention lookup must use the same component of origin as the static-ability walk, not the text's index.
    #[test]
    fn replacement_texts_keep_the_copy_exceptions_component_of_origin() {
        let mut g = game(&[(&FILLER, Zone::BATTLEFIELD)], Vec::new());
        let id = g.state.battlefield()[0];
        let (top, lower, below, own) = (g.cards.id_of(&FILLER), g.cards.id_of(&DOUBLER), g.cards.id_of(&GAINER), g.cards.id_of(&PEACE));
        let merged = g.state.merged_sources_push(&[Copiable::of(lower).with_text(below)]);
        let values = Copiable { merged, ..Copiable::of(top).with_text(own) };
        g.state.update_obj(id, |o| o.copy = Some(values));
        let link = Some(g.cards.component_link(&g.state, id, lower));
        let frame = g.state.agenda.len() - 1;
        let cx = Cx { g: &mut g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        let texts: Vec<_> = source_texts_linked(&cx, id).map(|(d, l)| (d.name, l)).collect();
        assert_eq!(texts, [(FILLER.name, None), (DOUBLER.name, link), (GAINER.name, link), (PEACE.name, None)]);
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

    #[test]
    fn a_concession_before_an_optional_replacement_removes_its_source() {
        static FAMILIAR: CardDef = CardDef {
            name: "Obstinate Familiar probe",
            types: TypeSet::of(&[CardType::Creature]),
            replacements: &[replacement(ReplacementKind::Draw, you_would_draw, instead, "skip that draw instead").optional(&SKIP)],
            ..CardDef::BLANK
        };
        let owner = PlayerId(2);
        let table = Box::leak(Box::new(CardTable::new([&FILLER, &FAMILIAR])));
        let deck = std::iter::repeat_n(table.id_of(&FILLER), 17).collect();
        let mut game = Game::new(table, &crate::Setup { decks: vec![deck, Vec::new(), Vec::new(), Vec::new()], ..Default::default() });
        let source = game.state.push_obj(Obj::new(table.id_of(&FAMILIAR), owner, Zone::BATTLEFIELD));
        game.state.update_obj(source, |o| o.controller = ME);
        game.state.zone_push(Zone::BATTLEFIELD, source);
        game.state.frame_push(Frame::new(FrameKind::Settle));
        let frame = game.state.agenda.len() - 1;
        game.state.update_frame(frame, |f| f.conceded.push((0, owner)));
        assert_eq!(draw(&mut game, 1), (8, 9, 20));
        assert!(game.state.player(owner).lost);
    }

    /// CR 109.5: "you may ... instead" is the choice of the effect's controller, not of the affected player (CR 616.1 gives
    /// that one only the order). Your effect on an opponent's draw asks you, and taken, they draw nothing.
    #[test]
    fn an_optional_replacement_is_chosen_by_its_controller_not_the_affected_player() {
        let mut game = game(&[(&DENIER, Zone::BATTLEFIELD)], Vec::new());
        let them = PlayerId(1);
        let frame = game.state.agenda.len() - 1;
        game.state.update_frame(frame, |f| f.answers = vec![(Some(ME), Answer::Yes)]);
        let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
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
            put_by: None,
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
            commander_components: [ObjectId::NONE; 8],
            omen: false,
            rebound: false,
            on_adventure: false,
            rebounding: false,
            destroy: false,
            regenerated: false,
            regeneration_shield: false,
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

#[cfg(test)]
#[path = "prevention_then_tests.rs"]
mod prevention_then_tests;
