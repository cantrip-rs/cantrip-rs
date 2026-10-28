//! What a player can do with priority, and doing it: playing a land (CR 305), casting a spell (CR 601), activating
//! an ability (CR 602, 605), and paying for them (CR 601.2g-h, 118.12).
//!
//! The options are listed in the C# engine's order: mana abilities, other activated abilities and instant-speed
//! casts, then (at sorcery timing) land plays and the remaining casts. A scenario that names "the Forest" means the
//! first one offered.

use cantrip_api::*;
use cantrip_state::Span;

use crate::layers::{self, live, summoning_sick};
use crate::{Cx, Flow, R, Stop, counters, ops, static_rules};
use crate::resolve::announced_x;

pub fn sorcery_timing(cx: &Cx, player: PlayerId) -> bool {
    let s = cx.s();
    s.turn.active == player && s.turn.step().is_main() && s.stack.is_empty()
}

/// CR 304.5, 602.5e, 605.3a: a mana ability may be used while paying unless it explicitly requires priority.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ManaWindow {
    Priority,
    Payment,
}

impl ManaWindow {
    fn allows(self, def: &ActivatedDef) -> bool {
        !def.priority_only || self == Self::Priority
    }
}

/// Everything `player` may do now beyond passing.
pub fn priority_options(cx: &mut Cx, player: PlayerId) -> Vec<ActionOption> {
    // The spells the cards would be are judged as the offer asks (`SpellsAsCast`): where an answer it took for granted turns
    // out otherwise, it is made again with what is now known, until it asks nothing new.
    let outer = cx.g.spells_as_cast.replace(SpellsAsCast::open(cx, player));
    let mut options = options_offered(cx, player);
    while judge_spells_as_cast(cx) {
        options = options_offered(cx, player);
    }
    cx.g.spells_as_cast.replace(outer);
    casts_forbidden_on_the_stack(cx, player, &mut options);
    casts_judged_on_the_stack(cx, player, &mut options);
    options
}

/// The face of `card` a cast by the way `way` casts, as `ops::as_cast` names it: `None` for the card itself (CR 709.3,
/// 715.3, 720.3, 712.8a, 702.103b).
fn cast_face_id(cx: &Cx, card: ObjectId, way: u8) -> Option<CardId> {
    let whole = cx.card(cx.s().obj(card).card);
    let face = cast_face(whole, way);
    (!std::ptr::eq(face, whole)).then(|| cx.card_id(face))
}

/// What the offer of `priority_options` asks of the spells the cards `player` could cast would be (CR 601.2a, 601.3e): a
/// spell is put on the stack before anything about it is announced, as the face it is cast as alone, under its caster, and a
/// permission, an alternative cost or a cost change that reads it reads that spell - with the effects that apply to it there,
/// not the card where it is. Open only for the length of that question (`Game::spells_as_cast`), closed at any other time; no
/// part of the position.
///
/// The card where it is has the face's own values (`layers::chars_as_face`) and the same effects as the spell would, unless
/// a static ability that functions outside the battlefield exists (`layers::statics_everywhere`: Arcane Adaptation's
/// "creature spells you control" reads the controller and the zone), or the card has a characteristic-defining ability of
/// its own, which could read its zone or controller (CR 604.3): only then is anything put on the stack for the moment
/// (`ops::as_cast`), and only for the card and face the offer asks about, once each. An answer not yet judged is taken as the
/// card where it is, and the question is asked again once it is judged, if it turns out otherwise (`judge_spells_as_cast`).
#[derive(Default)]
pub(crate) struct SpellsAsCast {
    open: bool,
    player: PlayerId,
    /// A static ability that functions outside the battlefield exists.
    everywhere: bool,
    /// The question is asked with sorcery timing (CR 307.5): whether a cast is as though it had flash changes only which
    /// pass of the offer makes it, so it is not judged.
    sorcery: bool,
    /// The card types an effect lets the player cast as though they had flash (`static_rules::flash_types`), and the
    /// permanents whose static abilities may let them (`static_rules::flash_granters`).
    flash_types: TypeSet,
    granters: Vec<(ObjectId, PlayerId, &'static CardDef)>,
    /// Each card and face (`None`: the card itself) judged: whether a permission lets it be cast as though it had flash, and
    /// the characteristics of the spell, where they are not those of the card where it is.
    flash: Vec<(SpellFace, bool)>,
    chars: Vec<(SpellFace, Option<Chars>)>,
    /// Those the offer asked about that are not judged yet.
    asked_flash: Vec<SpellFace>,
    asked_chars: Vec<SpellFace>,
}

/// A card and the face it would be cast as (`None`: the card itself), as `SpellsAsCast` keeps what it judges of them.
type SpellFace = (ObjectId, Option<CardId>);

impl SpellsAsCast {
    fn open(cx: &Cx, player: PlayerId) -> SpellsAsCast {
        SpellsAsCast {
            open: true,
            player,
            everywhere: layers::statics_everywhere(cx.cards(), cx.s()),
            sorcery: sorcery_timing(cx, player),
            flash_types: static_rules::flash_types(cx, player),
            granters: static_rules::flash_granters(cx),
            ..SpellsAsCast::default()
        }
    }

    /// Whether the spell `card` would be may differ from the card where it is: see the type.
    fn judged(&self, cx: &Cx, card: ObjectId) -> bool {
        self.everywhere || own_characteristic_defining(cx, card)
    }
}

/// Whether `card`'s card has a characteristic-defining ability (CR 604.3), which functions in every zone and could read the
/// zone or controller of its object.
fn own_characteristic_defining(cx: &Cx, card: ObjectId) -> bool {
    cx.card(cx.cards().current(cx.s().obj(card))).statics.iter().any(|st| st.own)
}

/// Whether a permission lets the player `priority_options` asks for cast `card` as `face` (`None`: itself) as though it had
/// flash (CR 307.5, 601.3b, 702.8a): an effect's for some card types (Teferi, Time Raveler's "sorcery spells"), a static
/// ability's for some spells (Rattlechains's "Spirit spells", Sigarda's Aid's "Aura and Equipment spells"), judged on the spell
/// it would be (`SpellsAsCast`). Not yet judged, it is taken as not, and the offer asks for it to be (`ask_as_though_flash`).
fn as_though_flash(cx: &Cx, card: ObjectId, face: Option<CardId>) -> bool {
    as_though_flash_judged(cx, card, face).unwrap_or_else(|| {
        ask_as_though_flash(cx, card, face);
        false
    })
}

/// `as_though_flash`, or `None` where it is not yet judged on the spell the card would be.
fn as_though_flash_judged(cx: &Cx, card: ObjectId, face: Option<CardId>) -> Option<bool> {
    let key = (card, face);
    let t = cx.g.spells_as_cast.borrow();
    if !t.open || (t.granters.is_empty() && t.flash_types == TypeSet::NONE) {
        return Some(false);
    }
    if !t.judged(cx, card) {
        let view = face.map(|face| AsFace { cx, spell: card, face });
        let q: &dyn Query = match &view {
            Some(view) => view,
            None => cx,
        };
        return Some(!flash_permissions(cx, q, &t.granters, t.player, card).is_empty());
    }
    if t.sorcery {
        return Some(false);
    }
    t.flash.iter().find(|(k, _)| *k == key).map(|&(_, granted)| granted)
}

/// Asks for `as_though_flash` of `card` cast as `face` to be judged on the spell it would be (`judge_spells_as_cast`).
fn ask_as_though_flash(cx: &Cx, card: ObjectId, face: Option<CardId>) {
    let mut t = cx.g.spells_as_cast.borrow_mut();
    if !t.asked_flash.contains(&(card, face)) {
        t.asked_flash.push((card, face));
    }
}

/// The characteristics `card` cast as `face` (`None`: itself) would have as the spell on the stack under the player
/// `priority_options` asks for, where they are not those of the card where it is (`SpellsAsCast`): what an alternative cost's
/// or a cost change's rule reads of the spell.
fn spell_chars(cx: &Cx, card: ObjectId, face: Option<CardId>) -> Option<Chars> {
    let key = (card, face);
    {
        let t = cx.g.spells_as_cast.borrow();
        if !t.open || !t.judged(cx, card) {
            return None;
        }
        if let Some(&(_, chars)) = t.chars.iter().find(|(k, _)| *k == key) {
            return chars;
        }
        if t.asked_chars.contains(&key) {
            return None;
        }
    }
    cx.g.spells_as_cast.borrow_mut().asked_chars.push(key);
    None
}

/// Judges what the offer asked of the spells the cards would be and had not judged (`SpellsAsCast`), each with the card put
/// on the stack for the moment as that spell (`ops::as_cast`). Whether any answer differs from the one the offer took for
/// granted, so that it must be made again.
fn judge_spells_as_cast(cx: &mut Cx) -> bool {
    let (player, granters, asked_flash, asked_chars) = {
        let mut t = cx.g.spells_as_cast.borrow_mut();
        (t.player, t.granters.clone(), std::mem::take(&mut t.asked_flash), std::mem::take(&mut t.asked_chars))
    };
    let mut changed = false;
    for (card, face) in asked_flash {
        let granted = ops::as_cast(cx, card, face, player, |cx| !flash_permissions(cx, cx, &granters, player, card).is_empty());
        changed |= granted;
        cx.g.spells_as_cast.borrow_mut().flash.push(((card, face), granted));
    }
    for (card, face) in asked_chars {
        let differs = chars_as_cast(cx, card, face, player);
        changed |= differs.is_some();
        cx.g.spells_as_cast.borrow_mut().chars.push(((card, face), differs));
    }
    changed
}

/// `chars_as_cast` where it could be other than the card where it is (`SpellsAsCast`), for a cast an effect makes as it
/// resolves.
fn chars_as_cast_now(cx: &mut Cx, card: ObjectId, face: Option<CardId>, player: PlayerId) -> Option<Chars> {
    if !layers::statics_everywhere(cx.cards(), cx.s()) && !own_characteristic_defining(cx, card) {
        return None;
    }
    chars_as_cast(cx, card, face, player)
}

/// The characteristics `card` cast by `player` as `face` (`None`: itself) would have as the spell on the stack (`ops::as_cast`),
/// where they are not those of the card where it is, judged as that face: what an alternative cost or a cost change reads.
fn chars_as_cast(cx: &mut Cx, card: ObjectId, face: Option<CardId>, player: PlayerId) -> Option<Chars> {
    let here = {
        let _face_up = layers::judged_face_up(cx.s(), card);
        match face {
            Some(face) => layers::chars_as_face(cx.cards(), cx.s(), &cx.g.log, card, face),
            None => cx.chars(card),
        }
    };
    let mut there = ops::as_cast(cx, card, face, player, |cx| cx.chars(card));
    // A spell is one (CR 112.1), which the card where it is was not: no alternative cost's or cost change's rule asks.
    there.spell = here.spell;
    (there != here).then_some(there)
}

/// Any query, with the spell a card would be read as `chars` (`spell_chars`): an alternative cost's or a cost change's rule
/// asks its characteristics of the spell, and everything else of the card where it is (Omniscience's "from your hand").
struct AsSpell<'a> {
    q: &'a dyn Query,
    spell: ObjectId,
    chars: Chars,
}

impl Query for AsSpell<'_> {
    fn linked_card(&self, card: CardId) -> CardId {
        self.q.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.q.text_linked(source)
    }

    fn state(&self) -> &GameState {
        self.q.state()
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.q.card(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        if id == self.spell { self.chars } else { self.q.chars(id) }
    }

    /// Its mana cost is the face's, as the card where it is has it (CR 202.3).
    fn mana_value(&self, id: ObjectId) -> u32 {
        self.q.mana_value(id)
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        self.q.token_chars(card, copy)
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.q.card_id(def)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        self.q.lost_all_abilities(id)
    }

    fn events(&self) -> &[Record] {
        self.q.events()
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        self.q.can_target(asker, target)
    }
}

/// The casts among `options`, and the suspends among them (CR 702.62a: "any time you could begin to cast this card"), that a
/// static ability forbids for the qualities of the spell each would cast (CR 101.2, 601.3: Gaddock Teeg, Deafening Silence),
/// judged on that spell on the stack under its caster, the face it is cast as alone (CR 601.2a, 601.3e, 709.3a, 715.3a,
/// 712.8a, `ops::as_cast`): what an effect makes it there as their spell (Arcane Adaptation's "creature spells you control")
/// is what the ban reads. Judged only where such a ban exists, each face once.
fn casts_forbidden_on_the_stack(cx: &mut Cx, player: PlayerId, options: &mut Vec<ActionOption>) {
    let forbidders = static_rules::spell_forbidders(cx);
    if forbidders.is_empty() {
        return;
    }
    let mut judged: Vec<((ObjectId, Option<CardId>), bool)> = Vec::new();
    let mut kept = Vec::with_capacity(options.len());
    for o in std::mem::take(options) {
        let cast = match (o.kind, o.source) {
            (ActionKind::CastSpell, Some(card)) => Some((card, cast_face_id(cx, card, o.index))),
            (ActionKind::ActivateAbility, Some(card)) if suspends(cx, card, o.index) => Some((card, None)),
            _ => None,
        };
        if let Some(key) = cast {
            let forbidden = match judged.iter().find(|(k, _)| *k == key) {
                Some(&(_, forbidden)) => forbidden,
                None => {
                    let forbidden = forbidden_as_cast(cx, &forbidders, key.0, key.1, player);
                    judged.push((key, forbidden));
                    forbidden
                }
            };
            if forbidden {
                continue;
            }
        }
        kept.push(o);
    }
    *options = kept;
}

/// Whether the activated ability `index` of `card` is one that may be activated only when the card could be cast (suspend,
/// CR 702.62a: `Timing::AsCast`), from a zone that is not the battlefield.
fn suspends(cx: &Cx, card: ObjectId, index: u8) -> bool {
    !cx.s().obj(card).zone.is(ZoneKind::Battlefield)
        && cx.chars(card).abilities().nth(index as usize).and_then(|ability| activated(cx, ability)).is_some_and(|def| def.timing == Timing::AsCast)
}

/// Whether one of `forbidders` (`static_rules::spell_forbidders`) forbids `player` to cast `card` as `face` (`None`: the card
/// itself), judged on the spell it would be on the stack under them (CR 101.2, 601.3, 601.3e, `ops::as_cast`).
fn forbidden_as_cast(cx: &mut Cx, forbidders: &[(ObjectId, PlayerId, &'static CardDef)], card: ObjectId, face: Option<CardId>, player: PlayerId) -> bool {
    ops::as_cast(cx, card, face, player, |cx| static_rules::spell_forbidden(&*cx, forbidders, player, card))
}

/// The casts among `options` judged as the spells they would be (CR 601.2a-c): a spell is put on the stack before its modes
/// and targets are announced, as the face it is cast as (CR 709.3, 715.3, 712.8, 720.3) with the effects that apply to it
/// there, and what may be targeted is judged on that spell (`ops::as_cast`). The offer judged the card where it is on
/// everything that does not read it (`layers::can_target`); where something on the board reads the spell that would target
/// it (`layers::targeting_readers`), each cast with targets is judged again there: its targets announced with it
/// are those both judgements allow, and one with no legal announcement there is not offered.
fn casts_judged_on_the_stack(cx: &mut Cx, player: PlayerId, options: &mut Vec<ActionOption>) {
    // Overload (CR 702.96a) has "each" for "target": nothing is targeted.
    let judged = |o: &ActionOption, cx: &Cx| {
        o.kind == ActionKind::CastSpell
            && o.index != WAY_OVERLOAD
            && o.source.is_some_and(|card| {
                let face = cast_face(cx.card(cx.s().obj(card).card), o.index);
                !matches!(cx.s().obj(card).zone.kind(), ZoneKind::Battlefield | ZoneKind::Stack)
                    && (face.modal.is_some() || !cx.cards().spell_targets_of(face).is_empty())
            })
    };
    if !options.iter().any(|o| judged(o, cx)) {
        return;
    }
    let Some(readers) = layers::targeting_readers(cx.cards(), cx.s(), &cx.g.log) else { return };
    let mut kept = Vec::with_capacity(options.len());
    for mut o in std::mem::take(options) {
        if judged(&o, cx)
            && let Some(card) = o.source
        {
            let whole = cx.card(cx.s().obj(card).card);
            let face = cast_face(whole, o.index);
            let face_id = (!std::ptr::eq(face, whole)).then(|| cx.card_id(face));
            let (way, announced) = (o.index, targets_with_cast(cx.cards(), face));
            // Targets announced with the cast, none of which anything refuses for the spell targeting it, are its own there.
            if announced && !o.target_candidates.iter().any(|&t| readers.read_by(t)) {
                kept.push(o);
                continue;
            }
            let (legal, candidates) = ops::as_cast(cx, card, face_id, player, |cx| {
                let mut there = ActionOption::on(ActionKind::CastSpell, card, way);
                let legal = offer_spell(cx, &Asker::spell(card, player), face, &mut there);
                (legal, there.target_candidates)
            });
            if !legal {
                continue;
            }
            if announced {
                o.target_candidates.retain(|t| candidates.contains(t));
                if o.target_candidates.is_empty() && !o.targets_optional {
                    continue;
                }
            }
        }
        kept.push(o);
    }
    *options = kept;
}

/// Everything `player` may do now beyond passing, a spell not yet cast judged where its card is (`priority_options`) but for
/// what reads the spell it would be (`SpellsAsCast`).
fn options_offered(cx: &Cx, player: PlayerId) -> Vec<ActionOption> {
    let mut options = Vec::new();
    mana_ability_options(cx, player, ManaWindow::Priority, &mut options);
    // Split second (CR 702.61a-b): with such a spell on the stack, mana abilities and special actions are all anyone may do
    // (`activated_options` offers only the special actions then).
    activated_options(cx, player, &std::cell::OnceCell::new(), &mut options);
    if split_second(cx) {
        return options;
    }
    let available = std::cell::OnceCell::new();
    castable_ways(cx, player, &available, true, &mut options);
    if sorcery_timing(cx, player) {
        land_plays(cx, player, &mut options);
        castable_ways(cx, player, &available, false, &mut options);
        take_companion_option(cx, player, &available, &mut options);
    }
    options
}

/// What taking a companion costs (CR 702.139a, 116.2g).
const COMPANION_COST: ManaCost = generic(3);

/// The companion `player` revealed before the game (CR 103.2b, 702.139a), while it is still outside the game, where it stays
/// revealed: once it has been put into their hand it is a new object (CR 400.7), so the special action is had once a game (CR
/// 116.2g).
pub fn revealed_companion(state: &GameState, player: PlayerId) -> Option<ObjectId> {
    let card = state.player(player).companion?;
    (live(state, card) && state.obj(card).zone == Zone::sideboard(player)).then_some(card)
}

/// Taking the companion (CR 702.139a, 116.2g): "any time you have priority and the stack is empty, but only during a main phase
/// of your turn" - sorcery timing, which the caller has judged - for {3}, offered only where the player could pay it, as a
/// special action whose cost they cannot meet is not (`activated_options`). It is no ability (CR 116.1): nothing that changes
/// or forbids activating abilities touches it, and mana that may only be spent on abilities or spells can't pay for it
/// (CR 106.6).
fn take_companion_option(cx: &Cx, player: PlayerId, available: &std::cell::OnceCell<Available>, options: &mut Vec<ActionOption>) {
    let Some(card) = revealed_companion(cx.s(), player) else { return };
    if available.get_or_init(|| self::available(cx, player)).covers_for(&COMPANION_COST, &ManaPayment::Other) {
        options.push(ActionOption::on(ActionKind::TakeCompanion, card, 0));
    }
}

/// The special action (CR 116.2g): {3} paid, and the card put into its owner's hand from outside the game (CR 400.11b), where
/// it stays in the game until it ends (CR 702.139c).
fn take_companion(cx: &mut Cx, player: PlayerId, card: ObjectId) -> R<()> {
    pay_mana(cx, player, card, COMPANION_COST)?;
    ops::bring_into_hand(cx, &[card]);
    Ok(())
}

/// Whether a spell with split second is on the stack (CR 702.61a): asked of each spell's characteristics, which is nothing
/// at all for nearly every priority pass.
fn split_second(cx: &Cx) -> bool {
    cx.s().stack.iter().any(|i| i.kind == StackKind::Spell && has_split_second(cx, i.object))
}

/// Whether the spell `spell` has split second, by its characteristics (CR 702.61a, 613.1f): its printed card's, or the
/// one it copies, or one a static ability grants it - Samut, Tyrant of Naktamun's "Instant and sorcery spells you control
/// have split second" applies in layer 6.
fn has_split_second(cx: &Cx, spell: ObjectId) -> bool {
    cx.chars(spell).keywords.has(Keyword::SplitSecond)
}

fn activated(cx: &Cx, ability: AbilityRef) -> Option<&'static ActivatedDef> {
    match cx.cards().ability(ability) {
        AbilityDef::Activated(a) => Some(a),
        AbilityDef::Triggered(_) | AbilityDef::Static(_) => None,
    }
}

/// Whether the non-mana parts of a spell's cost could be paid (CR 601.2b, 118.3): a cast that cannot be is not offered.
/// The card is judged where it is, before it moves to the stack, so it is not among what its own cost could choose.
fn can_pay_other(cx: &Cx, source: ObjectId, controller: PlayerId, cost: &Cost) -> bool {
    can_pay_other_as(cx, source, controller, cost, Judged::Cast, Some(CostFor::Spell), None)
}

/// Whether the non-mana parts of an activated ability's cost could be paid (CR 602.2b, 118.3): an activation that cannot
/// be is not offered at all. Its source stays where it is as the cost is paid, so it may be what the cost chooses (a
/// permanent that beholds itself, CR 701.4a). `paid` is what the cost is paid for (`activating`): `None` for a cost paid as
/// a spell or ability resolves, judged as an activation's is.
fn can_pay_activation(cx: &Cx, source: ObjectId, controller: PlayerId, cost: &Cost, paid: Option<CostFor>, by: Option<Linked>) -> bool {
    can_pay_other_as(cx, source, controller, cost, Judged::Activation, paid, by)
}

/// What the cost of an ability that isn't a mana ability is paid for, as the static abilities that forbid some payments
/// read it (`CostFor`, `CardDef::cost_bans`): activating it (CR 602.2b). A special action is no activation (CR 116.2), and
/// its cost is paid for nothing they forbid. A mana ability's is `CostFor::ManaAbility`, which its own paths name.
fn activating(def: &ActivatedDef) -> Option<CostFor> {
    (!def.special_action).then_some(CostFor::Ability)
}

/// What a cost is judged for by `can_pay_other_as`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Judged {
    /// A spell's cost, its card about to move to the stack.
    Cast,
    /// An activated ability's, its source staying where it is.
    Activation,
    /// An activated mana ability's, judged while finding what mana abilities could tap (`tappable_for_mana`): a cost of
    /// tapped permanents counts every permanent that passes its filter, tapped or not, rather than asking that question
    /// again of the mana abilities - it can only offer more than can be paid, and an activation that finds too few once
    /// its mana is paid is taken back (`choose_permanents`).
    Nested,
}

/// `paid` is what the cost is paid for, which a static ability may forbid some payments for (`CardDef::cost_bans`).
fn can_pay_other_as(cx: &Cx, source: ObjectId, controller: PlayerId, cost: &Cost, judged: Judged, paid: Option<CostFor>, by: Option<Linked>) -> bool {
    let obj = cx.s().obj(source);
    // A creature's {T} ability needs it under its controller's control since their turn began, unless it has haste or the
    // ability may be activated as though it had (CR 302.6).
    if cost.tap
        && (obj.tapped
            || summoning_sick(cx.cards(), cx.s(), &cx.g.log, source) && !static_rules::activates_as_though_haste(cx, controller, source))
    {
        return false;
    }
    if let Some(loyalty) = cost.loyalty
        && loyalty < 0
        && loyalty != LOYALTY_MINUS_X
        && obj.counters(CounterKind::Loyalty) < -(loyalty as i16)
    {
        return false;
    }
    // CR 101.2, 118.3: a cost that puts counters on a permanent that can't have them put on it can't be paid (Solemnity's
    // ruling): a loyalty ability's plus, or "put a counter on this" as a cost.
    let banned = |kind| obj.zone.is(ZoneKind::Battlefield) && static_rules::counters_banned(cx, cx.cards(), source, obj.controller, kind);
    if cost.loyalty.is_some_and(|l| l > 0) && banned(CounterKind::Loyalty) {
        return false;
    }
    if cost.put_counters.is_some_and(|(kind, n)| n > 0 && banned(kind)) {
        return false;
    }
    // Energy is paid from the payer's counters (CR 107.14, 118.3): payable only with at least that many.
    if cx.s().player(controller).energy() < cost.energy || !can_pay_life_for(cx, controller, cost.life, paid) {
        return false;
    }
    // CR 101.2, 701.21a: a permanent a static ability says can't be sacrificed for this cost can't pay "sacrifice this".
    if cost.sacrifice_self && obj.zone.is(ZoneKind::Battlefield) && !may_sacrifice_for(cx, controller, source, paid) {
        return false;
    }
    if cost.return_self && !obj.zone.is(ZoneKind::Battlefield) {
        return false;
    }
    if let Some(variable) = cost.variable
        && !variable_payable(cx, source, controller, variable, paid)
    {
        return false;
    }
    if cost.unattach_self && !obj.attached_to.is_some_and(|h| cx.s().obj(h).zone.is(ZoneKind::Battlefield) && cx.s().obj(h).became.is_none()) {
        return false;
    }
    if let Some((kind, n)) = cost.remove_counters
        && obj.counters(kind) < n as i16
    {
        return false;
    }
    if cost.remove_any_counter && !obj.counter_array().iter().any(|&n| n > 0) {
        return false;
    }
    if !cost.discard.is_none() {
        let asker = Asker { source, controller, by, spell: false };
        let hand: Vec<ObjectId> = cx.s().zone(Zone::hand(controller)).iter().copied().filter(|&c| c != source).collect();
        if !discards_payable(cx, &asker, &hand, &cost.discard) {
            return false;
        }
    }
    if let Some((n, filter)) = cost.exile_from_hand {
        let asker = Asker { source, controller, by, spell: false };
        let hand = cx.s().zone(Zone::hand(controller)).iter().filter(|&&c| c != source && filter(cx, &asker, c)).count();
        if hand < n as usize {
            return false;
        }
    }
    if cost.exile_from_graveyard > 0 && graveyard_exiles(cx, controller, source, cost, by).len() < cost.exile_from_graveyard as usize {
        return false;
    }
    // Collect evidence N (CR 701.59a): payable only if the graveyard's cards, the source apart, add up to N.
    if cost.collect_evidence > 0 {
        let total: u32 = cx.s().zone(Zone::graveyard(controller)).iter().filter(|&&c| c != source).map(|&c| cx.mana_value(c)).sum();
        if total < cost.collect_evidence as u32 {
            return false;
        }
    }
    if let Some(p) = cost.permanents {
        let asker = Asker { source, controller, by, spell: false };
        let mut candidates = cx.permanents_of(controller).into_iter().filter(|&id| pays_permanents(cx, cost, &p, &asker, id, paid)).count();
        // Tapped permanents are judged once the mana abilities have been activated (CR 601.2g-h): an untapped one that
        // passes the filter and one of its controller's mana abilities could tap will do.
        if p.tapped && candidates < p.count as usize && judged == Judged::Nested {
            candidates = cx
                .permanents_of(controller)
                .into_iter()
                .filter(|&id| pays_permanents(cx, cost, &PermanentCost { tapped: false, ..p }, &asker, id, paid))
                .count();
        } else if p.tapped && candidates < p.count as usize {
            let tappable = tappable_for_mana(cx, controller);
            candidates += cx
                .permanents_of(controller)
                .into_iter()
                .filter(|&id| !cx.s().obj(id).tapped && (p.filter)(cx, &asker, id) && permanent_allowed(cx, cost, &p, controller, id, paid) && tappable.contains(&id))
                .count();
        }
        if candidates < p.count as usize {
            return false;
        }
    }
    let except = if judged == Judged::Cast { Some(source) } else { None };
    if let Some((n, _, _)) = cost.behold
        && behold_candidates(cx, except, controller, cost, source, by).len() < n as usize
    {
        return false;
    }
    // CR 701.68b: a player who controls no creature can't choose to blight.
    if cost.blight > 0 && blight_candidates(cx, controller).is_empty() {
        return false;
    }
    if !counters_among_payable(cx, source, controller, cost) {
        return false;
    }
    if let Some(t) = cost.tap_creatures {
        let (_, powers) = tap_candidates(cx, source, controller, &t);
        // Enough if the creatures with positive power are: a negative one only takes from the total (CR 107.1b).
        if powers.iter().map(|&p| p.max(0) as u32).sum::<u32>() < t.power as u32 {
            return false;
        }
    }
    if let Some(m) = cost.materials {
        let candidates = material_candidates(cx, source, controller, m, by);
        if candidates.len() < m.count as usize || !parts_can_be_matched(cx, source, controller, m, &candidates, by) {
            return false;
        }
    }
    true
}

/// Whether materials named one by one can each be a separate object of the candidates (the Throne of the Grim Captain
/// ruling): a matching of every part to a distinct candidate passing its filter, found by trying each in turn. True for
/// materials of one kind.
fn parts_can_be_matched(cx: &Cx, source: ObjectId, controller: PlayerId, m: &Materials, candidates: &[ObjectId], by: Option<Linked>) -> bool {
    m.parts.is_empty() || assign_parts(cx, &Asker { source, controller, by, spell: false }, m.parts, candidates, &mut Vec::new())
}

/// Whether `parts` can each take a distinct candidate passing its filter, none of `used` among them.
fn assign_parts(cx: &Cx, asker: &Asker, parts: &[MaterialPart], candidates: &[ObjectId], used: &mut Vec<ObjectId>) -> bool {
    let Some((part, rest)) = parts.split_first() else { return true };
    for &c in candidates {
        if !used.contains(&c) && (part.filter)(cx, asker, c) {
            used.push(c);
            let found = assign_parts(cx, asker, rest, candidates, used);
            used.pop();
            if found {
                return true;
            }
        }
    }
    false
}

/// What craft may exile as its materials (CR 702.167a): the permanents the payer controls, then the cards in their
/// graveyard, that pass the materials' filter, the source apart (it is exiled as itself, not as a material).
fn material_candidates(cx: &Cx, source: ObjectId, controller: PlayerId, m: &Materials, by: Option<Linked>) -> Vec<ObjectId> {
    let asker = Asker { source, controller, by, spell: false };
    let mut out = cx.permanents_of(controller);
    // Cards in the graveyard: a token put there (a Treasure sacrificed for the mana) is no card, and ceases to exist there
    // (CR 111.7, 704.5d).
    out.extend(cx.s().zone(Zone::graveyard(controller)).iter().copied().filter(|&id| !cx.s().obj(id).token));
    out.retain(|&id| id != source && (m.filter)(cx, &asker, id));
    out
}

/// Chooses craft's materials (CR 702.167a, 601.2h), always asked, even when the choice is forced: exactly `count` of
/// them, or `count` or more. Too few left to choose among (one gone since the ability was offered) is a cost that can't
/// be paid, and the activation is taken back.
fn choose_materials(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost, by: Option<Linked>) -> R<Vec<ObjectId>> {
    let Some(m) = cost.materials else { return Ok(Vec::new()) };
    // The source is exiled with them: one a mana ability moved while paying can't be (CR 601.2h).
    if !live(cx.s(), source) {
        return Err(Stop::Cancel);
    }
    let cards = material_candidates(cx, source, player, m, by);
    if cards.len() < m.count as usize {
        return Err(Stop::Cancel);
    }
    // Materials named one by one: each asked in turn among what is left that passes its filter and still leaves each later
    // one a separate object, so no choice offered is one that can't be finished. With none (one gone since the ability was
    // offered) the cost can't be paid, and the activation is taken back (CR 602.2b, 601.2h).
    if !m.parts.is_empty() {
        let asker = Asker { source, controller: player, by, spell: false };
        let mut chosen: Vec<ObjectId> = Vec::new();
        for (i, part) in m.parts.iter().enumerate() {
            let left: Vec<ObjectId> = cards
                .iter()
                .copied()
                .filter(|&c| {
                    !chosen.contains(&c) && (part.filter)(cx, &asker, c) && {
                        let mut used: Vec<ObjectId> = chosen.iter().copied().chain(std::iter::once(c)).collect();
                        assign_parts(cx, &asker, &m.parts[i + 1..], &cards, &mut used)
                    }
                })
                .collect();
            if left.is_empty() {
                return Err(Stop::Cancel);
            }
            let ask = RequestKind::Craft { source, cards: left, minimum: 1, maximum: 1, quality: part.quality };
            match cx.ask_core(player, ask, true)? {
                Answer::Cards(one) => chosen.extend(one),
                _ => return Err(Stop::Cancel),
            }
        }
        return Ok(chosen);
    }
    let maximum = if m.or_more { u32::try_from(cards.len()).unwrap_or(u32::MAX) } else { u32::from(m.count) };
    let ask = RequestKind::Craft { source, cards, minimum: u32::from(m.count), maximum, quality: m.quality };
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// Pays craft's exile (CR 702.167a): the source and the materials chosen, from the battlefield and the graveyard, all at
/// once (CR 601.2h). The source goes first, so the history holds it and then the materials, just before the activation:
/// what `cantrip_mechanics::craft` reads to find the exiled cards used to craft a permanent (CR 702.167c).
fn pay_materials(cx: &mut Cx, source: ObjectId, chosen: &[ObjectId]) {
    // Nothing that is no longer the object it was is moved (CR 400.7).
    let moves: Vec<ops::Moving> = std::iter::once(source)
        .chain(chosen.iter().copied())
        .filter(|&id| live(cx.s(), id))
        .map(|id| ops::Moving::to(id, Zone::exile(cx.s().obj(id).owner)))
        .collect();
    ops::move_together(cx, &moves, ZoneReason::Exile);
}

/// What a cost that beholds may choose among (CR 701.4a): the permanents its payer controls and the cards in their hand
/// that pass the filter, `except` apart - a spell's card, judged in the hand before it is cast (it is on the stack by the
/// time the cost is paid). An activated ability's source is no exception: a permanent may behold itself.
fn behold_candidates(cx: &Cx, except: Option<ObjectId>, controller: PlayerId, cost: &Cost, source: ObjectId, by: Option<Linked>) -> Vec<ObjectId> {
    let Some((_, filter, _)) = cost.behold else { return Vec::new() };
    let asker = Asker { source, controller, by, spell: false };
    let mut out = cx.permanents_of(controller);
    out.extend(cx.s().zone(Zone::hand(controller)).iter().copied());
    out.retain(|&id| Some(id) != except && filter(cx, &asker, id));
    out
}

/// Chooses what a cost beholds (CR 701.4a, 601.2h), always asked, even when the choice is forced.
fn choose_behold(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost) -> R<Vec<ObjectId>> {
    let Some((n, _, _)) = cost.behold else { return Ok(Vec::new()) };
    let eligible = behold_candidates(cx, None, player, cost, source, None);
    let ask = RequestKind::choose_cards(source, Purpose::Behold, eligible.clone(), eligible, u32::from(n), false, "");
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// Pays the behold part of a cost (CR 701.4a): each card chosen from the hand is revealed; a permanent is only chosen.
/// Returns the cards revealed.
fn pay_behold(cx: &mut Cx, player: PlayerId, chosen: &[ObjectId]) -> Vec<ObjectId> {
    let revealed: Vec<ObjectId> = chosen.iter().copied().filter(|&id| cx.s().obj(id).zone.is(ZoneKind::Hand)).collect();
    if !revealed.is_empty() {
        Game::reveal(cx, player, &revealed);
    }
    revealed
}

/// The cards the cost or announcement of the spell or ability at `at` on the stack revealed from a hand (behold, CR 701.4a;
/// splice, CR 702.47a): among the objects its cost chose (`StackItem::members`), as they stay revealed until it leaves the
/// stack (CR 701.20a, `ops::seen_by_all`).
fn add_revealed_to_item(cx: &mut Cx, at: usize, revealed: &[ObjectId]) {
    if revealed.is_empty() {
        return;
    }
    let mut with = cx.s().stack[at];
    let mut objects = cx.s().members(with.members).to_vec();
    objects.extend_from_slice(revealed);
    with.members = cx.st().members_push(&objects);
    cx.st().stack_set(at, with);
}

/// The creatures a cost that blights may put its counters on (CR 701.68a): not one that can't have -1/-1 counters put on it
/// (CR 101.2, 118.3: Melira, Sylvok Outcast; Solemnity's ruling).
fn blight_candidates(cx: &Cx, controller: PlayerId) -> Vec<ObjectId> {
    cx.permanents_of(controller)
        .into_iter()
        .filter(|&id| {
            cx.chars(id).is(CardType::Creature) && !static_rules::counters_banned(cx, cx.cards(), id, controller, CounterKind::MinusOne)
        })
        .collect()
}

/// Chooses the creature a cost blights (CR 701.68a, 601.2h), always asked, even when the choice is forced.
fn choose_blight(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost) -> R<Option<ObjectId>> {
    if cost.blight == 0 {
        return Ok(None);
    }
    let eligible = blight_candidates(cx, player);
    let ask = RequestKind::choose_cards(source, Purpose::Blight, eligible.clone(), eligible, 1, false, "");
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => cards.first().copied().map(Some).ok_or(Stop::Cancel),
        _ => Err(Stop::Cancel),
    }
}

/// Pays the blight part of a cost (CR 701.68a): the -1/-1 counters go on the creature chosen, put by the player paying
/// (CR 118.3, 122.6a).
fn pay_blight(cx: &mut Cx, player: PlayerId, cost: &Cost, chosen: Option<ObjectId>) {
    if let Some(creature) = chosen {
        counters::put(cx, creature, CounterKind::MinusOne, cost.blight as i16, player, counters::How::Cost);
    }
}

/// What a cost that removes counters from among permanents may take them from (CR 118.3, 122.1): each kind of counter the
/// cost names (every kind, or its one) on each permanent the payer controls that passes the filter, with how many there
/// are, in battlefield order.
fn counter_pools(cx: &Cx, source: ObjectId, controller: PlayerId, among: CountersAmong, by: Option<Linked>) -> Vec<CounterPool> {
    let asker = Asker { source, controller, by, spell: false };
    let mut pools = Vec::new();
    for id in cx.permanents_of(controller) {
        if !(among.filter)(cx, &asker, id) {
            continue;
        }
        let obj = cx.s().obj(id);
        for kind in CounterKind::ALL {
            if among.kind.is_some_and(|k| k != kind) {
                continue;
            }
            let count = obj.counters(kind);
            if count > 0 {
                pools.push(CounterPool { object: id, kind, count: count as u16 });
            }
        }
    }
    pools
}

/// Whether there are counters enough among the permanents a cost removes them from (CR 118.3).
fn counters_among_payable(cx: &Cx, source: ObjectId, controller: PlayerId, cost: &Cost) -> bool {
    cost.remove_counters_among
        .is_none_or(|among| counter_pools(cx, source, controller, among, None).iter().map(|p| p.count as u32).sum::<u32>() >= among.count as u32)
}

/// Chooses the counters a cost removes from among permanents (CR 601.2h), always asked, even when the choice is forced. Too
/// few left (some gone since the action was offered) is a cost that can't be paid, and the spell or ability is taken back.
fn choose_counters_among(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost) -> R<Vec<(ObjectId, CounterKind, u16)>> {
    let Some(among) = cost.remove_counters_among else { return Ok(Vec::new()) };
    let total = among.count;
    let pools = counter_pools(cx, source, player, among, None);
    if pools.iter().map(|p| p.count as u32).sum::<u32>() < total as u32 {
        return Err(Stop::Cancel);
    }
    match cx.ask_core(player, RequestKind::RemoveCounters { source, pools, total, up_to: false }, true)? {
        Answer::Counters(taken) => Ok(taken),
        _ => Err(Stop::Cancel),
    }
}

/// Pays a removal of counters from among permanents (CR 118.3): each chosen number of each kind comes off its permanent.
fn pay_counters_among(cx: &mut Cx, taken: &[(ObjectId, CounterKind, u16)]) {
    for &(object, kind, n) in taken {
        counters::remove(cx, object, kind, n.min(i16::MAX as u16) as i16);
    }
}

/// The creatures that could be tapped for crew or saddle (CR 702.122a, 702.171a): the controller's other untapped
/// creatures, summoning sick or not, each with what it counts as towards the total power (CR 702.122b).
fn tap_candidates(cx: &Cx, source: ObjectId, controller: PlayerId, t: &TapCreatures) -> (Vec<ObjectId>, Vec<i16>) {
    let mut cards = Vec::new();
    let mut powers = Vec::new();
    for id in cx.permanents_of(controller) {
        if id == source || cx.s().obj(id).tapped {
            continue;
        }
        let c = cx.chars(id);
        if !c.is(CardType::Creature) {
            continue;
        }
        // "As though its power were 2 greater": the power it counts with is 2 more than its power, a negative one too. A
        // power still negative counts as it is (CR 107.1b: "If a calculation or comparison needs to use a negative value,
        // it does so"), so tapping it takes from the total.
        let bonus = if t.greater_with.is_some_and(|k| c.has(k)) { 2 } else { 0 };
        cards.push(id);
        powers.push(c.power.unwrap_or(0).saturating_add(bonus));
    }
    (cards, powers)
}

/// Chooses the creatures a crew, saddle or teamwork cost taps (CR 601.2h), always asked, even when the choice is forced.
/// Chosen once the mana abilities have been activated (CR 601.2g), which may have tapped a creature the cost was to tap
/// (one that taps for mana): with too little power left untapped the cost can't be paid (CR 601.2h), and the spell or
/// ability is taken back (CR 733.1) - not asked, since no answer would do.
fn choose_tappers(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost) -> R<Vec<ObjectId>> {
    let Some(t) = cost.tap_creatures else { return Ok(Vec::new()) };
    let (cards, powers) = tap_candidates(cx, source, player, &t);
    // As `can_pay_other` judges it: a negative power only takes from the total (CR 107.1b).
    if powers.iter().map(|&p| p.max(0) as u32).sum::<u32>() < t.power as u32 {
        return Err(Stop::Cancel);
    }
    let ask = RequestKind::TapForPower { source, purpose: Purpose::TapToPay, cards, powers, minimum: t.power, up_to: false };
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// Pays the exert and tap-creatures parts of a cost.
fn pay_exert_and_tappers(cx: &mut Cx, source: ObjectId, cost: &Cost, tappers: &[ObjectId]) {
    if cost.exert {
        ops::exert(cx, source);
    }
    if cost.unattach_self {
        ops::unattach(cx, source);
    }
    for &id in tappers {
        ops::tap(cx, id);
    }
}

/// "Activate only once each turn" (CR 602.5b-c): whether the `index`th ability of this object has been activated this
/// turn, by anyone. Counted for the object (a new object is a new count) and for each instance of an identically
/// worded ability on its own.
fn limit_reached(cx: &Cx, id: ObjectId, chars: &Chars, index: usize, def: &ActivatedDef) -> bool {
    // "Activate only once" (CR 602.5b; exhaust, 702.177a; power-up, 702.193a): this permanent has activated this
    // ability before, in any turn. A permanent that left and came back is another object, with the ability unused.
    if def.once
        && let Some(ability) = chars.abilities().nth(index)
    {
        // A power-up ability may be activated an additional time for each permanent its controller controls that says so
        // (CR 702.193a, 602.5b): only asked once it has been activated, which is rare.
        let mut allowed = 1;
        let mut activations = cx
            .history()
            .select(EventQuery::kind(EventMask::ABILITY_ACTIVATED).object(id))
            .filter(|r| matches!(r.event, Event::AbilityActivated { ability: a, .. } if a == ability));
        if activations.next().is_some() {
            if def.power_up {
                allowed += static_rules::extra_power_up_activations(cx, cx.s().obj(id).controller);
            }
            if 1 + activations.count() >= allowed as usize {
                return true;
            }
        }
    }
    if granted_limit_reached(cx, id, chars, index) {
        return true;
    }
    if !def.once_each_turn {
        return false;
    }
    let obj = cx.s().obj(id);
    obj.activated_turn == cx.s().turn.number && obj.activated_mask & (1 << index) != 0
}

/// "You may activate each of those abilities only once each turn" (CR 602.5b, `CardDef::granted_once_each_turn`): the
/// `index`th ability of `id` is an instance of an ability its permanent's static ability gave it with that limit, and as
/// many of them have been activated this turn as it has limited instances, asked of the history. An instance it has with
/// no limit (printed, or given by something else) is the one activated first: identical abilities are interchangeable,
/// so the limit binds only once every instance it has is a limited one.
fn granted_limit_reached(cx: &Cx, id: ObjectId, chars: &Chars, index: usize) -> bool {
    let Some(limited) = cx.card(chars.card).granted_once_each_turn else { return false };
    let Some(ability) = chars.abilities().nth(index) else { return false };
    let limited = limited(cx, id, ability);
    if limited == 0 || chars.abilities().filter(|&a| a == ability).count() > limited {
        return false;
    }
    let used = cx
        .history()
        .select(EventQuery::kind(EventMask::ABILITY_ACTIVATED).object(id).this_turn())
        .filter(|r| matches!(r.event, Event::AbilityActivated { ability: a, .. } if a == ability))
        .count();
    used >= limited
}

/// Notes that the `index`th ability of an object was activated this turn.
fn note_activation(cx: &mut Cx, id: ObjectId, index: usize, def: &ActivatedDef) {
    if !def.once_each_turn {
        return;
    }
    let turn = cx.s().turn.number;
    cx.st().update_obj(id, |o| {
        if o.activated_turn != turn {
            o.activated_turn = turn;
            o.activated_mask = 0;
        }
        o.activated_mask |= 1 << index;
    });
}

/// Whether `hand` holds the cards `discards` asks, each card paying one instruction (CR 601.2h): a matching of the
/// instructions' cards to cards that pass their filters (Kuhn's augmenting paths), so a Dryad Arbor pays either a "land
/// card" or a "creature card" but not both.
fn discards_payable(cx: &Cx, asker: &Asker, hand: &[ObjectId], discards: &Discards) -> bool {
    let total = discards.count() as usize;
    if total > hand.len() {
        return false;
    }
    // One slot for each card an instruction takes, and the cards of the hand each may take.
    let slots: Vec<Vec<usize>> = discards
        .parts()
        .flat_map(|(n, filter)| {
            let fits: Vec<usize> = (0..hand.len()).filter(|&i| filter.is_none_or(|f| f(cx, asker, hand[i]))).collect();
            std::iter::repeat_n(fits, usize::from(n))
        })
        .collect();
    fn augment(slot: usize, slots: &[Vec<usize>], taken_by: &mut [Option<usize>], seen: &mut [bool]) -> bool {
        for &card in &slots[slot] {
            if seen[card] {
                continue;
            }
            seen[card] = true;
            if taken_by[card].is_none_or(|other| augment(other, slots, taken_by, seen)) {
                taken_by[card] = Some(slot);
                return true;
            }
        }
        false
    }
    let mut taken_by = vec![None; hand.len()];
    (0..slots.len()).all(|slot| augment(slot, &slots, &mut taken_by, &mut vec![false; hand.len()]))
}

/// The cards a cost discards: all of them, and those among them that pay an instruction naming a characteristic of the card
/// ("discard a land card", CR 701.9c).
#[derive(Default)]
struct Discarding {
    cards: Vec<ObjectId>,
    paying: Vec<ObjectId>,
}

/// Chooses the cards a cost discards (CR 601.2h), always asked, even when the choice is forced: one question for each of
/// its instructions (`Discards::parts`), those that name a characteristic first, each from the cards the ones before left
/// (retrace's land card, then escalate's card). A choice that leaves a later instruction too few cards can't pay the cost,
/// and is taken back (CR 601.2h, 733.1).
fn choose_discards(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost, by: Option<Linked>) -> R<Discarding> {
    // "Discard your hand" (CR 701.9) chooses nothing: every card in it, the source (on the stack by now) not among them.
    if cost.discard_hand {
        return Ok(Discarding { cards: cx.s().zone(Zone::hand(player)).iter().copied().filter(|&c| c != source).collect(), paying: Vec::new() });
    }
    let asker = Asker { source, controller: player, by, spell: false };
    let hand: Vec<ObjectId> = cx.s().zone(Zone::hand(player)).to_vec();
    let mut out = Discarding::default();
    for (n, filter) in cost.discard.parts() {
        let eligible: Vec<ObjectId> =
            hand.iter().copied().filter(|&c| c != source && !out.cards.contains(&c) && filter.is_none_or(|f| f(cx, &asker, c))).collect();
        if eligible.len() < usize::from(n) {
            return Err(Stop::Cancel);
        }
        let ask = RequestKind::choose_cards(source, Purpose::Discard, hand.clone(), eligible, u32::from(n), false, "");
        let Answer::Cards(cards) = cx.ask_core(player, ask, true)? else { return Err(Stop::Cancel) };
        if filter.is_some() {
            out.paying.extend_from_slice(&cards);
        }
        out.cards.extend(cards);
    }
    Ok(out)
}

/// Chooses the cards a cost exiles from the hand (CR 601.2h, 118.9), always asked, even when the choice is forced: the
/// hand is shown, and the cards that pass the filter are the ones that may be chosen.
fn choose_hand_exiles(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost, by: Option<Linked>) -> R<Vec<ObjectId>> {
    let Some((n, filter)) = cost.exile_from_hand else { return Ok(Vec::new()) };
    let asker = Asker { source, controller: player, by, spell: false };
    let hand: Vec<ObjectId> = cx.s().zone(Zone::hand(player)).to_vec();
    let eligible: Vec<ObjectId> = hand.iter().copied().filter(|&c| c != source && filter(cx, &asker, c)).collect();
    let ask = RequestKind::choose_cards(source, Purpose::ExileToPay, hand, eligible, u32::from(n), false, "");
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// Collect evidence N (CR 701.59a): the cards, with total mana value N or more, that a cost exiles from the graveyard;
/// always asked.
fn choose_evidence(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost) -> R<Vec<ObjectId>> {
    if cost.collect_evidence == 0 {
        return Ok(Vec::new());
    }
    let cards: Vec<ObjectId> = cx.s().zone(Zone::graveyard(player)).iter().copied().filter(|&c| c != source).collect();
    let values: Vec<u8> = cards.iter().map(|&c| cx.mana_value(c).min(255) as u8).collect();
    let ask = RequestKind::CollectEvidence { source, cards, values, minimum: cost.collect_evidence };
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// The cards of `player`'s graveyard a cost may exile: any but the source, or those of the quality it names (CR 601.2h).
fn graveyard_exiles(cx: &Cx, player: PlayerId, source: ObjectId, cost: &Cost, by: Option<Linked>) -> Vec<ObjectId> {
    let asker = Asker { source, controller: player, by, spell: false };
    cx.s()
        .zone(Zone::graveyard(player))
        .iter()
        .copied()
        .filter(|&c| c != source && cost.exile_from_graveyard_of.is_none_or(|f| f(cx, &asker, c)))
        .collect()
}

/// Chooses the cards a cost exiles from the graveyard (CR 601.2h), always asked, even when the choice is forced.
fn choose_exiles(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost) -> R<Vec<ObjectId>> {
    if cost.exile_from_graveyard == 0 {
        return choose_evidence(cx, player, source, cost);
    }
    let eligible = graveyard_exiles(cx, player, source, cost, None);
    let ask = RequestKind::choose_cards(source, Purpose::ExileToPay, cx.s().zone(Zone::graveyard(player)).iter().copied().filter(|&c| c != source).collect(), eligible, u32::from(cost.exile_from_graveyard), false, "");
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// Pays the exile part of a cost: the chosen cards, together.
fn pay_exiles(cx: &mut Cx, chosen: &[ObjectId]) {
    if chosen.is_empty() {
        return;
    }
    let moves: Vec<ops::Moving> = chosen.iter().map(|&id| ops::Moving::to(id, Zone::exile(cx.s().obj(id).owner))).collect();
    ops::move_together(cx, &moves, ZoneReason::Exile);
}

/// Whether `id` may pay a permanents cost now: it passes the cost's filter, and is tapped if the cost asks for tapped
/// permanents ("return a tapped creature you control", CR 702.188a). When the cost taps permanents and also has {T}
/// ("{T}, Tap an untapped creature you control"), the source is not one of them: it is tapped by the {T} part of the same
/// cost, and one permanent can't be tapped to pay both (CR 118.3, 602.2b). When it asks for tapped permanents and has {T},
/// the source is one: the parts of a cost are paid in any order (CR 601.2h by way of 602.2b), so {T} may tap it first
/// ("{T}, Return a tapped creature you control" may return itself), as `activate` and `activate_mana` pay it.
/// `paid` is what the cost is paid for (`permanent_allowed`).
fn pays_permanents(cx: &Cx, cost: &Cost, p: &PermanentCost, asker: &Asker, id: ObjectId, paid: Option<CostFor>) -> bool {
    let tapped = cx.s().obj(id).tapped || cost.tap && id == asker.source;
    !(cost.tap && cost.tap_permanents && id == asker.source)
        && (!p.tapped || tapped)
        && (p.filter)(cx, asker, id)
        && permanent_allowed(cx, cost, p, asker.controller, id, paid)
}

/// Whether `player` may pay a permanents cost paid for `paid` with `id` as far as the static abilities that forbid some
/// payments go (CR 101.2, `CardDef::cost_bans`): a sacrifice one forbids can't be made (CR 701.21a), and tapping, returning
/// or exiling a permanent is no sacrifice.
fn permanent_allowed(cx: &Cx, cost: &Cost, p: &PermanentCost, player: PlayerId, id: ObjectId, paid: Option<CostFor>) -> bool {
    cost.tap_permanents || p.action != PermanentAction::Sacrifice || may_sacrifice_for(cx, player, id, paid)
}

/// The untapped permanents `player`'s mana abilities could tap as they are activated now (CR 601.2g, 605.3a): each one
/// whose own mana ability has {T} in its cost, and each one a mana ability's "tap an untapped [permanent] you control"
/// could tap.
fn tappable_for_mana(cx: &Cx, player: PlayerId) -> Vec<ObjectId> {
    let mut options = Vec::new();
    mana_ability_options_as(cx, player, ManaWindow::Payment, &mut options, Judged::Nested);
    let mut out = Vec::new();
    for o in &options {
        let Some(source) = o.source else { continue };
        if o.index >= PLAYER_MANA_ABILITY {
            continue;
        }
        let Some(def) = cx.chars(source).abilities().nth(o.index as usize).and_then(|a| activated(cx, a)) else { continue };
        if def.cost.tap && !cx.s().obj(source).tapped {
            out.push(source);
        }
        if def.cost.tap_permanents {
            out.extend(o.target_candidates.iter().filter_map(|t| t.object()));
        }
    }
    out
}

/// Whether a cost's permanents are chosen once its mana is paid (CR 601.2h): those that must be tapped, which a mana
/// ability activated while paying (CR 601.2g) may have tapped. The others are chosen with the rest of the cost, before.
fn permanents_after_mana(cost: &Cost) -> bool {
    cost.permanents.is_some_and(|p| p.tapped)
}

/// Chooses the permanents a cost names (CR 601.2h), always asked, even when the choice is forced. Too few to choose
/// among (tapped ones, judged once the mana is paid) is a cost that can't be paid: the announcement is taken back. `paid` is
/// what the cost is paid for (`permanent_allowed`).
fn choose_permanents(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost, paid: Option<CostFor>, by: Option<Linked>) -> R<Vec<ObjectId>> {
    let Some(p) = cost.permanents else { return Ok(Vec::new()) };
    let asker = Asker { source, controller: player, by, spell: false };
    let eligible: Vec<ObjectId> = cx.permanents_of(player).into_iter().filter(|&id| pays_permanents(cx, cost, &p, &asker, id, paid)).collect();
    if eligible.len() < p.count as usize {
        return Err(Stop::Cancel);
    }
    let purpose = match p.action {
        _ if cost.tap_permanents => Purpose::TapToPay,
        PermanentAction::ReturnToHand => Purpose::ReturnToHand,
        PermanentAction::Exile => Purpose::ExileToPay,
        PermanentAction::Sacrifice => Purpose::SacrificeCost,
    };
    // A cost of tapped permanents says so, and names creatures when those are all it can take (web-slinging's). An exile
    // of permanents says they are permanents (or creatures), not cards from a hand or graveyard as other exiles to pay are.
    let creatures = eligible.iter().all(|&id| cx.chars(id).is(CardType::Creature));
    let kind = match (p.tapped, p.action) {
        (true, _) if creatures => "tapped creature",
        (true, _) => "tapped permanent",
        (false, PermanentAction::Exile) if creatures => "creature",
        (false, PermanentAction::Exile) => "permanent",
        (false, _) => "",
    };
    let ask = RequestKind::choose_cards(source, purpose, eligible.clone(), eligible, p.count, false, kind);
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// Pays the discard part of a cost: the chosen cards, together (CR 701.9). Those an instruction names by a characteristic
/// ("Discard a creature card") can't be put into a hidden zone unrevealed (CR 701.9c, `ops::discard_paying`).
fn pay_discards(cx: &mut Cx, chosen: &Discarding) {
    if chosen.cards.is_empty() {
        return;
    }
    ops::discard_paying(cx, &chosen.cards, &chosen.paying);
}

/// Whether `player` can still pay this much energy (CR 107.14) and life (CR 119.4) once the mana of the cost is paid: the
/// mana abilities activated while paying it (CR 601.2g) may have spent energy or life the rest of the cost needs. `paid` is
/// what the cost is paid for (`payable_life_for`).
fn rest_payable(cx: &Cx, player: PlayerId, energy: u32, life: u32, paid: Option<CostFor>) -> bool {
    u32::from(cx.s().player(player).energy()) >= energy && can_pay_life_for(cx, player, life, paid)
}

/// Pays the life and counters parts of a cost.
fn pay_life_and_counters(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost) {
    if cost.energy > 0 {
        ops::change_energy(cx, player, -i32::from(cost.energy), player);
    }
    if cost.life > 0 {
        ops::change_life(cx, player, -life_paid(cost.life));
    }
    if let Some((kind, n)) = cost.remove_counters {
        counters::remove(cx, source, kind, n as i16);
    }
    // Counters a cost puts on its source are put by the player paying it (CR 118.3, 122.6a): what replaces counters being
    // put applies.
    if let Some((kind, n)) = cost.put_counters {
        counters::put(cx, source, kind, n as i16, player, counters::How::Cost); // a counter count is an i16 everywhere in the engine
    }
}

/// The kind of counter a cost that removes a counter of any kind takes from the source (CR 118.3): the only kind it has,
/// or the payer's choice among several, made as the cost is chosen.
fn choose_counter_kind(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost) -> R<Option<CounterKind>> {
    if !cost.remove_any_counter {
        return Ok(None);
    }
    let kinds: Vec<CounterKind> =
        CounterKind::ALL.into_iter().filter(|&k| cx.s().obj(source).counters(k) > 0).collect();
    if kinds.len() <= 1 {
        return Ok(kinds.first().copied());
    }
    let ask = RequestKind::ChooseModes {
        source,
        modes: kinds.iter().map(|&k| Label::Counter(k)).collect(),
        eligible: (0..kinds.len() as u8).collect(),
        count: 1,
        at_most: 0,
        affordable: Vec::new(),
        additional_cost: true,
        repeat: false,
    };
    match cx.ask_core(player, ask, true)? {
        Answer::Modes(chosen) => Ok(chosen.first().map(|&i| kinds[i as usize])),
        _ => Err(Stop::Cancel),
    }
}

/// The other cards of `player`'s hand that could pay "exile any number of [white] cards from your hand" (`ExileFromHandForLess`).
fn hand_exiles_for_less(cx: &Cx, source: ObjectId, player: PlayerId, cost: ExileFromHandForLess, by: Option<Linked>) -> Vec<ObjectId> {
    let asker = Asker { source, controller: player, by, spell: false };
    cx.s().zone(Zone::hand(player)).iter().copied().filter(|&c| c != source && (cost.filter)(cx, &asker, c)).collect()
}

/// "You may exile any number of [white] cards from your hand" (`ExileFromHandForLess`), as the intention to pay it is
/// announced (CR 601.2b): which of the cards that could pay will, any number of them or none, so the total cost is
/// determined knowing how many (CR 601.2f). Nothing moves yet. Not asked when no card could pay.
fn announce_hand_exiles(cx: &mut Cx, player: PlayerId, spell: ObjectId, could: &[ObjectId]) -> R<Vec<ObjectId>> {
    if could.is_empty() {
        return Ok(Vec::new());
    }
    let hand = cx.s().zone(Zone::hand(player)).to_vec();
    let ask = RequestKind::choose_cards(spell, Purpose::ExileToPay, hand, could.to_vec(), count_u32(could.len()), true, "");
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// The cards `announce_hand_exiles` chose, as the cost is paid (CR 601.2h): the same cards while the cards that could pay are
/// the ones that could as they were chosen (`could`). A mana ability activated while paying (CR 601.2g) may have taken one
/// from the hand, put another there or changed one's color; then as many as were announced, which the total cost was locked
/// in with (CR 601.2f), are chosen again from the cards that could pay now, since which ones pay is chosen as it is paid.
/// With too few, the cost can't be paid, and the cast is taken back (CR 601.2h, 733.1).
fn confirm_hand_exiles(cx: &mut Cx, player: PlayerId, spell: ObjectId, cost: ExileFromHandForLess, chosen: &[ObjectId], could: &[ObjectId]) -> R<Vec<ObjectId>> {
    if chosen.is_empty() {
        return Ok(Vec::new());
    }
    let now = hand_exiles_for_less(cx, spell, player, cost, None);
    if now.as_slice() == could {
        return Ok(chosen.to_vec());
    }
    if now.len() < chosen.len() {
        return Err(Stop::Cancel);
    }
    let hand = cx.s().zone(Zone::hand(player)).to_vec();
    let ask = RequestKind::choose_cards(spell, Purpose::ExileToPay, hand, now, count_u32(chosen.len()), false, "");
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// The permanents a variable sacrifice may take, or the other cards in the graveyard an exile may take (CR 601.2h). `paid`
/// is what the cost is paid for: a sacrifice a static ability forbids for it is no candidate (CR 101.2, `may_sacrifice_for`).
fn variable_candidates(cx: &Cx, source: ObjectId, player: PlayerId, variable: VariableCost, paid: Option<CostFor>, by: Option<Linked>) -> Vec<ObjectId> {
    match variable {
        VariableCost::SacrificeX(filter)
        | VariableCost::SacrificeOneOrMore(filter)
        | VariableCost::SacrificeAnyNumber { filter, .. }
        | VariableCost::SacrificeWithPowerX(filter) => {
            let asker = Asker { source, controller: player, by, spell: false };
            cx.permanents_of(player).into_iter().filter(|&id| filter(cx, &asker, id) && may_sacrifice_for(cx, player, id, paid)).collect()
        }
        VariableCost::ExileFromGraveyardWithTypes(_) => {
            cx.s().zone(Zone::graveyard(player)).iter().copied().filter(|&c| c != source).collect()
        }
        // Counters on the source, not objects.
        VariableCost::RemoveAnyNumberOfCounters(_) => Vec::new(),
        VariableCost::DiscardOneOrMore(filter) => {
            let asker = Asker { source, controller: player, by, spell: false };
            cx.s().zone(Zone::hand(player)).iter().copied().filter(|&c| c != source && filter(cx, &asker, c)).collect()
        }
    }
}

/// Whether a permanent's power is `x` or greater ("a creature with power X or greater", CR 702.153a): one with no power
/// has none to be.
fn power_at_least(cx: &Cx, id: ObjectId, x: u32) -> bool {
    cx.chars(id).power.is_some_and(|p| i64::from(p) >= i64::from(x))
}

/// How many card types appear among these cards (CR 205.2a).
fn types_among(cx: &Cx, cards: &[ObjectId]) -> usize {
    let mut seen = TypeSet::NONE;
    for &c in cards {
        for t in cx.chars(c).types.iter() {
            seen = seen.with(t);
        }
    }
    seen.iter().count()
}

/// Whether a variable cost could be paid at all: X may be zero, "one or more" needs one, and the exile needs enough types.
fn variable_payable(cx: &Cx, source: ObjectId, player: PlayerId, variable: VariableCost, paid: Option<CostFor>) -> bool {
    match variable {
        VariableCost::SacrificeX(_) | VariableCost::SacrificeAnyNumber { .. } | VariableCost::RemoveAnyNumberOfCounters(_) => true,
        VariableCost::SacrificeOneOrMore(_) | VariableCost::DiscardOneOrMore(_) => !variable_candidates(cx, source, player, variable, paid, None).is_empty(),
        // Its power is read as the cost is paid (CR 601.2h), after the mana abilities of CR 601.2g, which may change it: any
        // permanent that passes the filter may yet pay.
        VariableCost::SacrificeWithPowerX(_) => !variable_candidates(cx, source, player, variable, paid, None).is_empty(),
        VariableCost::ExileFromGraveyardWithTypes(n) => {
            types_among(cx, &variable_candidates(cx, source, player, variable, paid, None)) >= n.max(1) as usize
        }
    }
}

fn ask_cards(cx: &mut Cx, player: PlayerId, source: ObjectId, purpose: Purpose, cards: Vec<ObjectId>, count: usize, up_to: bool) -> R<Vec<ObjectId>> {
    let ask = RequestKind::choose_cards(source, purpose, cards.clone(), cards, u32::try_from(count).unwrap_or(u32::MAX), up_to, "");
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards),
        _ => Err(Stop::Cancel),
    }
}

/// What a variable cost takes, chosen as the cost is chosen (CR 601.2h): the permanents to sacrifice, the cards to exile,
/// or the cards to discard. `x` is the X announced for `SacrificeX`; `paid` is what the cost is paid for
/// (`variable_candidates`).
fn choose_variable(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost, x: u32, paid: Option<CostFor>) -> R<(Vec<ObjectId>, Vec<ObjectId>, Discarding)> {
    let Some(variable) = cost.variable else { return Ok((Vec::new(), Vec::new(), Discarding::default())) };
    let candidates = variable_candidates(cx, source, player, variable, paid, None);
    if let VariableCost::DiscardOneOrMore(_) = variable {
        // The hand shown, the cards that pass the filter eligible, as many as the payer likes but one at least: none chosen
        // gives the payment up. Each card pays an instruction that names a characteristic of it (CR 701.9c).
        if candidates.is_empty() {
            return Err(Stop::Cancel);
        }
        let hand = cx.s().zone(Zone::hand(player)).to_vec();
        let n = count_u32(candidates.len());
        let ask = RequestKind::choose_cards(source, Purpose::Discard, hand, candidates, n, true, "");
        let Answer::Cards(cards) = cx.ask_core(player, ask, true)? else { return Err(Stop::Cancel) };
        if cards.is_empty() {
            return Err(Stop::Cancel);
        }
        return Ok((Vec::new(), Vec::new(), Discarding { paying: cards.clone(), cards }));
    }
    let (sacrifices, exiles) = choose_variable_permanents(cx, player, source, variable, candidates, x)?;
    Ok((sacrifices, exiles, Discarding::default()))
}

/// `choose_variable`'s sacrifices and exiles.
fn choose_variable_permanents(cx: &mut Cx, player: PlayerId, source: ObjectId, variable: VariableCost, candidates: Vec<ObjectId>, x: u32) -> R<(Vec<ObjectId>, Vec<ObjectId>)> {
    match variable {
        VariableCost::DiscardOneOrMore(_) | VariableCost::RemoveAnyNumberOfCounters(_) => Ok((Vec::new(), Vec::new())),
        VariableCost::SacrificeX(_) | VariableCost::SacrificeAnyNumber { .. } => {
            if x == 0 {
                return Ok((Vec::new(), Vec::new()));
            }
            // A creature that would have paid has gone since X was announced: the announcement is taken back.
            if candidates.len() < x as usize {
                return Err(Stop::Cancel);
            }
            Ok((ask_cards(cx, player, source, Purpose::SacrificeCost, candidates, x as usize, false)?, Vec::new()))
        }
        VariableCost::SacrificeWithPowerX(_) => {
            // Its power is read as the cost is paid (CR 601.2h): one that has shrunk below X since X was announced does not
            // pay, and with none that does the announcement is taken back.
            let eligible: Vec<ObjectId> = candidates.into_iter().filter(|&id| power_at_least(cx, id, x)).collect();
            if eligible.is_empty() {
                return Err(Stop::Cancel);
            }
            Ok((ask_cards(cx, player, source, Purpose::SacrificeCost, eligible, 1, false)?, Vec::new()))
        }
        VariableCost::SacrificeOneOrMore(_) => {
            if candidates.is_empty() {
                return Err(Stop::Cancel);
            }
            // As the C# engine asked it: one mandatory permanent (unless it is the only one), then up to all the rest.
            let first = if candidates.len() > 1 {
                match ask_cards(cx, player, source, Purpose::SacrificeCost, candidates.clone(), 1, false)?.first() {
                    Some(&f) => f,
                    None => return Err(Stop::Cancel),
                }
            } else {
                candidates[0]
            };
            let mut chosen = vec![first];
            let rest: Vec<ObjectId> = candidates.into_iter().filter(|&c| c != first).collect();
            if !rest.is_empty() {
                let n = rest.len();
                let more = ask_cards(cx, player, source, Purpose::SacrificeCost, rest.clone(), n, true)?;
                chosen.extend(rest.into_iter().filter(|c| more.contains(c)));
            }
            Ok((chosen, Vec::new()))
        }
        VariableCost::ExileFromGraveyardWithTypes(n) => {
            if types_among(cx, &candidates) < n.max(1) as usize {
                return Err(Stop::Cancel);
            }
            let count = candidates.len();
            let chosen = ask_cards(cx, player, source, Purpose::ExileToPay, candidates, count, true)?;
            // A choice covering too few card types does not pay: the announcement is rewound.
            if types_among(cx, &chosen) < n.max(1) as usize {
                return Err(Stop::Cancel);
            }
            Ok((Vec::new(), chosen))
        }
    }
}

/// Pays the permanents part of a cost, once the rest is paid. `ability` is the activated ability whose cost it is: the
/// permanents its cost exiles are "exiled with" its source, linked to the other abilities of its text (CR 406.6, 607.2a). A
/// spell's are not: that rule is of abilities.
fn pay_permanents(cx: &mut Cx, player: PlayerId, cost: &Cost, chosen: &[ObjectId], ability: Option<Linked>) {
    if cost.tap_permanents {
        for &id in chosen {
            ops::tap(cx, id);
        }
        return;
    }
    match cost.permanents.map(|p| p.action) {
        Some(PermanentAction::ReturnToHand) => {
            ops::return_to_hand(cx, chosen);
        }
        Some(PermanentAction::Sacrifice) => {
            ops::sacrifice(cx, player, chosen);
        }
        // Exiled together (CR 118.3), each into its owner's exile; one that is no longer the permanent chosen stays where
        // it is (CR 400.7), as a sacrifice's does.
        Some(PermanentAction::Exile) => match ability {
            Some(source) => {
                let still: Vec<ObjectId> =
                    chosen.iter().copied().filter(|&id| live(cx.s(), id) && cx.s().obj(id).zone.is(ZoneKind::Battlefield)).collect();
                ops::exile_with(cx, &still, source, false);
            }
            None => {
                ops::exile_all(cx, chosen);
            }
        },
        None => {}
    }
}

fn mana_ability_options(cx: &Cx, player: PlayerId, window: ManaWindow, options: &mut Vec<ActionOption>) {
    mana_ability_options_as(cx, player, window, options, Judged::Activation);
}

/// The mana abilities `player` could activate now, their costs judged as `judged` says (`Judged::Nested` for the
/// question `tappable_for_mana` asks).
fn mana_ability_options_as(cx: &Cx, player: PlayerId, window: ManaWindow, options: &mut Vec<ActionOption>, judged: Judged) {
    let forbidders = static_rules::activation_forbidders(cx);
    for &id in cx.s().battlefield() {
        if cx.s().obj(id).controller != player {
            continue;
        }
        let chars = cx.chars(id);
        let givers = std::cell::OnceCell::new();
        let given = layers::acquisition_possible(cx.cards(), cx.s());
        for (index, ability) in chars.abilities().enumerate() {
            // Asked as the ability it is (CR 607.5): one an effect gave the source reads only its own links.
            let by = if given { cx.link_nth(id, &chars, index) } else { None };
            let Some(def) = activated(cx, ability) else { continue };
            if !def.is_mana_ability()
                || !window.allows(def)
                || def.from != ZoneKind::Battlefield
                || limit_reached(cx, id, &chars, index, def)
                || !can_pay_other_as(cx, id, player, &def.cost, judged, Some(CostFor::ManaAbility), by)
                || static_rules::activation_barred(cx, &forbidders, id, def)
            {
                continue;
            }
            // "Activate only if ..." (CR 602.5b), as Fanatic of Rhonas's ferocious mana.
            if let Some(condition) = def.condition
                && !condition(cx, &Asker { source: id, controller: player, by, spell: false })
            {
                continue;
            }
            // A mana ability with a mana cost of its own (Sunken Ruins' {U/B}) is not offered when that cost cannot be
            // paid by the other sources, its own {T} not among them (CR 605.3a, 602.2b, 609.4b) - unless it is judged
            // for what mana abilities could tap (`Judged::Nested`), where asking what the player can pay would ask it again.
            if def.cost.mana.x == 0 && def.cost.mana.mana_value() > 0 && judged != Judged::Nested {
                let a = available_without(cx, player, def.cost.tap.then_some(id), ManaWindow::Payment);
                let mut cost = def.cost.mana;
                if static_rules::spend_any_color_for(cx, player, id, SpendingFor::Activating) {
                    cost = cost.paid_with_any_color();
                }
                if !a.covers_for(&cost, &ability_payment(cx, id, chars.types, chars.subtypes)) {
                    continue;
                }
            }
            let mut option = ActionOption::on(ActionKind::ActivateAbility, id, index as u8);
            option.granted_by = granted_by(cx, id, Some(&chars), index, ability, &givers);
            option.restriction = def.restricted.map(|r| restriction_from(cx, id, r, by));
            // The permanents a cost takes come with the activation, as its option's candidates - but tapped ones are chosen
            // as the activation is made, once its mana is paid (`activate_mana`, CR 605.3a, 601.2g-h).
            if let Some(p) = def.cost.permanents.filter(|p| !p.tapped) {
                let asker = Asker { source: id, controller: player, by, spell: false };
                option.target_candidates = cx
                    .permanents_of(player)
                    .into_iter()
                    .filter(|&o| pays_permanents(cx, &def.cost, &p, &asker, o, Some(CostFor::ManaAbility)))
                    .map(Target::Object)
                    .collect();
                option.targets_required = p.count;
            }
            if def.colors_from_sacrifice {
                // The types any land it could sacrifice could produce; the activation then names one of that land's.
                let (mut colors, mut colorless) = (ColorSet::NONE, false);
                // Tapped permanents are chosen once the mana is paid: any that passes the filter could be one by then.
                let later: Vec<Target> = match def.cost.permanents.filter(|p| p.tapped) {
                    Some(p) => {
                        let asker = Asker { source: id, controller: player, by, spell: false };
                        let untapped = PermanentCost { tapped: false, ..p };
                        cx.permanents_of(player)
                            .into_iter()
                            .filter(|&o| pays_permanents(cx, &def.cost, &untapped, &asker, o, Some(CostFor::ManaAbility)))
                            .map(Target::Object)
                            .collect()
                    }
                    None => Vec::new(),
                };
                for t in option.target_candidates.iter().chain(&later) {
                    if let Some(land) = t.object() {
                        let (c, n) = producible_types(cx, land);
                        colors = ColorSet(colors.0 | c.0);
                        colorless |= n;
                        option.names_nothing |= c.is_empty() && !n;
                    }
                }
                option.color_choices = colors;
                if colorless {
                    option.mana = Some(Mana::C);
                }
            } else if def.any_color {
                option.color_choices = match def.colors_from {
                    Some(from) => from(cx, &Asker { source: id, controller: player, by, spell: false }),
                    None if def.colors.is_empty() => ColorSet(0b11111),
                    None => def.colors,
                };
                // With no color to choose it is still offered, and adds no mana (CR 106.5).
            }
            options.push(option);
        }
    }
    // Mana abilities the player has of their own, from a resolved spell or ability (CR 605.1a, 611.2a): offered whenever they
    // could activate a mana ability. Numbered in the order of the effects that gave them.
    for (n, effect) in cx.s().effects.iter().filter(|e| matches!(e.kind, EffectKind::PlayerManaAbility(_)) && e.controller == player).enumerate() {
        let EffectKind::PlayerManaAbility(ability) = effect.kind else { continue };
        if let Some(def) = activated(cx, ability)
            && window.allows(def)
            && can_pay_other_as(cx, effect.source, player, &def.cost, judged, Some(CostFor::ManaAbility), None)
        {
            options.push(ActionOption::on(ActionKind::ActivateAbility, effect.source, PLAYER_MANA_ABILITY + n as u8));
        }
    }
    // Mana abilities that function from the hand (CR 113.6, 605): Elvish Spirit Guide's exile-this-card.
    for &id in cx.s().zone(Zone::hand(player)) {
        let def = cx.card(cx.s().obj(id).card);
        for (index, ability) in def.abilities.iter().enumerate() {
            let AbilityDef::Activated(a) = ability else { continue };
            if !a.is_mana_ability()
                || !window.allows(a)
                || a.from != ZoneKind::Hand
                || !can_pay_other_as(cx, id, player, &a.cost, judged, Some(CostFor::ManaAbility), None)
                || static_rules::activation_barred(cx, &forbidders, id, a)
            {
                continue;
            }
            if let Some(condition) = a.condition
                && !condition(cx, &Asker { source: id, controller: player, by: None, spell: false })
            {
                continue;
            }
            options.push(ActionOption {
                restriction: a.restricted.map(|r| restriction_from(cx, id, r, None)),
                ..ActionOption::on(ActionKind::ActivateAbility, id, index as u8)
            });
        }
    }
}

/// The candidates for one target slot (CR 601.2c), in a stable order: players in turn order, then objects on the
/// battlefield, the stack and in graveyards.
/// No X has been announced (`Query::announced_x`): what some announcement could target.
pub fn target_candidates(cx: &Cx, asker: &Asker, spec: &TargetSpec) -> Vec<Target> {
    target_candidates_given(cx, asker, spec, &[], Target::None, None, None)
}

/// The candidates for a slot of a spell as if its gift were promised (`Query::gift_assumed`), without promising it.
fn target_candidates_if_promised(cx: &Cx, asker: &Asker, spec: &TargetSpec) -> Vec<Target> {
    candidates(cx, asker, spec, &[], Target::None, None, None, true)
}

/// The candidates for a slot after the slots before it chose `earlier` (CR 601.2c); `slot_player` is the opponent the slot
/// is for when it is one of a `TargetSpec::EACH_OPPONENT`; `x` the X announced for the spell or ability they are for
/// (`resolve::announced_x`), none before it is.
pub fn target_candidates_given(
    cx: &Cx,
    asker: &Asker,
    spec: &TargetSpec,
    earlier: &[Target],
    subject: Target,
    slot_player: Option<PlayerId>,
    x: Option<u16>,
) -> Vec<Target> {
    candidates(cx, asker, spec, earlier, subject, slot_player, x, false)
}

#[allow(clippy::too_many_arguments)]
fn candidates(
    q: &dyn Query,
    asker: &Asker,
    spec: &TargetSpec,
    earlier: &[Target],
    subject: Target,
    slot_player: Option<PlayerId>,
    x: Option<u16>,
    gift: bool,
) -> Vec<Target> {
    let s = q.state();
    let cx = &crate::resolve::Given { q, earlier, subject, slot_player, gift, x };
    let mut out = Vec::new();
    for p in s.seats() {
        let t = Target::Player(p);
        if (spec.filter)(cx, asker, t) && cx.can_target(asker, t) {
            out.push(t);
        }
    }
    let mut zones = vec![Zone::BATTLEFIELD, Zone::STACK];
    zones.extend(s.seats().map(Zone::graveyard));
    for zone in zones {
        for &id in s.zone(zone) {
            // A spell cannot target itself (CR 115.5); an ability may target its own source.
            if id == asker.source && zone.is(ZoneKind::Stack) {
                continue;
            }
            let t = Target::Object(id);
            if (spec.filter)(cx, asker, t) && cx.can_target(asker, t) {
                out.push(t);
            }
        }
    }
    // Abilities on the stack (CR 115.1), oldest first.
    for item in s.stack.iter().filter(|i| i.kind != StackKind::Spell) {
        let t = Target::Ability(item.serial);
        if (spec.filter)(cx, asker, t) && cx.can_target(asker, t) {
            out.push(t);
        }
    }
    out
}

/// Whether the targets are announced with the action (a single slot for one target), rather than asked slot by
/// slot once the spell or ability has begun to be put on the stack, as the C# engine does for anything else.
pub fn targets_with_action(specs: &[TargetSpec]) -> bool {
    specs.len() == 1 && specs[0].max == 1
}

/// Whether an activated ability's targets are announced with the action: one slot of one target, unless they wait for X
/// (`ActivatedDef::late_targets`, CR 601.2b-c by way of 602.2b).
fn targets_with_activation(def: &ActivatedDef) -> bool {
    !def.late_targets && targets_with_action(def.targets)
}

/// Whether every required slot has at least its minimum number of legal targets (CR 601.2c): fewer is no legal announcement, so the
/// spell or ability is not offered.
/// A slot whose targets together are judged (`TargetSpec::together`) needs that many the rule allows together, judged before X
/// is announced with its rule's `COUNTED` unread, so as large as any X could make it.
pub(crate) fn slots_satisfiable(cx: &Cx, asker: &Asker, specs: &[TargetSpec]) -> bool {
    specs.iter().all(|spec| {
        if !spec.required() {
            return true;
        }
        let candidates = target_candidates(cx, asker, spec);
        if spec.together.is_none() {
            candidates.len() >= spec.min as usize
        } else {
            select::first_targets(cx, spec.together.as_ref(), asker.controller, asker.source, &candidates, spec.min as usize).is_some()
        }
    })
}

/// Fills in an option's targets; false when a required slot has no candidate, so the option is not offered.
fn with_targets(cx: &Cx, asker: &Asker, specs: &[TargetSpec], option: &mut ActionOption) -> bool {
    if !targets_with_action(specs) {
        return slots_satisfiable(cx, asker, specs);
    }
    for spec in specs {
        // A target the rule of the slot's targets together refuses by itself is none (`TargetSpec::together`).
        let candidates = select::targets_allowed(cx, spec.together.as_slice(), asker.controller, asker.source, target_candidates(cx, asker, spec));
        if candidates.is_empty() && spec.required() {
            return false;
        }
        option.targets_required = option.targets_required.saturating_add(spec.max);
        option.targets_optional |= spec.min == 0;
        for c in candidates {
            if !option.target_candidates.contains(&c) {
                option.target_candidates.push(c);
            }
        }
    }
    true
}

/// The modes of `modal` that could be chosen now (CR 700.2a): those every one of whose target slots has a legal
/// target, or is optional.
pub(crate) fn eligible_modes(cx: &Cx, asker: &Asker, modal: &ModalDef) -> Vec<u8> {
    (0..modal.modes.len() as u8)
        .filter(|&i| {
            slots_satisfiable(cx, asker, modal.modes[i as usize].targets)
                && modal.allowed.is_none_or(|allowed| allowed(cx, asker, i))
        })
        .collect()
}

/// Whether enough modes could be chosen for the spell or ability to be announced at all (CR 700.2a).
fn modes_possible(cx: &Cx, asker: &Asker, modal: &ModalDef) -> bool {
    // A mode that may be chosen more than once is enough for any number (CR 700.2d).
    eligible_modes(cx, asker, modal).len() >= if modal.repeat { 1 } else { modal.min.max(1) as usize }
}

/// Whether a spell's targets are announced with the action: one slot of one target, on a spell that is not modal and
/// does not choose its target after its kicker or X (CR 601.2b-c), nor how many by its kicker.
fn targets_with_cast(cards: &CardTable, def: &'static CardDef) -> bool {
    let specs = cards.spell_targets_of(def);
    def.modal.is_none() && !def.late_targets && def.gift.is_none() && def.divide.is_none() && targets_with_action(specs) && specs[0].if_kicked.is_none()
}

/// Fills in a spell option's targets, or checks that its modes and targets could be announced: a modal spell needs
/// enough modes, one whose target waits on its kicker is offered if a target exists either way.
fn offer_spell(cx: &Cx, asker: &Asker, def: &'static CardDef, option: &mut ActionOption) -> bool {
    if let Some(modal) = def.modal {
        return modes_possible(cx, asker, modal);
    }
    let specs = cx.cards().spell_targets_of(def);
    // A gift is promised before the targets (CR 601.2b-c, 702.174m), so what may be targeted can depend on it: it is
    // offered if every required slot has a target without the gift, or with it.
    if def.late_targets || def.gift.is_some() || def.divide.is_some() {
        let met = |if_promised: bool| {
            specs.iter().all(|spec| {
                !spec.required()
                    || (if if_promised { target_candidates_if_promised(cx, asker, spec) } else { target_candidates(cx, asker, spec) }).len() >= spec.min as usize
            })
        };
        return def.kicker.is_some() || met(false) || (def.gift.is_some() && met(true));
    }
    // "If this spell was kicked, instead choose ..." (CR 702.33a, 601.2c): the targets wait for the kicker, and it may be
    // cast if it could be announced either unkicked or, with a kicker to pay, kicked.
    if def.kicker.is_some() && specs.iter().any(|spec| spec.if_kicked.is_some()) {
        return slots_satisfiable(cx, asker, specs) || slots_satisfiable(cx, asker, &specs.iter().map(|spec| spec.when_kicked(1)).collect::<Vec<_>>());
    }
    with_targets(cx, asker, specs, option)
}

/// How many steps - target announcements, choices toward one, and announcements made before the targets - the search for a
/// payable announcement (`some_cast`, `some_activation`) takes before it stops and answers that one might be paid for:
/// advice that offers an action whose cost the choices then made leave unpayable, and that is taken back (CR 601.2h,
/// 733.1), never one that withholds an action that could be taken. Reached only by many choices among many candidates
/// ("any number of targets" over a dozen) on a spell or ability something could change the cost of.
const ANNOUNCEMENT_BUDGET: u32 = 4096;

/// What is announced before the targets that the total cost follows from (CR 601.2b, 601.2f): the modes (a mask as
/// `StackItem::modes` holds them, with their costs, CR 700.2, 702.172a, 702.120a), X, the kicker (the times kicked, or the
/// X of a kicker with {X}), the optional additional cost, and the mana of the cards spliced onto the spell (CR 702.47a).
/// `NONE` is the cheapest: what the advice judges before anything is announced.
/// `spell` is the card as the spell will be once they are announced (`Announcing`), for the cost changes to read.
#[derive(Clone, Copy)]
struct Earlier {
    modes: Option<u32>,
    x: u16,
    kicks: u16,
    paid: bool,
    splice: ManaCost,
    spell: Option<Obj>,
}

impl Earlier {
    const NONE: Earlier = Earlier { modes: None, x: 0, kicks: 0, paid: false, splice: ManaCost::ZERO, spell: None };
}

/// The game as a spell's targets and total cost are judged once what comes before them is announced (CR 601.2b-c, 601.2f):
/// the card has the X, the kicker, the optional cost and the gift announced, as the spell on the stack will (a filter reads
/// them: "if this spell was kicked, instead target any creature").
struct Announcing<'a> {
    q: &'a dyn Query,
    spell: ObjectId,
    obj: Obj,
}

impl Query for Announcing<'_> {
    fn linked_card(&self, card: CardId) -> CardId {
        self.q.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.q.text_linked(source)
    }

    fn state(&self) -> &GameState {
        self.q.state()
    }

    fn mana_value(&self, id: ObjectId) -> u32 {
        self.q.mana_value(id)
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.q.card(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        self.q.chars(id)
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        self.q.token_chars(card, copy)
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.q.card_id(def)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        self.q.lost_all_abilities(id)
    }

    fn events(&self) -> &[Record] {
        self.q.events()
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        self.q.can_target(asker, target)
    }

    fn obj(&self, id: ObjectId) -> &Obj {
        if id == self.spell { &self.obj } else { self.q.obj(id) }
    }
}

/// The candidates of a slot that the search takes as alike (`AnnouncementSearch::slot`), and whether a cost change judged
/// since told any of them apart.
struct Watching {
    alike: std::cell::RefCell<Vec<ObjectId>>,
    told: std::cell::Cell<bool>,
}

/// The game as the cost changes read it while some candidates are taken as alike (`Watching`): every way a change could tell
/// one of them from another is noted - asking about one of them (its characteristics, its mana value, whether it may be
/// targeted), or reading anything that could name one (an object's record, the state, the history). A change that only
/// counts its targets, or reads the spell it costs, tells none apart (Fireball's "{1} more for each target beyond the
/// first"). A change compares a target to no object it was not given or did not read, so to tell two alike candidates apart
/// it must read one of these.
struct Watch<'a> {
    q: &'a dyn Query,
    w: &'a Watching,
    /// The spell being costed, a card not yet cast, where its record names no other object (`Obj::links_an_object`): reading
    /// it tells no candidates apart. A record that links one (what exiled it, what it became, ...) is noted as any is.
    spell: Option<ObjectId>,
}

impl Watch<'_> {
    fn note(&self, id: ObjectId) {
        if self.w.alike.borrow().contains(&id) {
            self.w.told.set(true);
        }
    }
}

impl Query for Watch<'_> {
    fn linked_card(&self, card: CardId) -> CardId {
        self.q.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.q.text_linked(source)
    }

    fn state(&self) -> &GameState {
        self.w.told.set(true);
        self.q.state()
    }

    fn mana_value(&self, id: ObjectId) -> u32 {
        self.note(id);
        self.q.mana_value(id)
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.q.card(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        self.note(id);
        self.q.chars(id)
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        self.q.token_chars(card, copy)
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.q.card_id(def)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        self.note(id);
        self.q.lost_all_abilities(id)
    }

    fn events(&self) -> &[Record] {
        self.w.told.set(true);
        self.q.events()
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        if let Target::Object(id) = target {
            self.note(id);
        }
        self.q.can_target(asker, target)
    }

    fn obj(&self, id: ObjectId) -> &Obj {
        // An object's record may name another (what it is attached to): only the costed card's, where it links none, tells
        // nothing.
        if Some(id) != self.spell || self.q.obj(id).links_an_object() {
            self.w.told.set(true);
        }
        self.q.obj(id)
    }
}

/// What looking for a payable announcement came to: one found, none, or the budget spent.
#[derive(PartialEq, Clone, Copy)]
enum Found {
    Yes,
    No,
    Spent,
}

/// The announcements of a spell's or ability's targets (CR 601.2c) for one announcement of what comes before them, as
/// `choose_slots` takes them: segment by segment (the spell's own slots, then each spliced card's in the order spliced,
/// CR 702.47b, their targets chosen after the spell's own, 702.47d), each segment's slots in order, the targets of a slot
/// different, a slot that chose nothing ("up to") keeping its place with `Target::None` among several, and a segment's
/// trailing empty slots taking none.
///
/// The last slot's candidates that no cost change could tell apart are taken as alike: objects the changes are not given
/// (`named`) nor chosen earlier. An announcement with some of them is judged once for each number of them, with the first
/// that many; while the changes judged read nothing that tells them apart (`Watch`), every other choice of as many is the
/// same to them. Once one does, the slot is walked whole. So Fireball's targets are judged by how many there are, not by
/// every set of them.
struct AnnouncementSearch<'a, K> {
    q: &'a dyn Query,
    asker: &'a Asker,
    /// Each slot, the opponent it is for, its segment, and how many slots its segment has.
    slots: Vec<(TargetSpec, Option<PlayerId>, usize, usize)>,
    counted: u32,
    gift: bool,
    key: &'a dyn Fn(&[Target], &Watching) -> K,
    covered: &'a mut dyn FnMut(&[Target]) -> bool,
    /// What the cost changes made of the announcements judged: `covered` is asked once for each.
    seen: Vec<K>,
    budget: &'a mut u32,
    /// The objects the cost changes are given (their sources, what is costed): never taken as alike.
    named: &'a [ObjectId],
    watching: Watching,
}

impl<K: PartialEq> AnnouncementSearch<'_, K> {
    fn spend(&mut self) -> bool {
        *self.budget = self.budget.saturating_sub(1);
        *self.budget > 0
    }

    /// The announcement of slot `at` on, after `targets` (its segment from `start`) for the slots before it.
    fn slot(&mut self, at: usize, targets: &mut Vec<Target>, start: usize) -> Found {
        if !self.spend() {
            return Found::Spent;
        }
        let mut start = start;
        if at > 0 && self.slots.get(at).is_none_or(|s| s.2 != self.slots[at - 1].2) {
            // A segment's trailing slots that chose nothing take no place (`choose_slots`).
            while targets.len() > start && targets.last() == Some(&Target::None) {
                targets.pop();
            }
            start = targets.len();
        }
        let Some(&(spec, slot_player, _, in_segment)) = self.slots.get(at) else {
            let key = (self.key)(targets, &self.watching);
            if self.seen.contains(&key) {
                return Found::No;
            }
            self.seen.push(key);
            return if (self.covered)(targets) { Found::Yes } else { Found::No };
        };
        let earlier: &[Target] = if in_segment > 1 { &targets[start..] } else { &[] };
        let candidates = candidates(self.q, self.asker, &spec, earlier, Target::None, slot_player, None, self.gift);
        let (candidates, together) = within_together(self.q, self.asker, &spec, self.counted, candidates);
        let (least, most) = spec.bounds(self.counted);
        if candidates.len() < least as usize || (candidates.is_empty() && spec.required()) {
            return Found::No;
        }
        let count = most.min(cantrip_state::count_u32(candidates.len())) as usize;
        let least = (least as usize).min(count);
        let before = targets.len();
        // The last slot, of no rule of its targets together, with candidates to take as alike: judged by how many of them,
        // while the changes tell none apart.
        if at + 1 == self.slots.len() && together.is_none() && !self.watching.told.get() {
            let alike: Vec<ObjectId> = candidates
                .iter()
                .filter_map(|&t| if let Target::Object(id) = t { Some(id) } else { None })
                .filter(|id| !self.named.contains(id) && !targets.contains(&Target::Object(*id)))
                .collect();
            if alike.len() > 1 && count > 1 {
                let named: Vec<Target> = candidates.iter().copied().filter(|t| !matches!(t, Target::Object(id) if alike.contains(id))).collect();
                *self.watching.alike.borrow_mut() = alike.clone();
                let found = self.alike(at, targets, start, &named, &alike, (least, count, in_segment > 1 && spec.max <= 1));
                self.watching.alike.borrow_mut().clear();
                targets.truncate(before);
                if found != Found::No || !self.watching.told.get() {
                    return found;
                }
                // A change told them apart: the slot is walked whole.
            }
        }
        let mut chosen: Vec<usize> = Vec::with_capacity(count);
        let mut picked: Vec<Target> = Vec::new();
        for n in least..=count {
            // Each `n` of the candidates, in order (CR 601.2c: different ones).
            chosen.clear();
            chosen.extend(0..n);
            loop {
                let allowed = together.is_none() || {
                    picked.clear();
                    picked.extend(chosen.iter().map(|&i| candidates[i]));
                    select::targets_together(self.q, together.as_slice(), self.asker.controller, self.asker.source, &picked)
                };
                if allowed {
                    targets.extend(chosen.iter().map(|&i| candidates[i]));
                    if in_segment > 1 && spec.max <= 1 && n == 0 {
                        targets.push(Target::None);
                    }
                    let found = self.slot(at + 1, targets, start);
                    targets.truncate(before);
                    if found != Found::No {
                        return found;
                    }
                } else if !self.spend() {
                    return Found::Spent;
                }
                // The next `n` of them, if any.
                let Some(i) = (0..n).rev().find(|&i| chosen[i] < candidates.len() - n + i) else { break };
                chosen[i] += 1;
                for j in i + 1..n {
                    chosen[j] = chosen[j - 1] + 1;
                }
            }
        }
        Found::No
    }

    /// The last slot `at` judged with its `alike` candidates taken as alike: each choice of its `named` ones, with the first
    /// so many of the alike ones to make each number from `least` to `count` (`pad`: a slot of one that chose nothing keeps
    /// its place). Stops, answering no, once a change tells the alike ones apart.
    fn alike(&mut self, at: usize, targets: &mut Vec<Target>, start: usize, named: &[Target], alike: &[ObjectId], (least, count, pad): (usize, usize, bool)) -> Found {
        let before = targets.len();
        let mut chosen: Vec<usize> = Vec::new();
        for n in 0..=named.len().min(count) {
            chosen.clear();
            chosen.extend(0..n);
            loop {
                for k in least.saturating_sub(n)..=(count - n).min(alike.len()) {
                    targets.extend(chosen.iter().map(|&i| named[i]));
                    targets.extend(alike[..k].iter().map(|&id| Target::Object(id)));
                    if pad && n + k == 0 {
                        targets.push(Target::None);
                    }
                    let found = self.slot(at + 1, targets, start);
                    targets.truncate(before);
                    if found != Found::No || self.watching.told.get() {
                        return found;
                    }
                }
                let Some(i) = (0..n).rev().find(|&i| chosen[i] < named.len() - n + i) else { break };
                chosen[i] += 1;
                for j in i + 1..n {
                    chosen[j] = chosen[j - 1] + 1;
                }
            }
        }
        Found::No
    }
}

/// Whether some legal announcement of the targets of `segments` after `prefix` (the target announced with the action, if
/// any), with what was announced before them (`counted`, `kicked`, `gift`, and `q` reading them), has a total cost `covered`
/// holds for. `key` is what the cost changes make of an announcement (`deltas`), the only part of the cost the targets
/// reach: `covered` is asked once for each different one.
#[allow(clippy::too_many_arguments)]
fn some_announcement<K: PartialEq>(
    q: &dyn Query,
    cx: &Cx,
    asker: &Asker,
    prefix: &[Target],
    segments: &[Vec<TargetSpec>],
    (counted, kicked, gift): (u32, u8, bool),
    budget: &mut u32,
    named: &[ObjectId],
    key: &dyn Fn(&[Target], &Watching) -> K,
    covered: &mut dyn FnMut(&[Target]) -> bool,
) -> Found {
    let mut slots = Vec::new();
    for (segment, specs) in segments.iter().enumerate() {
        let expanded = crate::resolve::expand_specs(cx.s(), asker.controller, specs);
        let n = expanded.len();
        slots.extend(expanded.into_iter().map(|(spec, player, _)| (spec.when_kicked(kicked), player, segment, n)));
    }
    let watching = Watching { alike: std::cell::RefCell::new(Vec::new()), told: std::cell::Cell::new(false) };
    let mut search = AnnouncementSearch { q, asker, slots, counted, gift, key, covered, seen: Vec::new(), budget, named, watching };
    let mut targets = prefix.to_vec();
    search.slot(0, &mut targets, prefix.len())
}

/// The objects the cost changes of `costed` are given (`CostChange`'s source, and what is costed): the permanents with cost
/// changes, the sources of lasting ones, and the costed object. A change may compare a target to these without reading
/// anything, so they are never taken as alike (`AnnouncementSearch`).
fn named_by_changes(cx: &Cx, a: &Available, costed: ObjectId) -> Vec<ObjectId> {
    let mut named = a.changers.clone();
    named.push(costed);
    named.extend(cx.s().effects.iter().filter(|e| matches!(e.kind, EffectKind::SpellCostChange { .. })).map(|e| e.source));
    named
}

/// Whether a slot's number of targets, or a part of the rule of its targets together ("with total mana value X or less",
/// "mana value X or less"), is counted by X or the kicker (CR 601.2b-c), unkicked or kicked.
fn reads_count(spec: &TargetSpec) -> bool {
    [*spec, spec.when_kicked(1)].iter().any(|s| s.min == TargetSpec::COUNTED || s.max == TargetSpec::COUNTED || s.together_counted())
}

/// The most X (or times kicked) that could make a difference to the targets of slots counted by it (`reads_count`): the
/// candidates there are, or their mana values together for a slot whose rule of its targets together reads it - a mana
/// value of the targets' (each at most X, or all of them together), which no X past their total changes; past that more
/// only costs more. A part reading X as anything but a mana value (a total power) must widen this bound.
fn count_bound<'a>(cx: &Cx, asker: &Asker, specs: impl Iterator<Item = &'a TargetSpec>) -> u16 {
    let mut bound = 0u32;
    for spec in specs.filter(|s| reads_count(s)) {
        let candidates = target_candidates(cx, asker, &spec.when_kicked(1));
        let n = if spec.together_counted() {
            candidates.iter().map(|&t| if let Target::Object(id) = t { cx.mana_value(id) } else { 0 }).sum::<u32>()
        } else {
            cantrip_state::count_u32(candidates.len())
        };
        bound = bound.max(n);
    }
    bound.min(u32::from(u16::MAX)) as u16
}

/// The choices of modes `modal` could be cast or activated with, between `range`'s numbers (CR 700.2), as masks: of the
/// modes that could be chosen now (`eligible_modes`), each once, or for a modal that repeats up to its most each
/// (CR 700.2d).
fn mode_masks(cx: &Cx, asker: &Asker, modal: &ModalDef, range: (u8, u8)) -> Vec<u32> {
    fn go(modal: &ModalDef, eligible: &[u8], at: usize, chosen: u8, mask: u32, (min, max): (u8, u8), out: &mut Vec<u32>) {
        let Some(&mode) = eligible.get(at) else {
            if chosen >= min.max(1) {
                out.push(mask);
            }
            return;
        };
        let most = if modal.repeat { max.saturating_sub(chosen).min(15) } else { u8::from(chosen < max) };
        let shift = if modal.repeat { 4 * u32::from(mode) } else { u32::from(mode) };
        if shift >= 32 {
            return go(modal, eligible, at + 1, chosen, mask, (min, max), out);
        }
        for times in 0..=most {
            go(modal, eligible, at + 1, chosen + times, mask | (u32::from(times) << shift), (min, max), out);
        }
    }
    let eligible = eligible_modes(cx, asker, modal);
    // The board may allow more modes than it prints (CR 700.2: Flame of Anor), as `choose_modes` reads it.
    let (min, mut max) = range;
    if let Some(up_to) = cx.card(cx.s().obj(asker.source).card).modes_up_to {
        max = max.max(up_to(cx, asker));
    }
    let mut out = Vec::new();
    go(modal, &eligible, 0, 0, 0, (min, max), &mut out);
    out
}

/// The cards in `player`'s hand that could be spliced onto the Arcane spell `card` (CR 702.47a) and bring targets: one without
/// adds to the cost and changes nothing a cost change reads.
fn splice_cards(cx: &Cx, player: PlayerId, card: ObjectId) -> Vec<ObjectId> {
    if !cx.chars(card).has_subtype(Subtype::Arcane) {
        return Vec::new();
    }
    let targeted = |c: ObjectId| {
        let def = cx.card(cx.s().obj(c).card);
        c != card && def.splice.is_some() && def.spell.is_some_and(|s| !s.targets.is_empty())
    };
    cx.s().zone(Zone::hand(player)).iter().copied().filter(|&c| targeted(c)).collect()
}

/// `f` for each order of the `cards` that could be spliced (CR 702.47b: each once, in the order chosen), none first, walked
/// one at a time so the search's budget bounds it: stops at the first answer that is not no.
fn each_splice_order(cards: &[ObjectId], now: &mut Vec<ObjectId>, f: &mut dyn FnMut(&[ObjectId]) -> Found) -> Found {
    let found = f(now);
    if found != Found::No {
        return found;
    }
    for &c in cards {
        if !now.contains(&c) {
            now.push(c);
            let found = each_splice_order(cards, now, f);
            now.pop();
            if found != Found::No {
                return found;
            }
        }
    }
    Found::No
}

/// Whether casting `def` of `asker.source` could be paid for at some legal announcement (CR 601.2b-c, 601.2f): its modes, X,
/// kicker, optional cost, gift and splices, then the targets they bring, its total cost determined from all of them -
/// This Town Ain't Big Enough ("costs {3} less to cast if it targets a permanent you control", up to two targets) is
/// cast for {1}{U} aimed at a permanent of its caster's, and Fireball ("costs {1} more to cast for each target beyond the
/// first", any number of targets) for its {R} with one target or none. `prefix` is the target announced with the cast, where
/// it is (`targets_with_cast`): the kicker, optional cost and splices are judged with it, and X at its least, which is all
/// its target needs. `x_symbols` is the {X} of the cost being paid. Past `ANNOUNCEMENT_BUDGET` it answers yes.
#[allow(clippy::too_many_arguments)]
fn some_cast(
    cx: &Cx,
    a: &Available,
    asker: &Asker,
    def: &'static CardDef,
    x_symbols: u8,
    prefix: Option<&[Target]>,
    budget: &mut u32,
    covered: &dyn Fn(&[Target], &Earlier) -> bool,
) -> bool {
    let (player, card) = (asker.controller, asker.source);
    let face = face_of(cx, card, def);
    let splices = splice_cards(cx, player, card);
    let named = named_by_changes(cx, a, card);
    let own = cx.cards().spell_targets_of(def);
    let open = prefix.is_none();
    // Promised to each opponent in the game, or not (CR 702.174a-b).
    let mut gifts: Vec<Option<PlayerId>> = vec![None];
    if open && def.gift.is_some() {
        gifts.extend(cx.s().opponents(player).filter(|&p| !cx.s().player(p).lost).map(Some));
    }
    let paid: &[bool] = if def.optional_cost.is_some() { &[false, true] } else { &[false] };
    let mode_specs_all = def.modal.iter().flat_map(|m| m.modes.iter().flat_map(|mode| mode.targets.iter()));
    let splice_specs = splices.iter().flat_map(|&c| cx.card(cx.s().obj(c).card).spell.map_or(&[][..], |s| s.targets).iter());
    let mut bound = count_bound(cx, asker, own.iter().chain(mode_specs_all).chain(splice_specs));
    // A total divided that X sets (CR 601.2d): as many targets as there are candidates need a total that large.
    if let Some(divide) = def.divide.filter(|d| d.of_x.is_some() || d.total == TargetSpec::COUNTED)
        && let Some(spec) = own.first()
    {
        let there = cantrip_state::count_u32(target_candidates(cx, asker, spec).len());
        let total = |x: u16| divide.of_x.map_or(u32::from(x), |f| u32::from(f(x)));
        bound = bound.max((0..=1000u16).find(|&x| total(x) >= there).unwrap_or(1000));
    }
    let kicks = match def.kicker {
        Some(k) if k.multi || k.cost.x > 0 => 0..=bound.max(1),
        Some(_) => 0..=1,
        _ => 0..=0,
    };
    // A kicker takes the X announced (CR 601.2b, `cast`); otherwise X from its least, up to what changes the targets. X is
    // announced for the {X} of the cost, or for an additional cost that pays X life or sacrifices X permanents (CR 601.2b,
    // 107.3), up to the life or permanents there are.
    let least = least_x(def.x_cant_be_zero);
    let variable = def.additional_cost.and_then(|c| c.variable);
    let x_life = def.additional_cost.is_some_and(|c| c.pay_x_life);
    let x_sacrifices = matches!(variable, Some(VariableCost::SacrificeX(_) | VariableCost::SacrificeWithPowerX(_)));
    let cap = if x_life {
        payable_life_for(cx, player, Some(CostFor::Spell)).clamp(0, 1000) as u32
    } else if let Some(v @ VariableCost::SacrificeX(_)) = variable {
        cantrip_state::count_u32(variable_candidates(cx, card, player, v, Some(CostFor::Spell), None).len())
    } else {
        u32::from(u16::MAX)
    };
    let most = u16::try_from(u32::from(bound.max(least)).min(cap)).unwrap_or(u16::MAX).max(least);
    let xs = if def.kicker.is_some() || (x_symbols == 0 && !x_life && !x_sacrifices) {
        0..=0
    } else if open {
        least..=most
    } else {
        least..=least
    };
    for &paid in paid {
        for kicks in kicks.clone() {
            for x in xs.clone() {
                // A kicker with {X} announces X and is kicked when it is not 0 (CR 702.33a, 107.3).
                let (x, kicked) = match def.kicker {
                    Some(k) if k.cost.x > 0 => (kicks, u8::from(kicks > 0)),
                    _ => (x, kicks.min(255) as u8),
                };
                let counted = u32::from(if x > 0 { x } else { u16::from(kicked) });
                let mut obj = *cx.s().obj(card);
                (obj.x, obj.kicked, obj.paid) = (x, kicked, u32::from(paid));
                let masks: Vec<Option<u32>> = match def.modal {
                    Some(modal) if open => {
                        let range = match modal.when_paid {
                            Some(range) if paid || kicked > 0 => range,
                            _ => (modal.min, modal.max),
                        };
                        mode_masks(cx, asker, modal, range).into_iter().map(Some).collect()
                    }
                    _ => vec![None],
                };
                for &gift in &gifts {
                    obj.gift = gift;
                    let view = Announcing { q: cx, spell: card, obj };
                    let key = |targets: &[Target], watching: &Watching| {
                        deltas_watched(cx, &a.changers, player, Costed::Spell(card), face, targets, Some(&obj), Some(watching))
                    };
                    for &mask in &masks {
                        let mine: Vec<TargetSpec> = match (prefix, def.modal, def.divide) {
                            (Some(_), _, _) => Vec::new(),
                            (None, Some(modal), _) => mode_specs(modal, mask.unwrap_or(0)),
                            (None, None, Some(divide)) => {
                                let total = match divide.of_x {
                                    Some(of_x) => u32::from(of_x(x)),
                                    None if divide.total == TargetSpec::COUNTED => u32::from(x),
                                    None => divide.total,
                                };
                                // Each target is given at least one of the total (CR 601.2d); none, for a total of nothing.
                                own.iter().map(|s| TargetSpec { min: if total == 0 { 0 } else { s.min }, max: s.max.min(total), ..*s }).collect()
                            }
                            (None, None, None) => own.to_vec(),
                        };
                        let found = each_splice_order(&splices, &mut Vec::new(), &mut |order| {
                            let mut segments = vec![mine.clone()];
                            let mut splice = ManaCost::ZERO;
                            for &c in order {
                                let spliced = cx.card(cx.s().obj(c).card);
                                segments.push(spliced.spell.map_or(Vec::new(), |s| s.targets.to_vec()));
                                splice = add_cost(splice, spliced.splice.unwrap_or(ManaCost::ZERO));
                            }
                            let earlier = Earlier { modes: mask, x, kicks, paid, splice, spell: Some(obj) };
                            some_announcement(
                                &view,
                                cx,
                                asker,
                                prefix.unwrap_or(&[]),
                                &segments,
                                (counted, kicked, gift.is_some()),
                                budget,
                                &named,
                                &key,
                                &mut |t| covered(t, &earlier),
                            )
                        });
                        if found != Found::No || *budget == 0 {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Whether activating the ability `def` (the `ability` of `asker.source`) for `cost` could be paid for at some legal
/// announcement (CR 602.2b, 601.2b-c, 601.2f): its modes and X, then the targets they bring. `covered` is asked with the
/// cost X makes it. Past `ANNOUNCEMENT_BUDGET` it answers yes.
fn some_activation(
    cx: &Cx,
    a: &Available,
    asker: &Asker,
    def: &ActivatedDef,
    ability: AbilityRef,
    cost: ManaCost,
    covered: &dyn Fn(ManaCost, &[Target]) -> bool,
) -> bool {
    let (player, id) = (asker.controller, asker.source);
    let key = |targets: &[Target], watching: &Watching| {
        deltas_watched(cx, &a.changers, player, Costed::Ability(id, ability, asker.by), None, targets, None, Some(watching))
    };
    let named = named_by_changes(cx, a, id);
    let mode_specs_all = def.modal.iter().flat_map(|m| m.modes.iter().flat_map(|mode| mode.targets.iter()));
    let bound = count_bound(cx, asker, def.targets.iter().chain(mode_specs_all));
    let least = least_x(def.x_cant_be_zero);
    // X of the mana cost (CR 107.3a), or of a "−X" loyalty cost (CR 606.4), which costs no mana.
    let xs = if cost.x > 0 || def.cost.loyalty == Some(LOYALTY_MINUS_X) { least..=bound.max(least) } else { 0..=0 };
    let masks: Vec<u32> = match def.modal {
        Some(modal) => mode_masks(cx, asker, modal, (modal.min, modal.max)),
        None => vec![0],
    };
    let mut budget = ANNOUNCEMENT_BUDGET;
    for x in xs {
        let mut owed = cost;
        if cost.x > 0 {
            owed.generic = owed.generic.saturating_add((u32::from(x) * u32::from(cost.x)).min(255) as u8);
            owed.x = 0;
        }
        for &mask in &masks {
            let specs = match def.modal {
                Some(modal) => mode_specs(modal, mask),
                None => def.targets.to_vec(),
            };
            let found = some_announcement(cx, cx, asker, &[], &[specs], (u32::from(x), 0, false), &mut budget, &named, &key, &mut |t| covered(owed, t));
            if found != Found::No || budget == 0 {
                return true;
            }
        }
    }
    false
}

/// Whether a cast is worth offering, its target candidates kept to those it could be paid for. The total cost is determined
/// once the targets are announced (CR 601.2c, 601.2f), after what is announced before them (CR 601.2b): where something
/// could change the cost, a cast whose one target is announced with it is judged for each candidate (`covered`), with the
/// cards that could be spliced onto it and their targets, and only those it could be paid for are offered; any other is
/// offered if some legal announcement could be paid for (`some_cast`). Otherwise it is judged once, with no target.
fn offer_costed(
    cx: &Cx,
    a: &Available,
    asker: &Asker,
    def: &'static CardDef,
    x_symbols: u8,
    option: &mut ActionOption,
    covered: impl Fn(&[Target], &Earlier) -> bool,
) -> bool {
    let changes = !a.changers.is_empty() || !def.own_cost_changes.is_empty() || lasting_changes(cx);
    if !changes {
        return covered(&[], &Earlier::NONE) && offer_spell(cx, asker, def, option);
    }
    // A kicker, an optional cost or a splice announced after the target announced with the cast (CR 601.2b-c) may change
    // what a change reads ("if this spell was kicked") or bring targets of its own.
    let announces = def.kicker.is_some() || def.optional_cost.is_some() || !splice_cards(cx, asker.controller, asker.source).is_empty();
    let mut budget = ANNOUNCEMENT_BUDGET;
    if targets_with_cast(cx.cards(), def) {
        if !offer_spell(cx, asker, def, option) {
            return false;
        }
        let mut judge =
            |prefix: &[Target]| covered(prefix, &Earlier::NONE) || announces && some_cast(cx, a, asker, def, x_symbols, Some(prefix), &mut budget, &covered);
        let none_covered = option.targets_optional && judge(&[]);
        option.target_candidates.retain(|&t| judge(&[t]));
        !option.target_candidates.is_empty() || none_covered
    } else {
        offer_spell(cx, asker, def, option) && some_cast(cx, a, asker, def, x_symbols, None, &mut budget, &covered)
    }
}

/// Whether the life the cost changes add to casting `card` as `def` with these targets (CR 118.8, 601.2f: "cost an
/// additional 3 life") could be paid, with the life its own additional cost asks, from its caster's life total (CR 119.4,
/// 118.3). Advice, as `least_cost` is.
fn added_life_payable(cx: &Cx, a: &Available, player: PlayerId, card: ObjectId, def: &CardDef, targets: &[Target], earlier: &Earlier) -> bool {
    if a.changers.is_empty() && def.own_cost_changes.is_empty() && !lasting_changes(cx) {
        return true;
    }
    let (_, _, added, _, _) = deltas_as(cx, &a.changers, player, Costed::Spell(card), face_of(cx, card, def), targets, earlier.spell.as_ref());
    added == 0
        || i64::from(payable_life_for(cx, player, Some(CostFor::Spell))) >= i64::from(added) + i64::from(def.additional_cost.map_or(0, |c| c.life))
}

/// The face `def` is of the card `card`, where it is one of its faces rather than the card itself (an Adventure, an Omen, a
/// back face, a bestowed Aura, a half): what the cost changes judge it as (CR 715.4, 720.3a, 709.3, 712.8a).
fn face_of(cx: &Cx, card: ObjectId, def: &CardDef) -> Option<CardId> {
    let whole = cx.card(cx.s().obj(card).card);
    let faces = [whole.adventure, whole.omen, whole.back, whole.bestow, whole.split.map(|s| s.0), whole.split.map(|s| s.1), cantrip_api::prototyped_face(whole)];
    faces.into_iter().flatten().find(|&f| std::ptr::eq(f, def)).map(|f| cx.card_id(f))
}

/// A cost with the static abilities that change it applied (CR 601.2f, 118.7): increases first, then reductions, a
/// reduction taking nothing below zero generic mana (CR 118.7c), and the life the changes add to the total cost as
/// additional costs (CR 118.8, 601.2f: "cost an additional 3 life to cast"). A reduction by hybrid mana is taken at its
/// first halves: what is paid announces and splits them first (`announce_hybrids`, `announce_reduction`).
pub fn apply_cost_changes_total(cx: &Cx, player: PlayerId, costed: Costed, cost: ManaCost, targets: &[Target]) -> (ManaCost, u32) {
    change_total(cx, &cost_change_sources(cx), player, costed, cost, None, targets)
}

/// `apply_cost_changes_total` with the least the total may come to (`CostDelta::at_least`) not yet applied but returned, for
/// a cast whose own reductions (harmonize, emerge, a sacrifice for each) are still to come: that least is applied once they
/// are (CR 601.2f).
/// `split`: the halves the payer chose of the hybrid symbols the reductions are by (CR 118.7e, `announce_reduction`).
fn apply_cost_changes_before_least(cx: &Cx, player: PlayerId, costed: Costed, cost: ManaCost, targets: &[Target], split: &[bool]) -> (ManaCost, u32, u8) {
    change_parts_split(cx, &cost_change_sources(cx), player, costed, cost, None, targets, split)
}

/// The total cost brought up to the least mana an effect says it costs (CR 601.2f, `CostDelta::at_least`): generic mana
/// makes up the difference, and the cost's own symbols are kept.
fn at_least(mut cost: ManaCost, least: u8) -> ManaCost {
    let value = cost.mana_value();
    if value < least as u32 {
        cost.generic = cost.generic.saturating_add((least as u32 - value) as u8);
    }
    cost
}

/// The permanents on the battlefield whose cards change costs: found from the board, not from the card table (which
/// holds every card), and empty, so unallocated, in nearly every game.
fn cost_change_sources(cx: &Cx) -> Vec<ObjectId> {
    cx.s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let obj = cx.s().obj(id);
            cx.cards().cost_changes[cx.cards().current(obj).0 as usize] && !cx.lost_all_abilities(id)
        })
        .collect()
}

/// Whether a cost change a resolved spell or ability made still lasts (CR 611.2c, `EffectKind::SpellCostChange`): read from
/// the effects in play, which nearly never hold one.
fn lasting_changes(cx: &Cx) -> bool {
    cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::SpellCostChange { .. }))
}

/// Whether anything in play could change what a spell costs (CR 601.2f): a permanent with a cost change, or a lasting effect
/// a resolved spell or ability made.
fn spell_cost_changers(game: &crate::Game) -> bool {
    game.state.effects.iter().any(|e| matches!(e.kind, EffectKind::SpellCostChange { .. }))
        || game.state.battlefield().iter().any(|&id| {
            game.cards.cost_changes[game.cards.current(game.state.obj(id)).0 as usize] && !game.lost_all_abilities(id)
        })
}

/// The game as the cost changes of a spell read it when the spell is judged before it is cast: the card in hand has the
/// characteristics of the face it would be cast as, the only spell it would be on the stack (CR 715.4, 709.3, 712.8a).
struct AsFace<'a> {
    cx: &'a Cx<'a>,
    spell: ObjectId,
    face: CardId,
}

impl Query for AsFace<'_> {
    fn linked_card(&self, card: CardId) -> CardId {
        self.cx.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.cx.text_linked(source)
    }

    fn state(&self) -> &GameState {
        self.cx.s()
    }

    /// A nonmodal double-faced card judged as its back face, as it would be cast transformed, has its front face's mana value
    /// (CR 712.8c, 202.3b); any other face its own.
    fn mana_value(&self, id: ObjectId) -> u32 {
        let front = self.cx.card(self.cx.s().obj(id).card);
        match front.back {
            Some(back) if id == self.spell && !back.has_cost && !front.back_land && std::ptr::eq(self.cx.card(self.face), back) => front.cost.mana_value(),
            _ => cantrip_api::mana_value_of(&|c| self.cx.card(c), self.cx.s(), id, &self.chars(id)),
        }
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.cx.card(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        if id == self.spell { layers::chars_as_face(self.cx.cards(), self.cx.s(), &self.cx.g.log, id, self.face) } else { self.cx.chars(id) }
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        self.cx.token_chars(card, copy)
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.cx.card_id(def)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        self.cx.lost_all_abilities(id)
    }

    fn events(&self) -> &[Record] {
        self.cx.events()
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        self.cx.can_target(asker, target)
    }
}

/// What activating the ability `index` of `source` (numbered as its options are) would cost `player` with each of
/// `candidates` as its one target, the total as it is determined once the target is announced (CR 601.2c, 601.2f,
/// 602.2b): for the menu row of an action whose cost depends on the target it would be announced with. Empty when
/// nothing could change that cost (no cost changer on the battlefield, none of the ability's or card's own), which is
/// nearly always, or when it is no activated ability with a mana cost.
pub fn activation_costs(game: &crate::Game, player: PlayerId, source: ObjectId, index: u8, candidates: &[Target]) -> Vec<(ManaCost, u32)> {
    let Some(ability) = game.chars(source).abilities().nth(index as usize) else { return Vec::new() };
    let AbilityDef::Activated(def) = game.cards.ability(ability) else { return Vec::new() };
    let changers = game.state.battlefield().iter().any(|&id| {
        game.cards.cost_changes[game.cards.current(game.state.obj(id)).0 as usize] && !game.lost_all_abilities(id)
    });
    let obj = game.state.obj(source);
    let own = !game.cards.get(obj.copied_card().unwrap_or(obj.card)).own_cost_changes.is_empty();
    if def.special_action || def.cost.mana.x > 0 || !(changers || own || def.cost_change.is_some()) {
        return Vec::new();
    }
    // The rules read the game through a frame's view; a copy of the position (no tree) is enough to ask it.
    let mut copy = crate::Game { state: game.state.clone(), log: game.log.clone(), tree: crate::tree::Tree::new(), at: crate::tree::NodeId::ROOT, cards: game.cards, verify: false, max_events: game.max_events, history_end: None, check: crate::StateCheck::NONE, spells_as_cast: Default::default() };
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    // The ability asked as it is (CR 607.5): its own cost change reads its own links.
    let by = cx.link_of_nth(source, index as usize);
    candidates.iter().map(|&t| apply_cost_changes_total(&cx, player, Costed::Ability(source, ability, by), def.cost.mana, &[t])).collect()
}

/// What `player` activating the mana ability `index` of `source` would add once the replacement effects on a permanent tapped
/// for mana have applied (CR 106.12b), for its menu row: for each type its chosen mana may be (None where it chooses none),
/// every outcome, in each order the player could apply them (`replace::mana_outcomes`) - the same judgment the tap itself and
/// what the player has to pay with (`available`) make. None where no replacement effect would change what it adds, which is
/// nearly always, or where it is no mana ability with {T} in its cost or its yield is not stated.
pub fn replaced_mana_yields(game: &crate::Game, player: PlayerId, source: ObjectId, index: u8) -> Option<Vec<(Option<Mana>, Vec<ManaPool>)>> {
    if !crate::replace::mana_may_be_replaced_in(&game.state, game.cards) {
        return None;
    }
    let ability = game.chars(source).abilities().nth(index as usize)?;
    let AbilityDef::Activated(def) = game.cards.ability(ability) else { return None };
    if !def.is_mana_ability() || !def.cost.tap || def.colors_from_sacrifice || def.adds.0.iter().all(|&n| n == 0) && def.adds_any == 0 {
        return None;
    }
    let mut copy = advice_game(game);
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    // Asked as the ability it is (CR 607.5): one an effect gave the source reads only its own links.
    let by = cx.link_of_nth(source, index as usize);
    let types = if def.adds_any > 0 { any_types(&cx, source, player, def, by) } else { 0 };
    let restriction = def.restricted.map(|r| restriction_from(&cx, source, r, by));
    let choices: Vec<Option<Mana>> =
        if def.adds_any > 0 && types != 0 { Mana::ALL.into_iter().filter(|&m| types & (1 << m as u8) != 0).map(Some).collect() } else { vec![None] };
    let mut changed = false;
    let yields = choices
        .into_iter()
        .map(|chosen| {
            let mut made = def.adds.0;
            if let Some(m) = chosen {
                made[m as usize] = made[m as usize].saturating_add(u32::from(def.adds_any));
            }
            let outcomes = replaced_yields(&cx, source, player, made, 0, 0, restriction).map_or_else(|| vec![ManaPool(made)], |o| {
                changed = true;
                o.into_iter().map(|p| p.mana).collect()
            });
            (chosen, outcomes)
        })
        .collect();
    changed.then_some(yields)
}

/// The face a cast option casts (CR 709.3, 715.3, 720.3, 712.8a): the card itself, or the face its way number names.
pub fn cast_face_of(game: &crate::Game, card: ObjectId, index: u8) -> &'static CardDef {
    cast_face(game.cards.get(game.state.obj(card).card), index)
}

/// A copy of the position (no tree) to ask the rules of, through a frame's view, for a menu row's advice.
fn advice_game(game: &crate::Game) -> crate::Game {
    crate::Game { state: game.state.clone(), log: game.log.clone(), tree: crate::tree::Tree::new(), at: crate::tree::NodeId::ROOT, cards: game.cards, verify: false, max_events: game.max_events, history_end: None, check: crate::StateCheck::NONE, spells_as_cast: Default::default() }
}

/// What `player` would be offered at this priority with a thousand mana of every type in their pool: what a script's refusal
/// of a cast or an activation is held against, so that one that holds only because the cost could not be paid (CR 601.2g,
/// 602.2b: such an action is not offered at all) is told from one the rules under test refuse. Nothing is paid; the pool
/// of a copy is all that changes.
pub fn options_with_mana_to_spare(game: &crate::Game, player: PlayerId) -> Vec<ActionOption> {
    let mut copy = advice_game(game);
    copy.state.update_player(player, |p| {
        for mana in Mana::ALL {
            p.pool.0[mana as usize] += 1000;
        }
    });
    let mut cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    priority_options(&mut cx, player)
}

/// `options_with_mana_to_spare` for a payment: the mana abilities `player` could activate while paying a cost (CR 601.2g,
/// 605.3a) with a thousand mana of every type to spare, for the scenario harness to tell a mana ability refused at a payment
/// for want of mana from one refused for any other reason.
pub fn payment_mana_options_with_mana_to_spare(game: &crate::Game, player: PlayerId) -> Vec<ActionOption> {
    let mut copy = advice_game(game);
    copy.state.update_player(player, |p| {
        for mana in Mana::ALL {
            p.pool.0[mana as usize] += 1000;
        }
    });
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    let mut options = Vec::new();
    mana_ability_options(&cx, player, ManaWindow::Payment, &mut options);
    options
}

/// The face a cast option casts, as the card id the cost changes judge it by where it is not the card itself (CR 715.4,
/// 720.3a, 709.3, 712.8a).
fn option_face(game: &crate::Game, card: ObjectId, index: u8) -> (&'static CardDef, Option<CardId>) {
    let whole = game.cards.get(game.state.obj(card).card);
    let face = cast_face(whole, index);
    (face, (!std::ptr::eq(face, whole)).then(|| game.cards.id_of(face)))
}

/// What casting a card by the cast option `option` would cost `player` with each of `candidates` as its one target, the
/// cost changes applied as they are once the target is announced (CR 601.2c, 601.2f): its mana, and the life the changes
/// add (CR 118.8). For the menu row of a cast whose cost depends on its target. Empty when nothing could change it (no cost
/// changer on the battlefield, none of the card's own), which is nearly always, or for a cast of another of its faces or ways
/// for its own cost.
pub fn cast_costs(game: &crate::Game, player: PlayerId, option: &ActionOption, candidates: &[Target]) -> Vec<(ManaCost, u32)> {
    let Some(card) = option.source else { return Vec::new() };
    // A card exiled face down is judged as the spell it would be (CR 406.3a), the copy below with it.
    let _face_up = layers::judged_face_up(&game.state, card);
    let def = game.cards.get(game.state.obj(card).card);
    if !(spell_cost_changers(game) || !def.own_cost_changes.is_empty()) {
        return Vec::new();
    }
    let base = if let Some((cost, _)) = alternative_cast_total(game, player, option) {
        cost.mana
    } else if let Some(cost) = granted_cast_cost(game, player, option) {
        cost.mana
    } else if option.index == 0 {
        def.cost
    } else {
        return Vec::new();
    };
    let (_, face) = option_face(game, card, option.index);
    let mut copy = advice_game(game);
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    candidates.iter().map(|&t| change_total(&cx, &cost_change_sources(&cx), player, Costed::Spell(card), base, face, &[t])).collect()
}

/// For a spell being cast whose additional cost sacrifices any number of permanents for a reduction each (CR 601.2b,
/// `VariableCost::SacrificeAnyNumber`): the generic mana each takes off, and how many its caster could sacrifice, for the
/// announcement's prompt. None for any other spell.
pub fn sacrifices_for_reduction(game: &crate::Game, spell: ObjectId, player: PlayerId) -> Option<(u8, usize)> {
    let obj = game.state.obj(spell);
    let def = game.cards.get(obj.copied_card().unwrap_or(obj.card));
    let Some(variable @ VariableCost::SacrificeAnyNumber { less_each, .. }) = def.additional_cost.and_then(|c| c.variable) else { return None };
    let mut copy = advice_game(game);
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    Some((less_each, variable_candidates(&cx, spell, player, variable, Some(CostFor::Spell), None).len()))
}

/// What casting a card from a hand by the cast option `option` costs `player` before any target is announced, with the cost
/// changes in play applied (CR 601.2f): its mana cost, an alternative cost of its own (a way, overload), or a face's mana
/// cost (an Adventure, an Omen, a half, a back face, a bestowed Aura), with the mana of its additional cost (CR 118.8), what
/// increases and reduces the total (Thalia, Helm of Awakening) and what sets its least (Trinisphere); and the life the
/// changes add. For the menu row, so a cast says what it will cost before it is paid. `None` when nothing could change it
/// (no cost changer on the battlefield, none of the card's own), which is nearly always; for a cast another object gives the
/// cost or the permission of (`granted_cast_total` and `alternative_cast_total` say what those cost); and for a cost with
/// {X}, whose total waits on the X announced. The cost it starts from is returned with the total, so the row says something
/// only where the changes make a difference.
pub fn cast_total(game: &crate::Game, player: PlayerId, option: &ActionOption) -> Option<(ManaCost, ManaCost, u32)> {
    let card = option.source?;
    let index = option.index;
    if !game.state.obj(card).zone.is(ZoneKind::Hand) || option.granted_by.is_some() || option.paid_by.is_some() {
        return None;
    }
    let whole = game.cards.get(game.state.obj(card).card);
    let face = cast_face(whole, index);
    if !(spell_cost_changers(game) || !face.own_cost_changes.is_empty()) {
        return None;
    }
    let base = match index {
        WAY_OVERLOAD => whole.overload?,
        WAY_BESTOW => whole.bestow_cost,
        n if n > 0 && (n as usize) <= whole.ways.len() => whole.ways[n as usize - 1].cost.mana,
        _ => face.cost,
    };
    let base = add_cost(base, face.additional_cost.map_or(ManaCost::ZERO, |extra| extra.mana));
    if base.x > 0 {
        return None;
    }
    let mut copy = advice_game(game);
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    let face_id = (!std::ptr::eq(face, whole)).then(|| cx.card_id(face));
    let (total, life) = change_total(&cx, &cost_change_sources(&cx), player, Costed::Spell(card), base, face_id, &[]);
    Some((base, total, life))
}

/// What a cast by a way a permanent grants (`option`, made by a `GrantedWay` that is its cost) costs `player`
/// (`granted_cast_cost`), and its mana as the cost changes in play leave it before any target is announced (CR 601.2f): for
/// the menu row of a cast without paying the mana cost, which says what it still costs ({1} more under Thalia).
pub fn granted_cast_total(game: &crate::Game, player: PlayerId, option: &ActionOption) -> Option<(Cost, ManaCost)> {
    let card = option.source?;
    let _face_up = layers::judged_face_up(&game.state, card);
    let cost = granted_cast_cost(game, player, option)?;
    let (_, face) = option_face(game, card, option.index);
    let mut copy = advice_game(game);
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    let mana = change_with(&cx, &cost_change_sources(&cx), player, Costed::Spell(card), cost.mana, face, &[]);
    Some((cost, mana))
}

/// What a cast for an alternative cost another object gives (`option.paid_by`; CR 118.9, 601.2b) pays rather than the mana
/// cost of the face it casts - its mana and its energy - and the mana as the cost changes in play leave it before any target
/// is announced (CR 601.2f): for the menu row, which says what the cast still costs ({1} more under Thalia). `None` for any
/// other cast.
pub fn alternative_cast_total(game: &crate::Game, player: PlayerId, option: &ActionOption) -> Option<(Cost, ManaCost)> {
    let card = option.source?;
    let _face_up = layers::judged_face_up(&game.state, card);
    let alternative = named_alternative(&game.state, game.cards, option.paid_by?)?;
    let cost = alternative.what.cost.cost();
    let (_, face) = option_face(game, card, option.index);
    let mut copy = advice_game(game);
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    let total = change_with(&cx, &cost_change_sources(&cx), player, Costed::Spell(card), cost.mana, face, &[]);
    Some((cost, total))
}

/// What a cast by a way a permanent grants that is its cost (CR 604, 118.9: escape, mayhem, Iroh's flashback) costs `player`,
/// read for the face it casts as the offer and the cast read it (CR 709.3a), before cost changes. For the menu row, since a
/// way's text need not say it (Iroh's "the flashback cost is equal to that card's mana cost"). `None` for any other cast.
pub fn granted_cast_cost(game: &crate::Game, player: PlayerId, option: &ActionOption) -> Option<Cost> {
    let card = option.source?;
    let _face_up = layers::judged_face_up(&game.state, card);
    let Some(Through::Way(way)) = through(game, game.cards, player, card, option.granted_by) else { return None };
    let (_, face) = option_face(game, card, option.index);
    let mut copy = advice_game(game);
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    match face {
        Some(face) => (way.what.cost)(&AsFace { cx: &cx, spell: card, face }, &way.asker(), card, player),
        None => (way.what.cost)(&cx, &way.asker(), card, player),
    }
}

/// The cost changes the object `id` carries itself (CR 601.2f): its card's, or a copy's.
fn own_cost_changes(cx: &Cx, id: ObjectId) -> &'static [CostChange] {
    let obj = cx.s().obj(id);
    cx.card(obj.copied_card().unwrap_or(obj.card)).own_cost_changes
}

fn change_with(
    cx: &Cx,
    sources: &[ObjectId],
    player: PlayerId,
    costed: Costed,
    cost: ManaCost,
    face: Option<CardId>,
    targets: &[Target],
) -> ManaCost {
    change_total(cx, sources, player, costed, cost, face, targets).0
}

/// The cost changes applied (`change_with`), and the life they add (CR 118.8, 601.2f).
fn change_total(
    cx: &Cx,
    sources: &[ObjectId],
    player: PlayerId,
    costed: Costed,
    cost: ManaCost,
    face: Option<CardId>,
    targets: &[Target],
) -> (ManaCost, u32) {
    let (cost, life, least) = change_parts(cx, sources, player, costed, cost, face, targets);
    (at_least(cost, least), life)
}

/// The increases and reductions applied (CR 601.2f), the life the changes add (CR 118.8), and the least mana the total may
/// come to, which is applied last (`CostDelta::at_least`), once every reduction is. Where a reduction meets a hybrid symbol
/// not yet announced (`announcements`), the cost for the first announcement; a cast announces them first
/// (`announce_hybrids`), and the advice tries each (`change_any`).
fn change_parts(
    cx: &Cx,
    sources: &[ObjectId],
    player: PlayerId,
    costed: Costed,
    cost: ManaCost,
    face: Option<CardId>,
    targets: &[Target],
) -> (ManaCost, u32, u8) {
    change_parts_split(cx, sources, player, costed, cost, face, targets, &[])
}

/// `change_parts` with the reductions by hybrid mana split as `split` says (CR 118.7e, `split_reduction`).
#[allow(clippy::too_many_arguments)]
fn change_parts_split(
    cx: &Cx,
    sources: &[ObjectId],
    player: PlayerId,
    costed: Costed,
    cost: ManaCost,
    face: Option<CardId>,
    targets: &[Target],
    split: &[bool],
) -> (ManaCost, u32, u8) {
    let (more, less, life, least, discount) = deltas(cx, sources, player, costed, face, targets);
    let less = split_reduction(&less, split);
    let mut cost = cost;
    cost.generic = (cost.generic as u32 + more).min(255) as u8;
    let mut first = cost;
    any_announcement(&cost, &less, &mut |c| {
        first = c;
        true
    });
    (reduced(first, &less, discount), life, least)
}

/// Whether `covered` holds for `change_parts` at some announcement of the hybrid symbols a reduction meets (CR 601.2b,
/// 601.2f): each total the cost could come to, with the least it may come to, as the advice judges it.
#[allow(clippy::too_many_arguments)]
fn change_any(
    cx: &Cx,
    sources: &[ObjectId],
    player: PlayerId,
    costed: Costed,
    cost: ManaCost,
    face: Option<CardId>,
    targets: &[Target],
    announced: Option<&Obj>,
    covered: &mut dyn FnMut(ManaCost, u8) -> bool,
) -> bool {
    let (more, whole, _, least, discount) = deltas_as(cx, sources, player, costed, face, targets, announced);
    let mut cost = cost;
    cost.generic = (cost.generic as u32 + more).min(255) as u8;
    // Each way the payer could split a reduction by hybrid mana (CR 118.7e).
    let n = reduction_choices(&whole).min(8);
    (0u32..1 << n).any(|mask| {
        let split: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
        let less = split_reduction(&whole, &split);
        any_announcement(&cost, &less, &mut |c| covered(reduced(c, &less, discount), least))
    })
}

/// How many symbols of a reduction the payer splits as it is applied (CR 118.7e): its hybrid symbols, two-color and {2/C}.
fn reduction_choices(less: &ManaCost) -> usize {
    less.hybrid.iter().flatten().count() + less.twobrid.iter().map(|&n| n as usize).sum::<usize>()
}

/// A reduction with each of its hybrid symbols split into the half the payer chose (CR 118.7e): `split[i]`, for the `i`th of
/// them (`reduction_choices`, two-color symbols first), is the second half - of {U/R} {R}, of {2/R} the {2} of generic
/// mana - and an unchosen one, or one `split` does not reach, the first ({U}; {R}).
fn split_reduction(less: &ManaCost, split: &[bool]) -> ManaCost {
    if reduction_choices(less) == 0 {
        return *less;
    }
    let mut out = ManaCost { hybrid: [None; 6], twobrid: [0; 5], ..*less };
    let mut at = 0;
    for (a, b) in less.hybrid.iter().flatten() {
        let half = if split.get(at).copied().unwrap_or(false) { *b } else { *a };
        out.symbols[half as usize] = out.symbols[half as usize].saturating_add(1);
        at += 1;
    }
    for (i, &n) in less.twobrid.iter().enumerate() {
        for _ in 0..n {
            if split.get(at).copied().unwrap_or(false) {
                out.generic = out.generic.saturating_add(2);
            } else {
                let m = Color::ALL[i].mana() as usize;
                out.symbols[m] = out.symbols[m].saturating_add(1);
            }
            at += 1;
        }
    }
    out
}

/// What a reduction could take off of each type, its hybrid symbols counted at both halves: the types whose hybrid
/// symbols in the cost it could meet, however the payer splits it (`announce_hybrids`).
fn reduction_reach(less: &ManaCost) -> ManaCost {
    let mut out = ManaCost { hybrid: [None; 6], twobrid: [0; 5], ..*less };
    for (a, b) in less.hybrid.iter().flatten() {
        out.symbols[*a as usize] = out.symbols[*a as usize].saturating_add(1);
        out.symbols[*b as usize] = out.symbols[*b as usize].saturating_add(1);
    }
    for (i, &n) in less.twobrid.iter().enumerate() {
        let m = Color::ALL[i].mana() as usize;
        out.symbols[m] = out.symbols[m].saturating_add(n);
    }
    out
}

/// Whether `f` holds for some cost of `announcements`, without listing them where no reduction meets a hybrid symbol.
fn any_announcement(cost: &ManaCost, less: &ManaCost, f: &mut dyn FnMut(ManaCost) -> bool) -> bool {
    if cost.hybrid.iter().all(Option::is_none) || less.symbols.iter().chain(&less.phyrexian).all(|&n| n == 0) {
        return f(*cost);
    }
    announcements(cost, less).into_iter().any(f)
}

/// The reductions `less` and the "less to one" `discount` applied to a cost whose increases are (CR 601.2f).
fn reduced(cost: ManaCost, less: &ManaCost, discount: u32) -> ManaCost {
    let mut cost = reduce(cost, less);
    // "Can't reduce the mana in that cost to less than one mana": applied to the cost the other changes left (CR 601.2f).
    if discount > 0 {
        let room = cost.mana_value().saturating_sub(1);
        cost.generic -= discount.min(room).min(cost.generic as u32) as u8;
    }
    cost
}

/// What the cost changes come to (CR 601.2f): the generic mana they add, the mana they take off, the life they add
/// (CR 118.8), the least the total may come to (`CostDelta::at_least`), and the "less to one" reduction.
fn deltas(cx: &Cx, sources: &[ObjectId], player: PlayerId, costed: Costed, face: Option<CardId>, targets: &[Target]) -> (u32, ManaCost, u32, u8, u32) {
    deltas_as(cx, sources, player, costed, face, targets, None)
}

/// `deltas` with the spell as `announced` has it, where what comes before its targets is announced (`Announcing`): a change
/// may read its X or kicker (CR 601.2f).
fn deltas_as(
    cx: &Cx,
    sources: &[ObjectId],
    player: PlayerId,
    costed: Costed,
    face: Option<CardId>,
    targets: &[Target],
    announced: Option<&Obj>,
) -> (u32, ManaCost, u32, u8, u32) {
    deltas_watched(cx, sources, player, costed, face, targets, announced, None)
}

/// `deltas_as`, the cost changes reading the game through `watching` where it is given (`Watch`): whether any of them told
/// apart the objects it watches.
#[allow(clippy::too_many_arguments)]
fn deltas_watched(
    cx: &Cx,
    sources: &[ObjectId],
    player: PlayerId,
    costed: Costed,
    face: Option<CardId>,
    targets: &[Target],
    announced: Option<&Obj>,
    watching: Option<&Watching>,
) -> (u32, ManaCost, u32, u8, u32) {
    // The thing costed is its own source for the changes it carries itself (affinity, channel's reduction).
    let (id, ability) = match costed {
        Costed::Spell(id) => (id, None),
        // A special action is no activated ability (CR 116.1): no discount on abilities, and no ability's own change.
        Costed::SpecialAction(id, _) => (id, None),
        Costed::Ability(id, ability, by) => (id, Some((ability, by))),
    };
    // A card's own changes are its spell's ("this spell costs {3} less ..."), never a special action's (CR 116.1): plotting
    // or foretelling the card is not made cheaper by what makes the spell cheaper.
    let own: &[CostChange] = match (face, costed) {
        (_, Costed::SpecialAction(..)) => &[],
        (Some(face), _) => cx.card(face).own_cost_changes,
        (None, _) => own_cost_changes(cx, id),
    };
    // The ability's own change ("this ability costs ... less", power-up), asked for this ability alone.
    let ability_own = ability.and_then(|(a, by)| activated(cx, a).and_then(|d| d.cost_change).map(|change| (change, by)));
    // The cost changes resolved spells and abilities made (CR 611.2c): of spells only, as every one printed says.
    let lasting = matches!(costed, Costed::Spell(_)) && lasting_changes(cx);
    if sources.is_empty() && own.is_empty() && ability_own.is_none() && !lasting {
        return (0, ManaCost::ZERO, 0, 0, 0);
    }
    let view;
    let as_face: &dyn Query = match (face, costed) {
        (Some(face), Costed::Spell(spell)) => {
            view = AsFace { cx, spell, face };
            &view
        }
        _ => cx,
    };
    // A change reads the characteristics of the spell it would be on the stack (`SpellsAsCast`): Herald's Horn's "creature
    // spells you cast of the chosen type" finds an opponent's creature card the caster's Arcane Adaptation makes that type as
    // their spell.
    let as_spell = match costed {
        Costed::Spell(spell) => spell_chars(cx, spell, face).map(|chars| AsSpell { q: as_face, spell, chars }),
        _ => None,
    };
    let as_face: &dyn Query = match &as_spell {
        Some(view) => view,
        None => as_face,
    };
    let patched;
    let announced_q: &dyn Query = match (announced, costed) {
        (Some(obj), Costed::Spell(spell)) => {
            patched = Announcing { q: as_face, spell, obj: *obj };
            &patched
        }
        _ => as_face,
    };
    let watch;
    let q: &dyn Query = match watching {
        Some(w) => {
            watch = Watch { q: announced_q, w, spell: match costed {
                Costed::Spell(id) => Some(id),
                _ => None,
            } };
            &watch
        }
        None => announced_q,
    };
    let mut more = 0u32;
    let mut less = ManaCost::ZERO;
    let mut life = 0u32;
    let mut least = 0u8;
    let mut discount = 0u32;
    let mut add = |delta: CostDelta| {
        more += delta.more as u32;
        less = less.plus(&delta.less);
        life += delta.life as u32;
        least = least.max(delta.at_least);
        discount += delta.less_to_one as u32;
    };
    for change in own {
        add(change(q, id, player, costed, targets));
    }
    // Asked as the ability it is (CR 607.5).
    if let Some((change, by)) = ability_own {
        add(change(q, &Asker { source: id, controller: player, by, spell: false }, costed, targets));
    }
    for &source in sources {
        let obj = cx.s().obj(source);
        let def = cx.card(cx.cards().current(obj));
        for change in def.cost_changes {
            add(change(q, source, player, costed, targets));
        }
    }
    // "[Kind of] spells you cast this turn cost {1} less" (CR 611.2c, 601.2f): each such effect about the caster, whatever has
    // become of its source, asks the card that made it what it does to this spell.
    if lasting {
        for e in &cx.s().effects {
            if let EffectKind::SpellCostChange { card, index, player: whose } = e.kind
                && whose == player
            {
                let def = &cx.card(card).lasting_cost_changes[index as usize];
                add((def.change)(q, e.source, player, costed, targets));
            }
        }
    }
    // Increases, then reductions (CR 601.2f), applied by the caller. Reductions of generic and colored mana commute, so the
    // order the player could apply them in changes nothing.
    (more, less, life, least, discount)
}

/// The two-color hybrid symbols of `cost` a reduction `less` meets: a symbol with a half of a type whose reduction the
/// cost's plain symbols of that type do not absorb (CR 118.7c), by slot, with the types of its halves so met. How such a
/// symbol was announced (CR 601.2b) decides whether the reduction takes it off or takes generic mana instead.
fn met_hybrids(cost: &ManaCost, less: &ManaCost) -> Vec<(usize, Vec<Mana>)> {
    let left = |m: Mana| reduction_of_type(less, m).saturating_sub(u32::from(cost.symbols[m as usize]));
    cost.hybrid
        .iter()
        .enumerate()
        .filter_map(|(slot, h)| {
            let (a, b) = (*h)?;
            let halves: Vec<Mana> = [a, b].into_iter().filter(|&m| left(m) > 0).collect();
            (!halves.is_empty()).then_some((slot, halves))
        })
        .collect()
}

/// How much mana of type `m` a reduction takes off (CR 118.7c-f): its plain symbols of that type, and its Phyrexian ones
/// of that color.
fn reduction_of_type(less: &ManaCost, m: Mana) -> u32 {
    let i = m as usize;
    u32::from(less.symbols[i]) + if i < 5 { u32::from(less.phyrexian[i]) } else { 0 }
}

/// The ways the hybrid symbols a reduction meets (`met_hybrids`) could be announced (CR 601.2b), as costs: each such symbol
/// left a hybrid symbol - announced as a half the reduction then does not take, which takes generic mana instead - or
/// announced as a half the reduction takes, no more of a type than the reduction has of it. Each is a cost the cast could
/// come to; with no symbol met, the cost itself.
fn announcements(cost: &ManaCost, less: &ManaCost) -> Vec<ManaCost> {
    let met = met_hybrids(cost, less);
    if met.is_empty() {
        return vec![*cost];
    }
    fn go(less: &ManaCost, met: &[(usize, Vec<Mana>)], at: usize, now: ManaCost, out: &mut Vec<ManaCost>) {
        let Some((slot, halves)) = met.get(at) else {
            let mut done = now;
            let kept: Vec<Option<(Mana, Mana)>> = done.hybrid.iter().copied().filter(Option::is_some).collect();
            done.hybrid = [None; 6];
            done.hybrid[..kept.len()].copy_from_slice(&kept);
            out.push(done);
            return;
        };
        for &m in halves {
            if u32::from(now.symbols[m as usize]) < reduction_of_type(less, m) {
                let mut next = now;
                next.hybrid[*slot] = None;
                next.symbols[m as usize] += 1;
                go(less, met, at + 1, next, out);
            }
        }
        go(less, met, at + 1, now, out);
    }
    let mut out = Vec::new();
    go(less, &met, 0, *cost, &mut out);
    out
}

/// A cost reduced by an amount of mana (CR 118.7): its generic part takes generic mana off only (118.7a); each colored
/// or colorless symbol takes off one mana of that type, and one the cost has none of (118.7b), or has fewer of (118.7c-d),
/// takes off generic mana instead; a Phyrexian symbol takes off mana of its color (118.7f). Nothing goes below zero
/// (CR 601.2f). A hybrid symbol of the cost is announced as one of its halves before the reductions (CR 601.2b): one a
/// reduction meets is announced first (`announce_hybrids`, `announcements`), and one still a hybrid symbol here meets none.
fn reduce(mut cost: ManaCost, by: &ManaCost) -> ManaCost {
    // A hybrid symbol in a reduction is split by the payer as the reduction is applied (CR 118.7e): the callers split it
    // (`split_reduction`); one that reaches here unsplit is taken at its first half.
    let by = &split_reduction(by, &[]);
    let mut excess = by.generic as u32;
    for i in 0..6 {
        let mut n = by.symbols[i] as u32;
        if i < 5 {
            n += by.phyrexian[i] as u32;
        }
        let off = n.min(cost.symbols[i] as u32);
        cost.symbols[i] -= off as u8;
        excess += n - off;
    }
    cost.generic = (cost.generic as u32).saturating_sub(excess) as u8;
    cost
}

/// The least mana a spell could be cast for with this base cost, for the advice that a cast that cannot be paid is
/// not offered (CR 601.2, 118.3): X at the least it may be (0, or 1 where it can't be 0), no kicker, the cheapest modes it must choose, Phyrexian symbols paid with
/// life where the life total allows. `targets` are the candidate it would be announced with, where its target is announced
/// with the cast, or none (CR 601.2c, 601.2f). `earlier` is what is announced before them where it is not the cheapest
/// (`Earlier::NONE`): the modes, X, kicker, optional cost and splices of an announcement whose targets are judged with it.
///
/// One for each way the hybrid symbols a reduction meets could be announced (`announcements`): the cast may be paid if any is.
#[allow(clippy::too_many_arguments)]
fn least_cost(
    cx: &Cx,
    available: &Available,
    player: PlayerId,
    card: ObjectId,
    def: &CardDef,
    base: ManaCost,
    targets: &[Target],
    earlier: &Earlier,
    covered: &mut dyn FnMut(ManaCost) -> bool,
) -> bool {
    least_cost_parts(cx, available, player, card, def, base, targets, earlier, &mut |cost, least| covered(at_least(cost, least)))
}

/// `least_cost` with the least the total may come to (`CostDelta::at_least`, Trinisphere) not yet applied but returned, for a
/// cast whose own reductions (emerge, harmonize) are still to come: the least is applied after every reduction, as the cast
/// applies it (CR 601.2f).
#[allow(clippy::too_many_arguments)]
fn least_cost_parts(
    cx: &Cx,
    available: &Available,
    player: PlayerId,
    card: ObjectId,
    def: &CardDef,
    base: ManaCost,
    targets: &[Target],
    earlier: &Earlier,
    covered: &mut dyn FnMut(ManaCost, u8) -> bool,
) -> bool {
    // "X can't be 0" (CR 107.3a): X at 1, and no cast at all for a cost where X could only be 0 (CR 107.3b), as `cast`
    // refuses it.
    if x_only_zero(def.x_cant_be_zero, def.cost.x, base.x) {
        return false;
    }
    let mut cost = at_least_x(base, def.x_cant_be_zero);
    // The X announced, where it is more than the least (CR 107.3a, 601.2b).
    let more_x = u32::from(earlier.x.saturating_sub(least_x(def.x_cant_be_zero)));
    cost.generic = cost.generic.saturating_add((more_x * u32::from(base.x)).min(255) as u8);
    // The kicker, the optional additional cost and the cards spliced onto it, where they are announced (CR 601.2b, 601.2f).
    if let Some(k) = def.kicker.filter(|_| earlier.kicks > 0) {
        cost = kicker_total(cost, k, earlier.kicks);
    }
    if let Some(optional) = def.optional_cost.filter(|_| earlier.paid) {
        cost = add_cost(cost, optional.cost.mana);
    }
    cost = add_cost(cost, earlier.splice);
    if let Some(extra) = def.additional_cost {
        cost = add_cost(cost, extra.mana);
    }
    // CR 601.2b: an additional cost that is one of several ("behold a Jace or pay {1}") costs at least the mana of the
    // cheapest of them that can be paid otherwise.
    if let Some(cheapest) = def
        .additional_choice
        .iter()
        .filter(|c| can_pay_other(cx, card, player, &c.cost))
        .map(|c| c.cost.mana)
        .min_by_key(|m| m.mana_value())
    {
        cost = add_cost(cost, cheapest);
    }
    if let (Some(modal), Some(mask)) = (def.modal, earlier.modes) {
        // The modes announced, with their costs (CR 601.2b, 702.172a, 702.120a).
        cost = modal_cost(modal, mask, cost);
    } else if let Some(modal) = def.modal {
        let mut costs: Vec<ManaCost> = modal.modes.iter().map(|m| m.cost).collect();
        costs.sort_by_key(|c| c.mana_value());
        // One mode to choose: the cheapest the player could pay for - Three Steps Ahead's {3} when its {1}{U} would need
        // a second blue.
        if modal.min <= 1
            && let Some(i) = costs.iter().position(|&c| {
                finish(cx, available, player, card, def, add_cost(cost, c), targets, earlier.spell.as_ref(), &mut |total, least| spell_covers(cx, available, player, card, def, &at_least(total, least)))
            })
        {
            costs.swap(0, i);
        }
        for c in costs.into_iter().take(modal.min.max(1) as usize) {
            cost = add_cost(cost, c);
        }
    }
    // "Costs {1} less for each permanent sacrificed this way": as little as sacrificing every permanent that could be.
    let off = match def.additional_cost.and_then(|c| c.variable) {
        Some(v @ VariableCost::SacrificeAnyNumber { less_each, .. }) => {
            (variable_candidates(cx, card, player, v, Some(CostFor::Spell), None).len() as u32 * less_each as u32).min(255) as u8
        }
        _ => 0,
    };
    // "Costs {2} less for each card exiled this way": as little as exiling every card that could be.
    let off = match def.additional_cost.and_then(|c| c.exile_from_hand_for_less) {
        Some(e) => off.saturating_add((hand_exiles_for_less(cx, card, player, e, None).len() as u32 * u32::from(e.less_each)).min(255) as u8),
        None => off,
    };
    finish(cx, available, player, card, def, cost, targets, earlier.spell.as_ref(), &mut |mut cost, least| {
        cost.generic = cost.generic.saturating_sub(off);
        covered(cost, least)
    })
}

/// `least_cost`'s Phyrexian symbols paid with life where the life total allows, and the cost changes applied.
/// Whether `covered` holds for one of the totals (with the least each may come to, not yet applied).
#[allow(clippy::too_many_arguments)]
fn finish(
    cx: &Cx,
    available: &Available,
    player: PlayerId,
    card: ObjectId,
    def: &CardDef,
    mut cost: ManaCost,
    targets: &[Target],
    announced: Option<&Obj>,
    covered: &mut dyn FnMut(ManaCost, u8) -> bool,
) -> bool {
    cost = phyrexian_with_life(cx, player, cost, 0, Some(CostFor::Spell));
    // A face cast from the hand (an Adventure, an Omen, a half, a back face) is judged as that face (CR 715.4, 720.3a,
    // 709.3, 712.8a).
    change_any(cx, &available.changers, player, Costed::Spell(card), cost, face_of(cx, card, def), targets, announced, covered)
}

/// `cost` with as many of its Phyrexian symbols paid with 2 life each as `player`'s life total allows beside `other_life`
/// the rest of the cost pays (CR 107.4f, 119.4: a payment of life the life total covers, down to 0), and the others paid
/// with their color: the least mana it could be announced to cost (advice; `announce_life` asks the real choice). `paid` is
/// what the cost is paid for (`payable_life_for`).
fn phyrexian_with_life(cx: &Cx, player: PlayerId, mut cost: ManaCost, other_life: u32, paid: Option<CostFor>) -> ManaCost {
    if cost.phyrexian == [0; 5] {
        return cost;
    }
    let mut life = i64::from(payable_life_for(cx, player, paid)) - i64::from(other_life);
    for (i, n) in cost.phyrexian.into_iter().enumerate() {
        for _ in 0..n {
            if life >= 2 {
                life -= 2;
            } else {
                cost.symbols[Color::ALL[i].mana() as usize] = cost.symbols[Color::ALL[i].mana() as usize].saturating_add(1);
            }
        }
    }
    cost.phyrexian = [0; 5];
    cost
}

/// Whether `player` could pay `cost` from the mana pool and what the mana abilities they could activate add (advice,
/// CR 601.2g): a mana ability whose yield is not stated is taken to be able to pay anything.
///
/// `source` is what the cost is for: a spell on the stack may be paid with the restricted mana its kind allows, anything
/// else with the restricted mana an ability may spend (CR 106.6); and the spell with mana spent as though it were mana of
/// any color where a static ability lets `player` do so to cast it (CR 609.4b).
pub fn can_afford(cx: &Cx, player: PlayerId, source: ObjectId, cost: &ManaCost) -> bool {
    if cost.mana_value() == 0 {
        return true;
    }
    let a = available(cx, player);
    let chars = cx.chars(source);
    if cx.s().obj(source).zone.is(ZoneKind::Stack) {
        let any_color = !a.any_color
            && !a.spenders.is_empty()
            && static_rules::lets_spend_any_color(cx, &a.spenders, &Spending { player, source: Some(source), purpose: SpendingFor::Casting });
        a.covers_for(&if any_color { cost.paid_with_any_color() } else { *cost }, &spell_payment(cx, source, a.names_subtypes()))
    } else {
        a.covers_for(cost, &ability_payment(cx, source, chars.types, chars.subtypes))
    }
}

/// The mana a player has to pay with: the pool and what their mana abilities add, summed once for every cost judged.
pub struct Available {
    fixed: [u32; 6],
    /// Mana of a type to be chosen, by the mana types it may be (a bit for each `Mana`, W U B R G C): a land with
    /// "{T}: Add {W}" and "{T}: Add {R}" is one mana that is white or red, never blue.
    flex: [u32; 64],
    /// Some mana ability's yield is not stated: anything may be payable.
    unknown: bool,
    /// The permanents that change what things cost.
    changers: Vec<ObjectId>,
    /// The permanents that give spells convoke, delve or improvise (`CardDef::grants_pay_with`).
    grantors: Vec<ObjectId>,
    /// Restricted mana the pool and the mana sources hold, one entry per restriction (CR 106.6): its mana of stated kinds,
    /// and its mana of any one color a mana ability adds (Great Hall of the Biblioplex, Delighted Halfling). Each pays
    /// only what `Available::covers_for` says it may. Mana that pays anything (`GainsHaste`) is in `fixed`.
    restricted: Vec<(ManaRestriction, [u32; 6], u32)>,
    /// The player may spend mana as though it were mana of any color for every payment (CR 609.4b: Mycosynth Lattice).
    any_color: bool,
    /// The permanents whose static abilities may let the player spend mana as though it were mana of any color for some
    /// payments (CR 609.4b: Oath of Nissa's planeswalker spells), asked of each spell judged (`spell_covers`). Empty in nearly
    /// every game.
    spenders: Vec<(ObjectId, PlayerId, &'static CardDef)>,
    /// Mana that comes in groups of one type chosen once, by the mana types it may be and how many: a tap whose type the
    /// player chooses (Shivan Reef's {U} or {R}) and the "one mana of any type that land produced" it triggers (Mana Flare)
    /// are one choice, {U}{U} or {R}{R} and never {U}{R} (CR 106.12a, 605.1b). Empty unless such a trigger waits.
    linked: Vec<(u8, u32)>,
    /// The mana sources whose yield is a choice the independent counts above can't say exactly - a choice between abilities
    /// that make different amounts or kinds (Shivan Reef's {C} or {U}/{R} with Mana Flare out; a land with a restricted
    /// and an unrestricted ability), "three mana of any one color" (Black Lotus), an ability with a mana cost of its own
    /// (Chromatic Star's {1}, paid before what it adds is there) - each with how many alike: whether a cost is covered is
    /// then found by trying each one's choices (`Available::exact`). Empty on most boards.
    complex: Vec<(Vec<Alternative>, u32)>,
    /// The permanents a mana ability that can be activated again would spend (`Repeat`: sacrifice or tap): one whose own {T}
    /// is in a cost being paid stays for it, so its mana is worked out without it (`available_without`).
    spent: Vec<ObjectId>,
}

/// One mana ability a source could be activated for, as the advice counts it: what it adds - fixed mana, and `k` mana of
/// one type chosen from `types` - and what restricts it, the mana it costs (paid as activating an ability of `source`,
/// CR 602.2b, 605.3a), and what the triggered mana abilities its tap would trigger add (`extra`, and `produced` mana of one
/// of the types the activation produced, CR 106.12a).
#[derive(Clone, Copy, PartialEq, Eq)]
struct Alternative {
    fixed: [u32; 6],
    k: u32,
    types: u8,
    restriction: Option<ManaRestriction>,
    cost: ManaCost,
    source: ManaPayment,
    extra: [u32; 6],
    produced: u32,
    /// Its cost has {T} (CR 107.5): it taps its source, as an ability that taps an untapped permanent would.
    tapped: bool,
}

impl Alternative {
    /// The mana it adds (before what its tap triggers) with `chosen` the type of its `k`.
    fn made(&self, chosen: Option<usize>) -> [u32; 6] {
        let mut made = self.fixed;
        if let Some(m) = chosen {
            made[m] = made[m].saturating_add(self.k);
        }
        made
    }

    /// The mana types it adds with `chosen` the type of its `k`.
    fn produces(&self, chosen: Option<usize>) -> u8 {
        let mut types = (0..6).filter(|&m| self.fixed[m] > 0).fold(0u8, |t, m| t | 1 << m);
        if let Some(m) = chosen.filter(|_| self.k > 0) {
            types |= 1 << m;
        }
        types
    }
}

/// Adds restricted mana to the entry of its restriction in `Available::restricted`.
fn add_available(list: &mut Vec<(ManaRestriction, [u32; 6], u32)>, restriction: ManaRestriction, fixed: [u32; 6], flex: u32) {
    let at = match list.iter().position(|(r, ..)| *r == restriction) {
        Some(at) => at,
        None => {
            list.push((restriction, [0; 6], 0));
            list.len() - 1
        }
    };
    for m in 0..6 {
        list[at].1[m] = list[at].1[m].saturating_add(fixed[m]);
    }
    list[at].2 = list[at].2.saturating_add(flex);
}

/// Mana that can pay a cost after its announcement (CR 601.2g, 605.3a): priority-only sources must be floated first.
pub fn available(cx: &Cx, player: PlayerId) -> Available {
    available_without(cx, player, None, ManaWindow::Payment)
}

/// What `player` has to pay with, `skip` apart: a permanent whose own {T} is in the cost being paid cannot also be
/// tapped for mana to pay it (CR 602.2b, 605.3b).
fn available_without(cx: &Cx, player: PlayerId, skip: Option<ObjectId>, window: ManaWindow) -> Available {
    available_as(cx, player, skip, window, false)
}

/// `available_without`. `defer`: each source's own abilities are kept until those that could tap it are known
/// (`Repeat::Tap`); without it they are counted as they are found, and finding such an ability starts over with it - so a
/// board with none pays nothing for them.
fn available_as(cx: &Cx, player: PlayerId, skip: Option<ObjectId>, window: ManaWindow, defer: bool) -> Available {
    let mut fixed = cx.s().player(player).pool.0;
    let mut restricted: Vec<(ManaRestriction, [u32; 6], u32)> = Vec::new();
    for (restriction, pool) in cx.s().restricted_of(player) {
        if restriction == ManaRestriction::GainsHaste {
            // Mana that pays anything is not a limit (CR 106.6): it is spent as ordinary mana is.
            for m in 0..6 {
                fixed[m] += pool.0[m];
            }
        } else {
            add_available(&mut restricted, restriction, pool.0, 0);
        }
    }
    let mut flex = [0u32; 64];
    // Mana abilities of the player's own (CR 605.1a): paid with life, as often as the life total allows (CR 119.4), and never
    // while the player can't lose life (CR 119.8) or can't pay life to activate mana abilities (`payable_life_for`).
    for effect in cx.s().effects.iter().filter(|e| matches!(e.kind, EffectKind::PlayerManaAbility(_)) && e.controller == player) {
        let EffectKind::PlayerManaAbility(ability) = effect.kind else { continue };
        if let Some(def) = activated(cx, ability)
            && window.allows(def)
            && def.cost.life > 0
        {
            let times = payable_life_for(cx, player, Some(CostFor::ManaAbility)).max(0) as u32 / def.cost.life;
            for m in 0..6 {
                fixed[m] = fixed[m].saturating_add(def.adds.0[m].saturating_mul(times));
            }
        }
    }
    let forbidders = static_rules::activation_forbidders(cx);
    // The triggered mana abilities waiting on a tap (CR 605.1b): nearly always none, and then nothing more is asked.
    let waiting = waiting_mana_triggers(cx);
    // A replacement effect that could change what a permanent tapped for mana produces (CR 106.12b): nearly always none.
    let replaceable = crate::replace::mana_may_be_replaced(cx);
    let mut linked: Vec<(u8, u32)> = Vec::new();
    let mut complex: Vec<(Vec<Alternative>, u32)> = Vec::new();
    // Each source's mana abilities, one list kept for all of them.
    let mut alternatives: Vec<Alternative> = Vec::new();
    // The mana abilities without {T} that may be activated again for as long as their cost can be paid (CR 602.2, 605):
    // each with the source and what one activation spends (`Repeat`).
    // Each with the ability as it is asked (`Asker::by`).
    let mut repeatable: Vec<(Alternative, ObjectId, Repeat, Option<Linked>)> = Vec::new();
    // Each source's own mana abilities, kept until the repeatable ones that could tap it are known (`Repeat::Tap`).
    // Kept in one list, each source's a run of it: no list of its own for each source.
    let mut kept: Vec<Alternative> = Vec::new();
    let mut own: Vec<(ObjectId, usize, usize)> = Vec::new();
    let mut spent: Vec<ObjectId> = Vec::new();
    for &id in cx.s().battlefield() {
        if cx.s().obj(id).controller != player || skip == Some(id) {
            continue;
        }
        let chars = cx.chars(id);
        // Each mana ability the source could be activated for now: alternatives, of which one is (CR 605.3b).
        alternatives.clear();
        let given = layers::acquisition_possible(cx.cards(), cx.s());
        for (index, ability) in chars.abilities().enumerate() {
            // Asked as the ability it is (CR 607.5): one an effect gave the source reads only its own links.
            let by = if given { cx.link_nth(id, &chars, index) } else { None };
            let Some(def) = activated(cx, ability) else { continue };
            if !def.is_mana_ability()
                || !window.allows(def)
                || def.from != ZoneKind::Battlefield
                || limit_reached(cx, id, &chars, index, def)
                || !can_pay_activation(cx, id, player, &def.cost, Some(CostFor::ManaAbility), by)
                || static_rules::activation_barred(cx, &forbidders, id, def)
            {
                continue;
            }
            // "Activate only if ..." (CR 602.5b): a Verge's second color is not there without its land type.
            if let Some(condition) = def.condition
                && !condition(cx, &Asker { source: id, controller: player, by, spell: false })
            {
                continue;
            }
            // A yield not stated, a cost in mana paid in a way announced (Phyrexian, {2/W}), or a cost of tapped permanents
            // (chosen once its mana is paid, and paid again for each, CR 601.2g-h), is not judged.
            if def.adds.0.iter().all(|&n| n == 0) && def.adds_any == 0
                || def.cost.mana.phyrexian != [0; 5]
                || def.cost.mana.twobrid != [0; 5]
                || def.cost.permanents.is_some_and(|p| p.tapped)
            {
                return Available {
                    fixed,
                    flex,
                    unknown: true,
                    changers: cost_change_sources(cx),
                    grantors: pay_with_grantors(cx),
                    restricted,
                    any_color: false,
                    spenders: Vec::new(),
                    linked,
                    complex: Vec::new(),
                    spent: Vec::new(),
                };
            }
            let mut alternative = Alternative {
                fixed: def.adds.0,
                k: u32::from(def.adds_any),
                types: if def.adds_any > 0 { any_types(cx, id, player, def, by) } else { 0 },
                restriction: def.restricted.filter(|&r| r != ManaRestriction::GainsHaste).map(|r| restriction_from(cx, id, r, by)),
                cost: ManaCost { x: 0, ..def.cost.mana },
                source: ability_payment(cx, id, chars.types, chars.subtypes),
                extra: [0; 6],
                produced: 0,
                tapped: false,
            };
            // Mana of a type chosen from none is no mana (Thespian's Stage copying Sunken Citadel has no chosen color).
            if alternative.types == 0 {
                alternative.k = 0;
            }
            alternative.tapped = def.cost.tap;
            // What the replacement effects on its mana would leave of it (CR 106.12b), where any would change it: one
            // alternative for each outcome, with the triggers its tap would trigger judged on that (CR 106.12a).
            if replaceable
                && def.cost.tap
                && let Some(outcomes) = replaced_yields(
                    cx,
                    id,
                    player,
                    alternative.fixed,
                    alternative.k,
                    alternative.types,
                    def.restricted.map(|r| restriction_from(cx, id, r, by)),
                )
            {
                for o in outcomes {
                    let mut replaced = Alternative {
                        fixed: o.mana.0,
                        k: 0,
                        types: 0,
                        restriction: o.restriction.filter(|&r| r != ManaRestriction::GainsHaste),
                        ..alternative
                    };
                    let types = replaced.produces(None);
                    if !waiting.is_empty() && types != 0 {
                        (replaced.extra, replaced.produced) = tap_triggers(cx, &waiting, id, player, types);
                    }
                    if !alternatives.contains(&replaced) {
                        alternatives.push(replaced);
                    }
                }
                continue;
            }
            // Only a mana ability with {T} in its cost taps the permanent for mana (CR 106.12), and so triggers what waits on
            // a tap.
            if !waiting.is_empty() && def.cost.tap {
                let types = alternative.produces(None) | alternative.types;
                if types != 0 {
                    (alternative.extra, alternative.produced) = tap_triggers(cx, &waiting, id, player, types);
                }
            }
            if let Some(repeat) = Repeat::of(&def.cost, def.colors_from_sacrifice) {
                if !defer && matches!(repeat, Repeat::Tap(..)) {
                    return available_as(cx, player, skip, window, true);
                }
                repeatable.push((alternative, id, repeat, by));
                continue;
            }
            alternatives.push(alternative);
        }
        if !defer {
            account(&alternatives, 1, &mut fixed, &mut flex, &mut restricted, &mut linked, &mut complex);
        } else if !alternatives.is_empty() {
            own.push((id, kept.len(), kept.len() + alternatives.len()));
            kept.extend_from_slice(&alternatives);
        }
    }
    // An ability that taps an untapped permanent of its controller's (Urza, Lord High Artificer's "Tap an untapped artifact
    // you control: Add {U}") is activated once for each such permanent, and taps it as the permanent's own {T} would: each
    // is a source whose alternatives are its own {T} mana abilities and those that could tap it - one of them, never both. A
    // permanent with a mana ability without {T} keeps that apart, the tap counted on its own.
    let taps: Vec<(Alternative, ObjectId, Cost, PermanentCost, Option<Linked>)> = repeatable
        .iter()
        .filter_map(|&(a, source, repeat, by)| match repeat {
            Repeat::Tap(cost, p) => Some((a, source, cost, p, by)),
            _ => None,
        })
        .collect();
    let tapping = |target: ObjectId| -> Vec<Alternative> {
        if cx.s().obj(target).tapped {
            return Vec::new();
        }
        taps.iter()
            .filter(|(_, source, cost, p, by)| pays_permanents(cx, cost, p, &Asker { source: *source, controller: player, by: *by, spell: false }, target, Some(CostFor::ManaAbility)))
            .map(|&(a, ..)| a)
            .collect()
    };
    for &(id, from, to) in &own {
        let alternatives = &kept[from..to];
        if !taps.is_empty() && alternatives.iter().all(|a| a.tapped) {
            let outlets = tapping(id);
            if !outlets.is_empty() {
                spent.push(id);
                let both: Vec<Alternative> = alternatives.iter().copied().chain(outlets).collect();
                account(&both, 1, &mut fixed, &mut flex, &mut restricted, &mut linked, &mut complex);
                continue;
            }
        }
        account(alternatives, 1, &mut fixed, &mut flex, &mut restricted, &mut linked, &mut complex);
    }
    if !taps.is_empty() {
        for target in cx.permanents_of(player) {
            if skip == Some(target) || own.iter().any(|&(id, from, to)| id == target && kept[from..to].iter().all(|a| a.tapped)) {
                continue;
            }
            let outlets = tapping(target);
            if !outlets.is_empty() {
                spent.push(target);
                account(&outlets, 1, &mut fixed, &mut flex, &mut restricted, &mut linked, &mut complex);
            }
        }
    }
    // A repeatable ability is activated once for each thing its cost spends: each permanent it could sacrifice is a source of
    // its own, whose alternatives are the abilities that could sacrifice it - so one sacrificed to one is not to another -
    // and each counter it could remove one activation. A permanent's own mana abilities are judged apart: it may be tapped
    // for mana and then sacrificed.
    if !repeatable.is_empty() {
        let mut units: Vec<(Vec<Alternative>, u32)> = Vec::new();
        for target in cx.permanents_of(player) {
            // A permanent whose own {T} is in the cost being paid stays to be tapped for it (CR 602.2b, 605.3b).
            if skip == Some(target) {
                continue;
            }
            let outlets: Vec<Alternative> = repeatable
                .iter()
                .filter_map(|&(a, source, repeat, by)| match repeat {
                    Repeat::Sacrifice(cost, p, from_it) if pays_permanents(cx, &cost, &p, &Asker { source, controller: player, by, spell: false }, target, Some(CostFor::ManaAbility)) => {
                        if !from_it {
                            return Some(a);
                        }
                        // The types this land could produce (CR 106.7), not every land's.
                        let (colors, colorless) = producible_types(cx, target);
                        let types = colors.0 | if colorless { 1 << Mana::C as u8 } else { 0 };
                        Some(Alternative { types, k: if types == 0 { 0 } else { a.k }, ..a })
                    }
                    _ => None,
                })
                .collect();
            if outlets.is_empty() {
                continue;
            }
            spent.push(target);
            match units.iter_mut().find(|(alike, _)| *alike == outlets) {
                Some((_, n)) => *n += 1,
                None => units.push((outlets, 1)),
            }
        }
        for &(a, source, repeat, _) in &repeatable {
            if let Repeat::Counters(kind, each) = repeat {
                let times = cx.s().obj(source).counters(kind).max(0) as u32 / u32::from(each.max(1));
                if times > 0 {
                    units.push((vec![a], times));
                }
            }
        }
        for (outlets, n) in units {
            account(&outlets, n, &mut fixed, &mut flex, &mut restricted, &mut linked, &mut complex);
        }
    }
    let spenders = static_rules::any_color_spenders(cx);
    Available {
        fixed,
        flex,
        unknown: false,
        changers: cost_change_sources(cx),
        grantors: pay_with_grantors(cx),
        restricted,
        any_color: static_rules::spend_any_color_always(cx, &spenders, player),
        spenders,
        linked,
        complex,
        spent,
    }
}

/// What one activation of a mana ability without {T} spends, where its cost is that alone and so may be paid again for as
/// long as there is more of it (CR 602.2): a permanent sacrificed (Ashnod's Altar, Phyrexian Altar, Krark-Clan Ironworks),
/// an untapped permanent tapped (Urza, Lord High Artificer), or counters removed from its source (Cryptic Trilobite).
#[derive(Clone, Copy)]
enum Repeat {
    /// With whether it adds a type the sacrificed permanent could produce (Squandered Resources, CR 106.7), which is then
    /// that permanent's own.
    Sacrifice(Cost, PermanentCost, bool),
    Tap(Cost, PermanentCost),
    Counters(CounterKind, u8),
}

impl Repeat {
    fn of(cost: &Cost, colors_from_sacrifice: bool) -> Option<Repeat> {
        if cost.tap {
            return None;
        }
        match (cost.permanents, cost.remove_counters) {
            (Some(p), None) if p.count == 1 && p.action == PermanentAction::Sacrifice && !p.tapped && Cost { permanents: None, tap: true, ..*cost }.is_tap_only() => {
                Some(Repeat::Sacrifice(*cost, p, colors_from_sacrifice))
            }
            (Some(p), None)
                if p.count == 1
                    && cost.tap_permanents
                    && !p.tapped
                    && Cost { permanents: None, tap_permanents: false, tap: true, ..*cost }.is_tap_only() =>
            {
                Some(Repeat::Tap(*cost, p))
            }
            (None, Some((kind, n))) if n > 0 && Cost { remove_counters: None, tap: true, ..*cost }.is_tap_only() => Some(Repeat::Counters(kind, n)),
            _ => None,
        }
    }
}

/// Adds `n` sources alike, each one of `alternatives`, to what a player has to pay with (`available_without`).
#[inline]
fn account(
    alternatives: &[Alternative],
    n: u32,
    fixed: &mut [u32; 6],
    flex: &mut [u32; 64],
    restricted: &mut Vec<(ManaRestriction, [u32; 6], u32)>,
    linked: &mut Vec<(u8, u32)>,
    complex: &mut Vec<(Vec<Alternative>, u32)>,
) {
    if n == 0 {
        return;
    }
    let times = |x: u32| x.saturating_mul(n);
    match alternatives {
        [] => {}
        // One ability: its fixed mana, and one mana of a chosen type, are independent of anything else; its triggered
        // mana of a type it produced is too where that type is fixed, and one choice with it where the type is chosen.
        [one]
            if one.cost.is_zero()
                && one.k <= 1
                && !(one.k == 1 && one.restriction.is_some() && one.types != ANY_COLOR)
                && !(one.k == 1 && one.produced > 0 && (one.restriction.is_some() || one.produces(None) != 0)) =>
        {
            match one.restriction {
                Some(kind) => add_available(restricted, kind, one.fixed.map(times), times(one.k)),
                None => {
                    for m in 0..6 {
                        fixed[m] = fixed[m].saturating_add(times(one.fixed[m]));
                    }
                    if one.k == 1 && one.produced == 0 {
                        flex[one.types as usize] = flex[one.types as usize].saturating_add(n);
                    }
                }
            }
            for m in 0..6 {
                fixed[m] = fixed[m].saturating_add(times(one.extra[m]));
            }
            if one.produced > 0 {
                if one.k == 1 {
                    linked.extend(std::iter::repeat_n((one.types, 1 + one.produced), n as usize));
                } else {
                    let types = one.produces(None) as usize;
                    flex[types] = flex[types].saturating_add(times(one.produced));
                }
            }
        }
        // Several abilities each making one unrestricted mana for nothing: one mana of a type chosen among theirs
        // (a dual land), and triggered mana of that type one choice with it.
        several
            if several.len() > 1
                && several.iter().all(|a| {
                    a.restriction.is_none()
                        && a.cost.is_zero()
                        && a.fixed.iter().sum::<u32>() + a.k == 1
                        && a.extra == several[0].extra
                        && a.produced == several[0].produced
                }) =>
        {
            let types = several.iter().fold(0u8, |t, a| t | a.produces(None) | a.types);
            for m in 0..6 {
                fixed[m] = fixed[m].saturating_add(times(several[0].extra[m]));
            }
            if several[0].produced > 0 {
                linked.extend(std::iter::repeat_n((types, 1 + several[0].produced), n as usize));
            } else {
                flex[types as usize] = flex[types as usize].saturating_add(n);
            }
        }
        // Anything else is a choice the counts can't say: tried exactly (`Available::exact`).
        _ => match complex.iter_mut().find(|(alike, _)| alike.as_slice() == alternatives) {
            Some((_, count)) => *count = count.saturating_add(n),
            None => complex.push((alternatives.to_vec(), n)),
        },
    }
}

/// The permanents that could have a triggered mana ability (CR 605.1b) now, however it was given (`settle::could_have`): the
/// only ones looked at when a permanent is tapped for mana. Unallocated while there are none.
fn mana_trigger_holders(cx: &Cx) -> Vec<ObjectId> {
    let could = crate::settle::could_have(cx, &cx.cards().mana_triggers, |t| t.mana_ability);
    let holders: Vec<ObjectId> = cx.s().battlefield().iter().copied().filter(|&id| could(id)).collect();
    // With `CANTRIP_VERIFY`: a permanent left out has no triggered mana ability now - a way of giving one that the set misses
    // is found here, not as mana that silently never comes.
    if crate::layers::verifying() {
        for &id in cx.s().battlefield().iter().filter(|&&id| !could(id)) {
            assert!(
                !cx.chars(id).abilities().any(|a| matches!(cx.cards().ability(a), AbilityDef::Triggered(t) if t.mana_ability)),
                "{id:?} has a triggered mana ability that nothing counted as able to give it (CR 605.1b)"
            );
        }
    }
    holders
}

/// The triggered mana abilities that a permanent's being tapped for mana could trigger (CR 605.1b): those of the permanents
/// that could have one (`mana_trigger_holders`), and the delayed ones (High Tide's). Empty, and unallocated, while there are none.
/// Each asked as the ability it is, where an effect gave it (CR 607.5, `Cx::link_nth`).
fn waiting_mana_triggers(cx: &Cx) -> Vec<(Asker, &'static TriggeredDef)> {
    let mut waiting = Vec::new();
    let cards = cx.cards();
    for id in mana_trigger_holders(cx) {
        let controller = cx.s().obj(id).controller;
        let chars = cx.chars(id);
        for (nth, ability) in chars.abilities().enumerate() {
            if let AbilityDef::Triggered(t) = cards.ability(ability)
                && t.mana_ability
            {
                waiting.push((Asker { source: id, controller, by: cx.link_nth(id, &chars, nth), spell: false }, t));
            }
        }
    }
    for d in cx.s().delayed.iter().filter(|d| d.when.is_event()) {
        if let AbilityDef::Triggered(t) = cards.ability(d.ability)
            && t.mana_ability
        {
            let by = layers::given_link(cards, cx.s(), d.source, d.ability, d.acquired);
            waiting.push((Asker { source: d.source, controller: d.controller, by, spell: false }, t));
        }
    }
    waiting
}

/// What `id`, tapped for mana by `player` with a mana ability that adds `fixed` and `k` mana of one type chosen from `types`
/// (bits as `Mana` orders them), would produce once the replacement effects on its mana have applied (CR 106.12b): every
/// outcome, for each type its chosen mana may be, in each order its player could apply them (`replace::mana_outcomes`). None
/// where no effect would change what it produces, whatever is chosen. A choice that adds nothing produces no mana, and is no
/// event to replace (CR 106.12b).
fn replaced_yields(cx: &Cx, id: ObjectId, player: PlayerId, fixed: [u32; 6], k: u32, types: u8, restriction: Option<ManaRestriction>) -> Option<Vec<ManaProduction>> {
    let mut out: Vec<ManaProduction> = Vec::new();
    let mut changed = false;
    let mut each = |made: [u32; 6]| {
        let production = ManaProduction { permanent: id, player, to: player, mana: ManaPool(made), restriction, lasting: false };
        if production.mana.is_empty() {
            out.push(production);
            return;
        }
        for o in crate::replace::mana_outcomes(cx, production) {
            changed |= o != production;
            if !out.contains(&o) {
                out.push(o);
            }
        }
    };
    if k > 0 && types != 0 {
        for m in (0..6).filter(|&m| types & (1 << m) != 0) {
            let mut made = fixed;
            made[m] = made[m].saturating_add(k);
            each(made);
        }
    } else {
        each(fixed);
    }
    changed.then_some(out)
}

/// What the triggered mana abilities `waiting` would add to `player` as `id` is tapped for mana of one of `types` (CR 605.1b,
/// 605.4a: they resolve at once, as part of the mana ability): advice, added to what the permanent itself adds, so a cast
/// High Tide's extra {U} pays for is offered before the Islands are tapped. Each one that would trigger on the tap counts
/// once, by what it says it adds (`TriggeredMana`): fixed mana, and how many add "one mana of any type that land produced",
/// which is a type the activation produced (CR 106.12a) - the type it chose, where it chose one.
fn tap_triggers(cx: &Cx, waiting: &[(Asker, &'static TriggeredDef)], id: ObjectId, player: PlayerId, types: u8) -> ([u32; 6], u32) {
    let (mut extra, mut produced_count) = ([0u32; 6], 0u32);
    let first = (0..6).find(|&m| types & (1 << m) != 0).unwrap_or(5);
    let mut produced = ManaTally([0; 6]);
    produced.0[first] = 1;
    let event = Event::TappedForMana { permanent: id, player, produced };
    for (asker, t) in waiting {
        if t.fires(cx, asker, &event).is_none() || t.condition.is_some_and(|condition| !condition(cx, asker)) {
            continue;
        }
        match t.mana_yield {
            TriggeredMana::One(mana) => extra[mana as usize] = extra[mana as usize].saturating_add(1),
            TriggeredMana::OneOfProduced => produced_count = produced_count.saturating_add(1),
            TriggeredMana::ChosenColor => {
                if let Some(color) = cx.s().chosen_color(cx.linked_to(asker)) {
                    let mana = color.mana() as usize;
                    extra[mana] = extra[mana].saturating_add(1);
                }
            }
            TriggeredMana::Unstated => {}
        }
    }
    (extra, produced_count)
}

/// The spell a card or face is when cast, as far as the mana limited to some spells is concerned (CR 106.6): advice,
/// which the payment judges again on the stack. Its subtypes are the face's, a changeling face's every creature type
/// (CR 702.73a), and for the whole card those effects give it where it is. They are only worked out when some mana at hand
/// is restricted by subtype.
///
/// Its mana value is the face's with X at 0 (CR 202.3e), or for the back face of a nonmodal double-faced card its front
/// face's (CR 712.8c); `x`, whether its mana cost has {X} in it, says it could be more (`Available::covers_for`).
fn spell_kind(cx: &Cx, a: &Available, card: ObjectId, def: &CardDef) -> ManaPayment {
    let whole = cx.card(cx.s().obj(card).card);
    let back = !def.has_cost && whole.back.is_some_and(|b| std::ptr::eq(b, def));
    // At the least X may be announced as (CR 202.3e), as `least_cost` judges it.
    let mana_value = if back { whole.cost.mana_value() } else { def.cost.mana_value() + u32::from(def.cost.x) * u32::from(least_x(def.x_cant_be_zero)) };
    let (types, legendary, x) = (def.types, def.supertypes.legendary, def.cost.x > 0);
    // Cast from where the card is (CR 601.2a).
    let from = cx.s().obj(card).zone.kind();
    if !a.names_subtypes() {
        return ManaPayment::Spell { types, subtypes: SubtypeSet::NONE, legendary, mana_value, x, from };
    }
    let mut subtypes = def.subtypes;
    if std::ptr::eq(def, cx.card(cx.s().obj(card).card)) {
        subtypes = subtypes.union(cx.chars(card).subtypes);
    }
    if def.keywords.has(Keyword::Changeling) && (def.types.has(CardType::Creature) || def.types.has(CardType::Kindred)) {
        subtypes = Subtype::CREATURE_TYPES.into_iter().fold(subtypes, SubtypeSet::with);
    }
    ManaPayment::Spell { types, subtypes, legendary, mana_value, x, from }
}

/// Whether a static ability lets `player` spend mana as though it were mana of any color to cast `card` as `def` (CR
/// 609.4b: Oath of Nissa's "to cast planeswalker spells"), asked before it is cast: the rule reads the card as the spell it
/// would be, the face it would be cast as (CR 712.11c, 709.3a, 715.4, `AsFace`). Mana spent so for every payment
/// (`Available::any_color`) is not asked again.
fn casts_with_any_color(cx: &Cx, a: &Available, player: PlayerId, card: ObjectId, def: &CardDef) -> bool {
    if a.spenders.is_empty() || a.any_color {
        return false;
    }
    let spending = Spending { player, source: Some(card), purpose: SpendingFor::Casting };
    match face_of(cx, card, def) {
        Some(face) => static_rules::lets_spend_any_color(&AsFace { cx, spell: card, face }, &a.spenders, &spending),
        None => static_rules::lets_spend_any_color(cx, &a.spenders, &spending),
    }
}

/// Whether `cost` could be paid to cast `card` as `def` (advice, CR 601.2g-h): by the mana at hand that may pay for that
/// spell (`spell_kind`), spent as though it were mana of any color where a static ability lets `player` do so for it
/// (`casts_with_any_color`, CR 609.4b).
fn spell_covers(cx: &Cx, a: &Available, player: PlayerId, card: ObjectId, def: &CardDef, cost: &ManaCost) -> bool {
    let payment = spell_kind(cx, a, card, def);
    if casts_with_any_color(cx, a, player, card, def) { a.covers_for(&cost.paid_with_any_color(), &payment) } else { a.covers_for(cost, &payment) }
}

impl Available {
    /// Whether `cost` could be paid for `payment` (casting a spell of a kind, activating an ability of a source): the
    /// restricted mana that may be spent on it counts (CR 106.6, 601.2g, 602.2b).
    /// Whether some of its restricted mana is limited by the spell's subtypes.
    fn names_subtypes(&self) -> bool {
        self.restricted.iter().any(|(r, ..)| r.names_subtypes())
            || self.complex.iter().flat_map(|(alternatives, _)| alternatives).any(|a| a.restriction.is_some_and(|r| r.names_subtypes()))
    }

    pub fn covers_for(&self, cost: &ManaCost, payment: &ManaPayment) -> bool {
        // A monocolored hybrid symbol ({2/W}) is paid as {W} or as {2}, as announced (CR 601.2b): covered if some
        // announcement is.
        if cost.twobrid != [0; 5] {
            let n = cost.twobrids().min(8) as usize;
            return (0u32..1 << n).any(|mask| {
                let generic: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
                self.covers_for(&cost.with_twobrids_paid(&generic), payment)
            });
        }
        // CR 609.4b: a player who may spend mana as though it were mana of any color pays a colored symbol with any mana.
        let as_paid;
        let cost = if self.any_color {
            as_paid = cost.paid_with_any_color();
            &as_paid
        } else {
            cost
        };
        if self.covers_as(cost, payment) {
            return true;
        }
        // A spell whose X is still to be announced (CR 601.2b) has the larger mana value the larger X is (CR 202.3e):
        // mana for spells of a mana value or greater may pay for it at an X that brings it there (Ashling, Rimebound's
        // mana and a {X}{R} spell), each X costing its {X} symbols more.
        let ManaPayment::Spell { types, subtypes, legendary, mana_value, x, from } = *payment else { return false };
        let complex = self.complex.iter().flat_map(|(alternatives, _)| alternatives).filter_map(|a| a.restriction);
        let Some(most) = self.restricted.iter().map(|&(r, ..)| r).chain(complex).filter_map(|r| r.least_mana_value()).max() else { return false };
        if cost.x == 0 || mana_value >= most as u32 {
            return false;
        }
        let per = cost.x as u32;
        (1..=(most as u32 - mana_value).div_ceil(per)).any(|n| {
            let mut more = *cost;
            more.generic = (more.generic as u32 + n * per).min(255) as u8;
            self.covers_as(&more, &ManaPayment::Spell { types, subtypes, legendary, mana_value: mana_value + n * per, x, from })
        })
    }

    /// `covers_for` at the cost as it is.
    fn covers_as(&self, cost: &ManaCost, payment: &ManaPayment) -> bool {
        if !self.complex.is_empty() {
            return self.exact(cost, Some(payment));
        }
        if !self.restricted.iter().any(|(r, ..)| r.may_pay(payment)) {
            return self.covers(cost);
        }
        let mut with = Available {
            fixed: self.fixed,
            flex: self.flex,
            unknown: self.unknown,
            changers: Vec::new(),
            grantors: Vec::new(),
            restricted: Vec::new(),
            any_color: false,
            spenders: Vec::new(),
            linked: self.linked.clone(),
            complex: Vec::new(),
            spent: Vec::new(),
        };
        for (r, fixed, flex) in &self.restricted {
            if r.may_pay(payment) {
                for m in 0..6 {
                    with.fixed[m] = with.fixed[m].saturating_add(fixed[m]);
                }
                with.flex[ANY_COLOR as usize] += flex;
            }
        }
        with.covers(cost)
    }

    /// How much mana of no restriction it could pay at most (advice): the pool's and what the mana abilities add, less
    /// what those abilities cost themselves; `u16::MAX` when some ability's yield is not stated.
    pub fn most_unrestricted(&self) -> u16 {
        if self.unknown {
            return u16::MAX;
        }
        let complex: u32 = self
            .complex
            .iter()
            .map(|(alternatives, n)| {
                let most = alternatives
                    .iter()
                    .filter(|a| a.restriction.is_none())
                    .map(|a| (a.fixed.iter().sum::<u32>() + a.k + a.extra.iter().sum::<u32>() + a.produced).saturating_sub(a.cost.mana_value()))
                    .max()
                    .unwrap_or(0);
                most.saturating_mul(*n)
            })
            .fold(0u32, u32::saturating_add);
        let total = self.fixed.iter().map(|&n| n as u32).sum::<u32>() + self.flex.iter().sum::<u32>() + self.linked.iter().map(|&(_, n)| n).sum::<u32>() + complex;
        total.min(u16::MAX as u32) as u16
    }

    /// Whether `cost` could be paid (for `payment`, or with unrestricted mana alone) by some choice of the `complex` sources:
    /// each one an ability of its own, or none where an ability costs mana, and the type of each "one type chosen" it makes;
    /// sources alike are tried by how many make each choice. The rest is judged as it is.
    fn exact(&self, cost: &ManaCost, payment: Option<&ManaPayment>) -> bool {
        // The types some symbol asks for by name: mana of any other type pays only generic mana, so of the types a source
        // may choose that none asks for, one stands for them all.
        let mut named = cost.hybrid.iter().flatten().fold(0u8, |t, &(a, b)| t | 1 << a as usize | 1 << b as usize);
        for (alternatives, _) in &self.complex {
            for a in alternatives {
                named |= a.cost.hybrid.iter().flatten().fold(0u8, |t, &(a, b)| t | 1 << a as usize | 1 << b as usize);
                named |= (0..6).filter(|&m| a.cost.symbols[m] > 0).fold(0u8, |t, m| t | 1 << m);
            }
        }
        named |= (0..6).filter(|&m| cost.symbols[m] > 0).fold(0u8, |t, m| t | 1 << m);
        // Each source's choices: the ability, and the type of its chosen mana; `None` for leaving it unactivated.
        let choices: Vec<Vec<Option<(Alternative, Option<usize>)>>> = self
            .complex
            .iter()
            .map(|(alternatives, _)| {
                let mut out = Vec::new();
                if alternatives.iter().any(|a| !a.cost.is_zero()) {
                    out.push(None);
                }
                for a in alternatives {
                    if a.k > 0 {
                        let other = (0..6).find(|&m| a.types & !named & (1 << m) != 0);
                        out.extend((0..6).filter(|&m| a.types & named & (1 << m) != 0 || Some(m) == other).map(|m| Some((*a, Some(m)))));
                    } else {
                        out.push(Some((*a, None)));
                    }
                }
                if out.is_empty() {
                    out.push(None);
                }
                out
            })
            .collect();
        // The activations some choice pays for with mana: restricted mana that may not pay the cost might pay for them.
        let mut costed: Vec<ManaPayment> = Vec::new();
        for a in self.complex.iter().flat_map(|(alternatives, _)| alternatives).filter(|a| !a.cost.is_zero()) {
            if !costed.contains(&a.source) {
                costed.push(a.source);
            }
        }
        let mut search = Search {
            cost,
            payment,
            choices: &choices,
            counts: &[],
            demand: demand_by_types(cost),
            costed,
            later: Vec::new(),
            each: Vec::new(),
            budget: std::cell::Cell::new(SEARCH_BUDGET),
        };
        // At most what a source of each group could make of each set of types, its best choice for that set; and the sum
        // over the groups after each.
        // The first 64: of each set of types; the next 64: of each set and of all types, added (one choice makes both, so a
        // choice that makes the most of one may not of the other - Cavern of Souls under Mana Flare: {C}{C}, or one colored).
        search.each = choices
            .iter()
            .map(|options| {
                let mut most = [0u32; 128];
                for option in options {
                    let all = search.within(*option, 63);
                    for types in 0..64 {
                        let n = search.within(*option, types as u8);
                        most[types] = most[types].max(n);
                        most[64 + types] = most[64 + types].max(n.saturating_add(all));
                    }
                }
                most
            })
            .collect();
        let counts: Vec<u32> = self.complex.iter().map(|&(_, n)| n).collect();
        search.later = vec![[0u32; 128]; choices.len() + 1];
        for g in (0..choices.len()).rev() {
            for i in 0..128 {
                search.later[g][i] = search.later[g + 1][i].saturating_add(search.each[g][i].saturating_mul(counts[g]));
            }
        }
        search.counts = &counts;
        let with = self.without_complex();
        let held = search.held(&with);
        search.group(0, &Leaf { with, costed: Vec::new(), held })
    }

    /// A copy with no `complex` sources (nor the permanents that change costs, which the copy doesn't judge).
    fn without_complex(&self) -> Available {
        Available {
            fixed: self.fixed,
            flex: self.flex,
            unknown: self.unknown,
            changers: Vec::new(),
            grantors: Vec::new(),
            restricted: self.restricted.clone(),
            any_color: self.any_color,
            spenders: Vec::new(),
            linked: self.linked.clone(),
            complex: Vec::new(),
            spent: Vec::new(),
        }
    }

    /// Whether `cost` could be paid with it (advice, CR 601.2g).
    pub fn covers(&self, cost: &ManaCost) -> bool {
        if self.unknown || cost.mana_value() == 0 {
            return true;
        }
        if !self.complex.is_empty() {
            return self.exact(cost, None);
        }
        if self.linked.is_empty() {
            return self.covers_with(cost, self.fixed);
        }
        // Each group of one chosen type is that many of one of its types: covered if some choice for every group is. Groups
        // alike are interchangeable, so only how many of them choose each type is tried.
        let mut kinds: Vec<((u8, u32), u32)> = Vec::new();
        for &group in &self.linked {
            match kinds.iter_mut().find(|(g, _)| *g == group) {
                Some((_, n)) => *n += 1,
                None => kinds.push((group, 1)),
            }
        }
        fn choose(this: &Available, cost: &ManaCost, kinds: &[((u8, u32), u32)], fixed: [u32; 6]) -> bool {
            let Some((&((types, each), count), rest)) = kinds.split_first() else { return this.covers_with(cost, fixed) };
            let options: Vec<usize> = (0..6).filter(|&m| types & (1 << m) != 0).collect();
            // `count` groups among `options`' types, as many to each as `share` says.
            fn spread(this: &Available, cost: &ManaCost, rest: &[((u8, u32), u32)], fixed: [u32; 6], options: &[usize], left: u32, each: u32) -> bool {
                let Some((&m, others)) = options.split_first() else { return left == 0 && choose(this, cost, rest, fixed) };
                if others.is_empty() {
                    let mut with = fixed;
                    with[m] = with[m].saturating_add(left.saturating_mul(each));
                    return choose(this, cost, rest, with);
                }
                (0..=left).any(|here| {
                    let mut with = fixed;
                    with[m] = with[m].saturating_add(here.saturating_mul(each));
                    spread(this, cost, rest, with, others, left - here, each)
                })
            }
            spread(this, cost, rest, fixed, &options, count, each)
        }
        choose(self, cost, &kinds, self.fixed)
    }

    /// `covers` with this mana of fixed types.
    fn covers_with(&self, cost: &ManaCost, fixed: [u32; 6]) -> bool {
        // A hybrid symbol may be paid by either of two types: which mana pays which symbol is found as a flow (a symbol paid
        // greedily may take the mana another needed - Leyline of the Guildpact off a Forest, Plains, Island and Mountain).
        if cost.hybrid.iter().any(Option::is_some) {
            let mut supply: Vec<Supply> = (0..6).map(|m| Supply { stage: 0, types: 1 << m, restriction: None, n: fixed[m] }).collect();
            supply.extend(self.flex.iter().enumerate().filter(|&(_, &n)| n > 0).map(|(types, &n)| Supply { stage: 0, types: types as u8, restriction: None, n }));
            let mut demand = Vec::new();
            Demand::of(cost, 1, None, &mut demand);
            supply.retain(|s| s.n > 0 && s.types != 0);
            demand.retain(|d| d.n > 0);
            return transports(&supply, &demand);
        }
        let flex: u32 = self.flex.iter().sum();
        let mut left = fixed;
        // The symbols the fixed mana leaves unpaid, by the mana types that could pay each.
        let mut unpaid = [0u32; 64];
        let mut flex_needed: u32 = 0;
        for m in 0..6 {
            let need = u32::from(cost.symbols[m]);
            let have = left[m];
            let used = need.min(have);
            left[m] -= used;
            if need > used {
                unpaid[1 << m] += need - used;
                flex_needed += need - used;
            }
        }
        for (a, b) in cost.hybrid.iter().flatten() {
            if left[*a as usize] > 0 {
                left[*a as usize] -= 1;
            } else if left[*b as usize] > 0 {
                left[*b as usize] -= 1;
            } else {
                unpaid[(1 << *a as usize) | (1 << *b as usize)] += 1;
                flex_needed += 1;
            }
        }
        if flex_needed > 0 && !flex_pays(&unpaid, &self.flex) {
            return false;
        }
        let spare: u32 = left.iter().map(|&n| n as u32).sum::<u32>() + (flex - flex_needed);
        spare >= cost.generic as u32
    }
}

/// One choice of the `complex` sources, made so far: the mana of those that cost nothing to activate, added to `with`, and
/// the activations that cost mana, each how many times.
struct Leaf {
    with: Available,
    costed: Vec<(Alternative, Option<usize>, u32)>,
    /// What it could make at most of each set of types (`Search::held`, `Search::within`).
    held: [u32; 64],
}

/// How many steps `Available::exact` takes at most - choices weighed and payments tried - before it stops and answers that
/// the cost might be covered: advice that offers a cast whose payment then fails, and is taken back (CR 733.1), never one
/// that withholds a cast that could be made. Rare heavy boards reach it (about 1 ms of search: several Caverns of Souls and
/// Chromatic Stars under two Mana Flares with a Black Lotus), and get that over-offer.
const SEARCH_BUDGET: u32 = 768;

/// How many symbols of `cost` only mana of the types in each set (a bit for each `Mana`) may pay: a colored symbol its
/// type, a hybrid one either, the generic mana every type.
fn demand_by_types(cost: &ManaCost) -> [u32; 64] {
    let mut demand = [0u32; 64];
    for (types, demand) in demand.iter_mut().enumerate() {
        let mut n: u32 = (0..6).filter(|&m| types & (1 << m) != 0).map(|m| u32::from(cost.symbols[m])).sum();
        n += cost.hybrid.iter().flatten().filter(|&&(a, b)| types & (1 << a as usize) != 0 && types & (1 << b as usize) != 0).count() as u32;
        if types == 63 {
            n += u32::from(cost.generic);
        }
        *demand = n;
    }
    demand
}

/// `Available::exact`'s search over the choices of the complex sources, group by group.
struct Search<'a> {
    cost: &'a ManaCost,
    payment: Option<&'a ManaPayment>,
    choices: &'a [Vec<Option<(Alternative, Option<usize>)>>],
    counts: &'a [u32],
    /// `demand_by_types` of the cost.
    demand: [u32; 64],
    /// What the activations some choice pays mana for are paid as (`Alternative::source`).
    costed: Vec<ManaPayment>,
    /// For each group, what one of its sources could make at most of each set of types (`Search::within`).
    each: Vec<[u32; 128]>,
    /// For each group, what all the groups from it on could make at most of each set of types.
    later: Vec<[u32; 128]>,
    /// The steps left (`SEARCH_BUDGET`).
    budget: std::cell::Cell<u32>,
}

impl Search<'_> {
    /// Takes `n` steps of the budget: false once it is spent.
    fn spend(&self, n: u32) -> bool {
        let left = self.budget.get();
        self.budget.set(left.saturating_sub(n));
        left > 0
    }

    /// Whether mana restricted to `kind` may pay for some activation a choice costs mana for.
    fn pays_activation(&self, kind: ManaRestriction) -> bool {
        self.costed.iter().any(|p| kind.may_pay(p))
    }

    /// Whether mana restricted to `kind` may pay the cost.
    fn may_pay(&self, kind: ManaRestriction) -> bool {
        self.payment.is_some_and(|p| kind.may_pay(p))
    }

    /// How much mana `choice` could make that is one of `types` and could pay the cost: for every type, what it makes less
    /// what its activation costs; restricted mana that may not pay the cost only where some activation costs mana it might
    /// pay instead.
    fn within(&self, choice: Option<(Alternative, Option<usize>)>, types: u8) -> u32 {
        let Some((a, chosen)) = choice else { return 0 };
        let made = a.made(chosen);
        let own = if a.restriction.is_none_or(|r| self.may_pay(r) || (types == 63 && self.pays_activation(r))) {
            (0..6).filter(|&m| types & (1 << m) != 0).map(|m| made[m]).fold(0u32, u32::saturating_add)
        } else {
            0
        };
        let extra = (0..6).filter(|&m| types & (1 << m) != 0).map(|m| a.extra[m]).fold(0u32, u32::saturating_add);
        let produced = if a.produces(chosen) & types != 0 { a.produced } else { 0 };
        let n = own.saturating_add(extra).saturating_add(produced);
        if types == 63 { n.saturating_sub(a.cost.mana_value()) } else { n }
    }

    /// What `with` holds of each set of types that could pay the cost (Hall's condition, as `flex_pays`).
    fn held(&self, with: &Available) -> [u32; 64] {
        let mut held = [0u32; 64];
        let flex: Vec<(usize, u32)> = with.flex.iter().enumerate().filter(|&(_, &k)| k > 0).map(|(mask, &k)| (mask, k)).collect();
        for (types, held) in held.iter_mut().enumerate() {
            let mut n: u32 = (0..6).filter(|&m| types & (1 << m) != 0).map(|m| with.fixed[m]).fold(0, u32::saturating_add);
            n = flex.iter().filter(|&&(mask, _)| mask & types != 0).fold(n, |n, &(_, k)| n.saturating_add(k));
            for &(kind, fixed, k) in &with.restricted {
                if self.may_pay(kind) || (types == 63 && self.pays_activation(kind)) {
                    n = (0..6).filter(|&m| types & (1 << m) != 0).fold(n, |n, m| n.saturating_add(fixed[m]));
                    if types & ANY_COLOR as usize != 0 {
                        n = n.saturating_add(k);
                    }
                }
            }
            n = with.linked.iter().filter(|&&(mask, _)| mask as usize & types != 0).fold(n, |n, &(_, k)| n.saturating_add(k));
            *held = n;
        }
        held
    }

    /// Whether some choice for the groups from `at` on covers the cost.
    fn group(&self, at: usize, leaf: &Leaf) -> bool {
        if at == self.choices.len() {
            return self.judge(leaf);
        }
        self.spread(at, 0, self.counts[at], leaf)
    }

    /// Whether some spread of `left` sources of group `at` over its choices from `option` on covers the cost.
    fn spread(&self, at: usize, option: usize, left: u32, leaf: &Leaf) -> bool {
        if !self.spend(1) {
            return true;
        }
        if !self.might(at, left, leaf) {
            return false;
        }
        let options = &self.choices[at];
        if option + 1 >= options.len() {
            return match left {
                0 => self.group(at + 1, leaf),
                _ => self.group(at + 1, &self.take(leaf, options[option], left)),
            };
        }
        (0..=left).rev().any(|here| match here {
            0 => self.spread(at, option + 1, left, leaf),
            _ => self.spread(at, option + 1, left - here, &self.take(leaf, options[option], here)),
        })
    }

    /// Whether the cost might still be covered with `leaf`, `left` sources of group `at` and those of the groups after it to
    /// be chosen for: for every set of types, the symbols only those types may pay are no more than the mana of them there
    /// could be if each source made the most of them it could (Hall's condition on a bound), and those symbols and all the
    /// cost no more than the most a source could make of both together - a choice that fails it fails.
    fn might(&self, at: usize, left: u32, leaf: &Leaf) -> bool {
        let could = |i: usize, held: u32| held.saturating_add(self.each[at][i].saturating_mul(left)).saturating_add(self.later[at + 1][i]);
        (1..64).all(|types| {
            self.demand[types] <= could(types, leaf.held[types])
                && self.demand[types].saturating_add(self.demand[63]) <= could(64 + types, leaf.held[types].saturating_add(leaf.held[63]))
        })
    }

    /// `leaf` with `n` sources more making `choice`.
    fn take(&self, leaf: &Leaf, choice: Option<(Alternative, Option<usize>)>, n: u32) -> Leaf {
        let mut next = Leaf { with: leaf.with.without_complex(), costed: leaf.costed.clone(), held: leaf.held };
        let Some((a, chosen)) = choice.filter(|_| n > 0) else { return next };
        for (types, held) in next.held.iter_mut().enumerate() {
            *held = held.saturating_add(self.within(choice, types as u8).saturating_mul(n));
        }
        if !a.cost.is_zero() {
            next.costed.push((a, chosen, n));
            return next;
        }
        let with = &mut next.with;
        let made = a.made(chosen).map(|x| x.saturating_mul(n));
        match a.restriction {
            Some(kind) => add_available(&mut with.restricted, kind, made, 0),
            None => {
                for m in 0..6 {
                    with.fixed[m] = with.fixed[m].saturating_add(made[m]);
                }
            }
        }
        for m in 0..6 {
            with.fixed[m] = with.fixed[m].saturating_add(a.extra[m].saturating_mul(n));
        }
        if a.produced > 0 {
            let types = a.produces(chosen) as usize;
            with.flex[types] = with.flex[types].saturating_add(a.produced.saturating_mul(n));
        }
        next
    }

    /// Whether the mana of one full choice covers the cost: as it is where no activation costs mana, and otherwise in some
    /// order of the activations, each paid with mana there before it (CR 605.3a: a mana ability's cost is paid as it is
    /// activated, never with its own mana).
    fn judge(&self, leaf: &Leaf) -> bool {
        if leaf.costed.is_empty() || leaf.with.unknown {
            return match self.payment {
                Some(payment) => leaf.with.covers_as(self.cost, payment),
                None => leaf.with.covers(self.cost),
            };
        }
        let with = &leaf.with;
        let mut supply: Vec<Supply> = Vec::new();
        for m in 0..6 {
            supply.push(Supply { stage: 0, types: 1 << m, restriction: None, n: with.fixed[m] });
        }
        for (types, &n) in with.flex.iter().enumerate().filter(|(_, n)| **n > 0) {
            supply.push(Supply { stage: 0, types: types as u8, restriction: None, n });
        }
        for &(kind, fixed, n) in &with.restricted {
            for m in 0..6 {
                supply.push(Supply { stage: 0, types: 1 << m, restriction: Some(kind), n: fixed[m] });
            }
            supply.push(Supply { stage: 0, types: ANY_COLOR, restriction: Some(kind), n });
        }
        let mut linked = with.linked.clone();
        linked.sort_unstable();
        let mut left: Vec<u32> = leaf.costed.iter().map(|&(_, _, n)| n).collect();
        self.linked_types(leaf, &mut supply, &linked, 0, 0, &mut Vec::new(), &mut left)
    }

    /// `judge`, with a type chosen for each group of `linked` from `at` on (groups alike in a type order no lower than the
    /// one before, the same choices in another order being the same mana).
    #[allow(clippy::too_many_arguments)]
    fn linked_types(&self, leaf: &Leaf, supply: &mut Vec<Supply>, linked: &[(u8, u32)], at: usize, least: usize, order: &mut Vec<usize>, left: &mut [u32]) -> bool {
        let Some(&(types, n)) = linked.get(at) else {
            return self.orders(leaf, supply, order, left);
        };
        let floor = if at > 0 && linked[at - 1] == linked[at] { least } else { 0 };
        (floor..6).filter(|&m| types & (1 << m) != 0).any(|m| {
            supply.push(Supply { stage: 0, types: 1 << m, restriction: None, n });
            let covered = self.linked_types(leaf, supply, linked, at + 1, m, order, left);
            supply.pop();
            covered
        })
    }

    /// Whether, in some order of the activations that cost mana (those alike interchangeable), each is paid with mana there
    /// before it and the cost with what is left.
    fn orders(&self, leaf: &Leaf, supply: &[Supply], order: &mut Vec<usize>, left: &mut [u32]) -> bool {
        if order.is_empty() {
            // With every activation's mana there before any is paid for, no order can do better: tried first, and the
            // orders only where it pays.
            let all: Vec<usize> = left.iter().enumerate().flat_map(|(i, &n)| std::iter::repeat_n(i, n as usize)).collect();
            if !self.staged(leaf, supply, &all, true) {
                return false;
            }
        }
        if left.iter().all(|&n| n == 0) {
            return self.staged(leaf, supply, order, false);
        }
        for i in 0..left.len() {
            if left[i] == 0 {
                continue;
            }
            left[i] -= 1;
            order.push(i);
            let covered = self.orders(leaf, supply, order, left);
            order.pop();
            left[i] += 1;
            if covered {
                return true;
            }
        }
        false
    }

    /// Whether the activations in `order` and then the cost can all be paid, each with mana made before it: each mana sent
    /// to a symbol it may pay (CR 106.6, 601.2h), found as a maximum flow.
    /// `at_once`: every activation's mana there before any is paid for (its own too) - a bound no order passes if it fails.
    fn staged(&self, leaf: &Leaf, base: &[Supply], order: &[usize], at_once: bool) -> bool {
        if !self.spend(4) {
            return true;
        }
        let mut supply: Vec<Supply> = base.to_vec();
        let mut demand: Vec<Demand> = Vec::new();
        for (step, &i) in order.iter().enumerate() {
            let stage = if at_once { 0 } else { step as u32 + 1 };
            let (a, chosen, _) = leaf.costed[i];
            let cost = if leaf.with.any_color { a.cost.paid_with_any_color() } else { a.cost };
            Demand::of(&cost, if at_once { 1 } else { stage }, Some(a.source), &mut demand);
            let made = a.made(chosen);
            for m in 0..6 {
                supply.push(Supply { stage, types: 1 << m, restriction: a.restriction, n: made[m] });
                supply.push(Supply { stage, types: 1 << m, restriction: None, n: a.extra[m] });
            }
            supply.push(Supply { stage, types: a.produces(chosen), restriction: None, n: a.produced });
        }
        Demand::of(self.cost, u32::MAX, self.payment.copied(), &mut demand);
        supply.retain(|s| s.n > 0 && s.types != 0);
        demand.retain(|d| d.n > 0);
        transports(&supply, &demand)
    }
}

/// Mana, for `Search::staged`: made at a `stage` (0 for what is there before any activation that costs mana), of one of
/// `types`, how much.
#[derive(Clone, Copy)]
struct Supply {
    stage: u32,
    types: u8,
    restriction: Option<ManaRestriction>,
    n: u32,
}

/// Symbols to pay, for `Search::staged`: paid at a `stage` with mana made before it, by mana of one of `types`, for
/// `payment` (none: unrestricted mana only), how many.
struct Demand {
    stage: u32,
    types: u8,
    payment: Option<ManaPayment>,
    n: u32,
}

impl Demand {
    /// The symbols of `cost` (its X at 0): each colored one paid by its type, a hybrid one by either, the generic by any.
    fn of(cost: &ManaCost, stage: u32, payment: Option<ManaPayment>, out: &mut Vec<Demand>) {
        for m in 0..6 {
            out.push(Demand { stage, types: 1 << m, payment, n: u32::from(cost.symbols[m]) });
        }
        for (a, b) in cost.hybrid.iter().flatten() {
            out.push(Demand { stage, types: (1 << *a as usize) | (1 << *b as usize), payment, n: 1 });
        }
        out.push(Demand { stage, types: 0b11_1111, payment, n: u32::from(cost.generic) });
    }

    /// Whether `supply` may pay it.
    fn takes(&self, supply: &Supply) -> bool {
        supply.stage < self.stage && supply.types & self.types != 0 && supply.restriction.is_none_or(|r| self.payment.is_some_and(|p| r.may_pay(&p)))
    }
}

/// Whether all of `demand` can be paid from `supply`: a maximum flow from the mana to the symbols (Edmonds-Karp).
fn transports(supply: &[Supply], demand: &[Demand]) -> bool {
    let need: u64 = demand.iter().map(|d| u64::from(d.n)).sum();
    if need == 0 {
        return true;
    }
    if supply.iter().map(|s| u64::from(s.n)).sum::<u64>() < need {
        return false;
    }
    // Nodes: the source, each supply, each demand, the sink.
    let size = supply.len() + demand.len() + 2;
    let (source, sink, first_demand) = (0, size - 1, 1 + supply.len());
    let mut capacity = vec![vec![0u64; size]; size];
    for (i, s) in supply.iter().enumerate() {
        capacity[source][1 + i] = u64::from(s.n);
        for (j, d) in demand.iter().enumerate() {
            if d.takes(s) {
                capacity[1 + i][first_demand + j] = u64::from(s.n);
            }
        }
    }
    for (j, d) in demand.iter().enumerate() {
        capacity[first_demand + j][sink] = u64::from(d.n);
    }
    let mut flow = 0u64;
    loop {
        let mut parent = vec![usize::MAX; size];
        parent[source] = source;
        let mut queue = std::collections::VecDeque::from([source]);
        while let Some(u) = queue.pop_front() {
            for v in 0..size {
                if parent[v] == usize::MAX && capacity[u][v] > 0 {
                    parent[v] = u;
                    queue.push_back(v);
                }
            }
        }
        if parent[sink] == usize::MAX {
            return false;
        }
        let mut push = u64::MAX;
        let mut v = sink;
        while v != source {
            push = push.min(capacity[parent[v]][v]);
            v = parent[v];
        }
        let mut v = sink;
        while v != source {
            let u = parent[v];
            capacity[u][v] -= push;
            capacity[v][u] += push;
            v = u;
        }
        flow += push;
        if flow >= need {
            return true;
        }
    }
}

/// `ColorSet`'s five colors as mana types: what "mana of any color" may be.
const ANY_COLOR: u8 = 0b11111;

/// Whether the mana of a type to be chosen in `flex` could pay the symbols in `unpaid`, both counted by the mana types
/// each may be. Hall's condition (each symbol matched to a mana that may pay it): for every set of types, the symbols
/// payable only within it are no more than the mana that can be one of them.
fn flex_pays(unpaid: &[u32; 64], flex: &[u32; 64]) -> bool {
    let needs: Vec<usize> = (1..64).filter(|&m| unpaid[m] > 0).collect();
    let sources: Vec<usize> = (1..64).filter(|&m| flex[m] > 0).collect();
    for set in 1..64usize {
        let need: u32 = needs.iter().filter(|&&m| m & !set == 0).map(|&m| unpaid[m]).sum();
        if need > 0 && need > sources.iter().filter(|&&m| m & set != 0).map(|&m| flex[m]).sum::<u32>() {
            return false;
        }
    }
    true
}

/// The colors a mana ability's "mana of any color" may be: those it names, or all five.
fn any_types(cx: &Cx, id: ObjectId, player: PlayerId, def: &ActivatedDef, by: Option<Linked>) -> u8 {
    match def.colors_from {
        Some(from) => from(cx, &Asker { source: id, controller: player, by, spell: false }).0,
        None if def.colors.is_empty() => ANY_COLOR,
        None => def.colors.0,
    }
}

/// The activated abilities `player` may activate now, judged with the mana `available` could pay.
fn activated_options(cx: &Cx, player: PlayerId, available: &std::cell::OnceCell<Available>, options: &mut Vec<ActionOption>) {
    // Abilities of permanents, then abilities that function from the hand (cycling) and the graveyard (unearth).
    let sources: Vec<ObjectId> = cx
        .s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| cx.s().obj(id).controller == player)
        .chain(cx.s().zone(Zone::hand(player)).iter().copied())
        .chain(cx.s().zone(Zone::graveyard(player)).iter().copied())
        .collect();
    let forbidders = static_rules::activation_forbidders(cx);
    // The permanents that let a player pay another cost for an ability (CR 118.9, 602.2b): none, nearly always.
    let alternative_sources = static_rules::activation_alternative_sources(cx);
    // Split second (CR 702.61a-b): no activated ability but a special action (turning a face-down creature face up, CR 116.2b).
    let split = split_second(cx);
    for id in sources {
        let zone = cx.s().obj(id).zone.kind();
        let chars = cx.chars(id);
        let givers = std::cell::OnceCell::new();
        let given = layers::acquisition_possible(cx.cards(), cx.s());
        for (index, ability) in chars.abilities().enumerate() {
            // Asked as the ability it is (CR 607.5): one an effect gave the source reads only its own links.
            let by = if given { cx.link_nth(id, &chars, index) } else { None };
            let Some(def) = activated(cx, ability) else { continue };
            if def.is_mana_ability()
                || (split && !def.special_action)
                || def.from != zone
                || limit_reached(cx, id, &chars, index, def)
                || static_rules::activation_barred(cx, &forbidders, id, def)
            {
                continue;
            }
            match def.timing {
                // Foretell (CR 702.143a): during your own turn.
                Timing::OwnTurn if cx.s().turn.active != player => continue,
                // Suspend (CR 702.62a): whenever the card could be cast.
                // "Could begin to cast this card by putting it onto the stack from your hand" (CR 702.62a): from the zone
                // the ability works from, which is the zone the card would be cast from, and only where nothing would stop
                // that cast - a spell limit already reached (Archon of Emeria), the card's own "cast this spell only", a
                // ban on spells like it (the suspend rulings: "any other effects that could stop you from casting it"), a
                // spell with split second on the stack (CR 702.61a: suspend is a special action, which split second lets be
                // taken, but the card could not be cast). It could be cast as an instant could where a permission lets it
                // be cast as though it had flash, judged on the spell it would be (`as_though_flash`: Valley Floodcaller's
                // "noncreature spells"); a ban on spells like it is judged there too (`casts_forbidden_on_the_stack`).
                Timing::AsCast
                    if split
                        || cast_barred(cx, player).contains(zone)
                        || !cast_only_if_met(cx, cx.card(cx.s().obj(id).card), &Asker { source: id, controller: player, by, spell: true })
                        || !(chars.is(CardType::Instant)
                            || chars.has(Keyword::Flash)
                            || cx.card(cx.s().obj(id).card).cast_as_though_flash
                            || flash_if_met(cx, cx.card(cx.s().obj(id).card), id, player)
                            || sorcery_timing(cx, player)
                            || as_though_flash(cx, id, None)) =>
                {
                    continue;
                }
                _ => {}
            }
            let sorcery = def.timing == Timing::Sorcery || def.cost.loyalty.is_some();
            // A static ability, or an effect about the player, may lift the timing restriction of a loyalty ability (CR 606.3).
            if sorcery
                && !sorcery_timing(cx, player)
                && !(def.cost.loyalty.is_some() && static_rules::instant_loyalty(cx, player, id))
                && !static_rules::instant_activation(cx, player, id, index as u8)
            {
                continue;
            }
            // A loyalty ability once per turn per permanent (CR 606.3).
            if def.cost.loyalty.is_some() && !loyalty_available(cx, id) {
                continue;
            }
            let asker = Asker { source: id, controller: player, by, spell: false };
            if let Some(condition) = def.condition
                && !condition(cx, &asker)
            {
                continue;
            }
            // A special action's whole cost: turning a face-down permanent up for a morph cost that is not mana alone
            // ("Morph—Pay 5 life") is offered only where that could be paid (CR 702.37e, 118.3).
            let whole = if def.special_action { special_action_whole_cost(cx, id, ability, def) } else { def.cost };
            if !can_pay_activation(cx, id, player, &whole, activating(def), by) {
                continue;
            }
            // An X that can't be 0 counting targets there are none of ("X target ...", CR 107.3a, 601.2c): nothing may be
            // announced, and the activation would only be taken back.
            if def.x_cant_be_zero && def.cost.mana.x > 0 && x_target_cap(cx, &asker, def) < least_x(true) {
                continue;
            }
            // An activation whose mana cost the player cannot meet is not offered, as a spell they cannot afford is not
            // (CR 602.2b): a seat handed it would cancel at the payment and be handed it again, forever.
            let owed = if def.special_action { special_action_cost(cx, player, id, ability, def) } else { def.cost.mana };
            // An {X} is judged at the least it may be announced as (CR 107.3a, 601.2b): 0, or 1 where X can't be 0.
            let judged = at_least_x(owed, def.x_cant_be_zero).mana_value() > 0 && !def.tap_creatures_for_mana && !def.waterbend;
            // The source's own mana abilities cannot pay for an ability that taps it (CR 602.2b, 605.3b).
            // Nor may it be spent for mana by another's ability (sacrificed to an altar, tapped by Urza) while paying it.
            let own = (judged
                && def.cost.tap
                && (has_mana_source(cx, id, player) || available.get_or_init(|| self::available(cx, player)).spent.contains(&id)))
            .then(|| available_without(cx, player, Some(id), ManaWindow::Payment));
            let a = judged.then(|| own.as_ref().unwrap_or_else(|| available.get_or_init(|| self::available(cx, player))));
            // A special action is no ability, so mana limited to activating abilities can't pay for it (CR 116.2, 106.6).
            let payment = if def.special_action { ManaPayment::Other } else { ability_payment(cx, id, chars.types, chars.subtypes) };
            let covered = |a: &Available, owed: ManaCost, targets: &[Target]| {
                let owed = at_least_x(owed, def.x_cant_be_zero);
                // A special action is no activated ability (CR 116.2): what changes the cost of activating abilities does not
                // touch it; what changes its own cost is already in `owed` (`special_action_cost`), which `special_action`
                // pays. A cost of nothing is judged too: an increase still applies to it (CR 118.9d, 601.2f).
                // Mana of any color pays for it (CR 609.4b): every symbol is paid as generic mana.
                let any_color = static_rules::spend_any_color_for(cx, player, id, if def.special_action { SpendingFor::Other } else { SpendingFor::Activating });
                // Its Phyrexian symbols paid with life where the life total allows, beside the life the cost pays
                // (CR 107.4f, 601.2b by way of 602.2b, 116.2b), as the activation announces them (`announce_life`).
                let judge = |cost: ManaCost| {
                    let cost = phyrexian_with_life(cx, player, cost, whole.life, activating(def));
                    a.covers_for(&if any_color { cost.paid_with_any_color() } else { cost }, &payment)
                };
                if def.special_action {
                    // Its own cost changes, each way a reduction by hybrid mana could be split (CR 118.7e) and the hybrid
                    // symbols it meets announced (CR 601.2b), as `special_action` asks them.
                    let printed = at_least_x(special_action_printed_cost(cx, id, ability, def), def.x_cant_be_zero);
                    change_any(cx, &a.changers, player, Costed::SpecialAction(id, ability), printed, None, &[], None, &mut |c, least| judge(at_least(c, least)))
                } else {
                    // Each way the hybrid symbols a reduction meets could be announced (CR 601.2b): covered if any is.
                    change_any(cx, &a.changers, player, Costed::Ability(id, ability, by), owed, None, targets, None, &mut |c, least| judge(at_least(c, least)))
                }
            };
            // The total cost is determined once the targets are announced (CR 601.2c, 601.2f, 602.2b), so a change that reads
            // them ("equip abilities that target this creature cost {2} less") is judged for each candidate a target announced
            // with the action could be: only those the cost could then be paid for are offered. Only where something could
            // change the cost - a cost changer on the battlefield, or the ability's or card's own - since nearly nothing does;
            // otherwise the cost is judged once, before any target is looked for.
            let changes = |a: &Available| !a.changers.is_empty() || def.cost_change.is_some() || !own_cost_changes(cx, id).is_empty();
            let each_target = a.is_some_and(|a| targets_with_activation(def) && !def.special_action && def.modal.is_none() && changes(a));
            // Targets chosen once it has begun, after its modes and X (CR 601.2b-c by way of 602.2b): offered if some legal
            // announcement of them could be paid for (`some_activation`).
            let announced = a.is_some_and(|a| {
                !def.special_action && (def.modal.is_some() || !def.targets.is_empty()) && !(targets_with_activation(def) && def.modal.is_none()) && changes(a)
            });
            let judged = |a: &Available, cost: ManaCost| {
                if announced { some_activation(cx, a, &asker, def, ability, cost, &|c, t| covered(a, c, t)) } else { covered(a, cost, &[]) }
            };
            // The ability for its own cost, and for each cost a static ability lets the player pay rather than its mana cost
            // (CR 118.9, 602.2b: Forge Anew's, Kili's {0} equip), each an option of its own, made by that permanent
            // (`ActionOption::paid_by`).
            let alternatives: Vec<(Grant, ManaCost)> = if alternative_sources.is_empty() || def.special_action {
                Vec::new()
            } else {
                // An alternative cost with no {X} for an ability whose X can't be 0 leaves no X to announce (CR 107.3a, 118.9).
                static_rules::activation_alternatives(cx, &alternative_sources, player, id, def)
                    .into_iter()
                    .filter(|&(_, c)| c != owed && !x_only_zero(def.x_cant_be_zero, def.cost.mana.x, c.x))
                    .collect()
            };
            let ways = || {
                std::iter::once((None, owed))
                    .chain(alternatives.iter().map(|&(g, c)| (Some(g), c)))
                    .filter(|&(_, cost)| a.is_none_or(|a| each_target || judged(a, cost)))
            };
            if ways().next().is_none() {
                continue;
            }
            let mut option = ActionOption { granted_by: granted_by(cx, id, Some(&chars), index, ability, &givers), ..ActionOption::on(ActionKind::ActivateAbility, id, index as u8) };
            // "Sacrifice [the card that gives this ability]" (CR 201.5a, 701.21a): only with that permanent the player's, and
            // one a static ability lets them sacrifice for it (CR 101.2, `may_sacrifice_for`).
            if def.cost.sacrifice_granter
                && !option.granted_by.is_some_and(|g| cx.s().obj(g.object).controller == player && may_sacrifice_for(cx, player, g.object, activating(def)))
            {
                continue;
            }
            let possible = match def.modal {
                Some(modal) => modes_possible(cx, &asker, modal),
                None if def.late_targets => slots_satisfiable(cx, &asker, def.targets),
                None => with_targets(cx, &asker, def.targets, &mut option),
            };
            if !possible {
                continue;
            }
            for (paid_by, cost) in ways() {
                let mut option = ActionOption { paid_by, ..option.clone() };
                if let Some(a) = a.filter(|_| each_target) {
                    let kept: Vec<Target> = option.target_candidates.iter().copied().filter(|&t| covered(a, cost, &[t])).collect();
                    if kept.len() != option.target_candidates.len() {
                        if kept.is_empty() && !(option.targets_optional && covered(a, cost, &[])) {
                            continue;
                        }
                        option.target_candidates = kept;
                    }
                }
                options.push(option);
            }
        }
    }
}

/// What gave `id` the ability `ability` it has `index`th (as `Chars::abilities()` counts them), where another object's effect
/// did (CR 113.2c, 613.1f): Chromatic Lantern for the mana ability it gives a land, an Equipment for the ability it gives the
/// creature. Asked only of an ability that is not of the object's own card (its face, the card it copies) nor a rule's
/// (a basic land type's), so nearly never; `givers` keeps the object's givers once worked out (`layers::ability_givers`).
fn granted_by(
    cx: &Cx,
    id: ObjectId,
    chars: Option<&Chars>,
    index: usize,
    ability: AbilityRef,
    givers: &std::cell::OnceCell<[layers::Giver; cantrip_state::MAX_ABILITIES]>,
) -> Option<Grant> {
    let obj = cx.s().obj(id);
    if ability.card == cx.cards().intrinsic || ability.card == obj.card || obj.copy.is_some_and(|c| c.card == ability.card) {
        return None;
    }
    let worked_out;
    let chars = match chars {
        Some(chars) => chars,
        None => {
            worked_out = cx.chars(id);
            &worked_out
        }
    };
    if ability.card == chars.card {
        return None;
    }
    let slot = chars.abilities.iter().enumerate().filter(|(_, a)| a.is_some()).nth(index)?.0;
    let giver = givers.get_or_init(|| layers::ability_givers(cx.cards(), cx.s(), &cx.g.log, id))[slot].object;
    (giver != ObjectId::NONE && giver != id).then_some(Grant { object: giver, ability })
}

/// How `source` acquired its `index`th ability, as `Chars::abilities()` counts them (CR 607.5; `layers::acquired`).
fn acquired_nth(cx: &Cx, source: ObjectId, index: usize) -> Acquired {
    if !layers::acquisition_possible(cx.cards(), cx.s()) {
        return Acquired::OWN;
    }
    let chars = cx.chars(source);
    let slot = chars.abilities.iter().enumerate().filter(|(_, a)| a.is_some()).nth(index).map_or(0, |(slot, _)| slot);
    layers::acquired(cx.cards(), cx.s(), &cx.g.log, source, &chars, slot)
}

/// The marker in the history of an effect having allowed one more loyalty activation this turn (CR 606.3): an
/// activation of this ability index, past any a card prints.
pub const EXTRA_LOYALTY_ALLOWED: u8 = AbilityRef::EXTRA_LOYALTY_ALLOWED;

/// Whether a loyalty ability of `id` may be activated now (CR 606.3): none has been this turn, or effects have allowed
/// more than have been. Counted in the history, where the activations and the allowances are both recorded.
fn loyalty_available(cx: &Cx, id: ObjectId) -> bool {
    if cx.s().obj(id).loyalty_turn != cx.s().turn.number {
        return true;
    }
    let (mut allowed, mut used) = (1usize, 0usize);
    for r in cx.history().select(EventQuery::kind(EventMask::ABILITY_ACTIVATED).object(id).this_turn()) {
        if let Event::AbilityActivated { ability, .. } = r.event {
            if ability.index == EXTRA_LOYALTY_ALLOWED {
                allowed += 1;
            } else if matches!(cx.cards().ability(ability), AbilityDef::Activated(d) if d.cost.loyalty.is_some()) {
                used += 1;
            }
        }
    }
    used < allowed
}

/// The permissions `player` holds to play `card` from where it is (CR 601.3, 305.1), each its own: two that would both let
/// it be played are two ways of playing it, and the player chooses which is used (CR 601.2b).
pub fn permissions_for<'a>(cx: &'a Cx, card: ObjectId, player: PlayerId) -> impl Iterator<Item = &'a PlayPermission> + 'a {
    cx.s().permissions.iter().filter(move |p| p.card == card && p.player == player && permission_stands(cx, p, player))
}

/// Whether a permission lets `player` play its card now, as far as its own terms say (CR 601.3, 305.1): not on the turn it
/// was given if it is for a later turn (CR 702.185a), only while its condition holds (CR 611.2), and only while a "for as
/// long as you control" duration does (CR 611.2b; one that lapsed mid-resolution has not been dropped yet).
fn permission_stands(cx: &Cx, p: &PlayPermission, player: PlayerId) -> bool {
    !(p.grant.later_turns && p.turn == cx.s().turn.number)
        && p.grant.when.is_none_or(|when| play_condition_holds(cx, player, when))
        && cx.s().permission_lasts(p)
}

/// Whether the condition on a permission to play holds now (CR 611.2): asked of the history of this turn, which no later
/// event can change, so it is a creature as it was declared that counts, dead, changed or taken since; "you attacked" is
/// the declaring player's, the active player (CR 506.2, 508.1).
fn play_condition_holds(cx: &Cx, player: PlayerId, when: PlayCondition) -> bool {
    match when {
        PlayCondition::AttackedWithThisTurn(kind) => cx.history().attacked_with(player).any(|a| a.chars(cx).has_subtype(kind)),
        PlayCondition::ControlsSubtype(kind) => {
            cx.s().battlefield().iter().any(|&id| cx.s().obj(id).controller == player && cx.chars(id).has_subtype(kind))
        }
    }
}

/// The effects that let `player` play lands and cast spells from their graveyard now (`EffectKind::PlayFromGraveyard`:
/// Yawgmoth's Will), each as the `Grant` a play or cast by it is made by: the spell whose effect it is. Two are two
/// permissions, each its own option. Nothing is allocated while there is none.
fn graveyard_play(cx: &Cx, player: PlayerId) -> Vec<Grant> {
    cx.s()
        .effects
        .iter()
        .filter(|e| e.kind == EffectKind::PlayFromGraveyard && e.controller == player)
        .map(|e| Grant { object: e.source, ability: AbilityRef::spell(cx.s().obj(e.source).card) })
        .collect()
}

/// The play rules that apply to `player` now (`CardDef::play_rules`), each with the permanent whose rule it is: those of the
/// permanents they control, and with `command` of the emblems they have (CR 114.4), that have not lost their abilities (CR
/// 613.1f). A rule under "as long as" (`PlayRule::While`) applies only while its condition holds for its permanent: a
/// Case's "Solved - [static ability]" (CR 702.169b). A Room's are those of its unlocked doors: a locked door has no rules
/// text (CR 709.5).
pub fn each_play_rule(q: &dyn Query, cards: &CardTable, player: PlayerId, command: bool, mut f: impl FnMut(ObjectId, &'static PlayRule)) {
    each_play_rule_named(q, cards, player, command, |source, _, rule| f(source, rule));
}

/// `each_play_rule`, each rule with the `AbilityRef` that names it (`AbilityRef::play_rule`: the card that has it, a Room's
/// door for a door's, and its place among that card's rules, those under a `While` counted whether or not it holds), which a
/// `Grant` names it by.
pub fn each_play_rule_named(q: &dyn Query, cards: &CardTable, player: PlayerId, command: bool, mut f: impl FnMut(ObjectId, AbilityRef, &'static PlayRule)) {
    fn walk(q: &dyn Query, asker: &Asker, card: CardId, rules: &'static [PlayRule], k: &mut usize, holds: bool, f: &mut impl FnMut(ObjectId, AbilityRef, &'static PlayRule)) {
        for rule in rules {
            match rule {
                PlayRule::While(condition, inner) => {
                    let holds = holds && condition(q, asker);
                    walk(q, asker, card, inner, k, holds, f);
                }
                other => {
                    if holds {
                        f(asker.source, AbilityRef::play_rule(card, *k), other);
                    }
                    *k += 1;
                }
            }
        }
    }
    if !cards.any_play_rules {
        return;
    }
    let s = q.state();
    let emblems: &[ObjectId] = if command { s.zone(Zone::COMMAND) } else { &[] };
    for &id in s.battlefield().iter().chain(emblems) {
        let obj = s.obj(id);
        let card = cards.current(obj);
        if obj.controller != player || !cards.play_rules[card.0 as usize] || q.lost_all_abilities(id) {
            continue;
        }
        let def = cards.get(card);
        let asker = Asker { source: id, controller: player, by: None, spell: false };
        if def.is_room() {
            for door in def.unlocked_doors(obj.marks.doors) {
                walk(q, &asker, cards.id_of(door), door.play_rules, &mut 0, true, &mut f);
            }
        } else {
            walk(q, &asker, card, def.play_rules, &mut 0, true, &mut f);
        }
    }
}

/// The play rule an `AbilityRef::play_rule` names, read from its card whatever holds now.
fn play_rule_at(cards: &CardTable, ability: AbilityRef) -> Option<&'static PlayRule> {
    fn find(rules: &'static [PlayRule], k: &mut usize, want: usize) -> Option<&'static PlayRule> {
        for rule in rules {
            match rule {
                PlayRule::While(_, inner) => {
                    if let Some(found) = find(inner, k, want) {
                        return Some(found);
                    }
                }
                other => {
                    if *k == want {
                        return Some(other);
                    }
                    *k += 1;
                }
            }
        }
        None
    }
    find(cards.get(ability.card).play_rules, &mut 0, ability.as_play_rule()?)
}

/// What `player`'s permanents and the effects in force let them do about land plays (CR 305.2) and the top of their
/// library (CR 305.1, 601.3).
#[derive(Clone, Default)]
struct LandRules {
    /// The land plays beyond the one: the static abilities' and the effects' together (CR 305.2).
    extra: u32,
    /// No limit to the land plays.
    unlimited: bool,
    /// What lets lands be played from the graveyard, each its own permission (Icetill Explorer's rule, Yawgmoth's Will).
    graveyard: Vec<Grant>,
    /// What lets lands be played from the top of the library, each its own (Bolas's Citadel; Case of the Locked Hothouse,
    /// solved).
    top_lands: Vec<Grant>,
}

fn land_rules(cx: &Cx, player: PlayerId) -> LandRules {
    // "Until end of turn, you may play lands and cast spells from your graveyard" (CR 305.1, 601.3): Yawgmoth's Will.
    let mut rules = LandRules { graveyard: graveyard_play(cx, player), ..LandRules::default() };
    each_play_rule_named(cx, cx.cards(), player, false, |source, ability, rule| match rule {
        PlayRule::ExtraLandPlays(n) => rules.extra += *n as u32,
        PlayRule::AnyNumberOfLandPlays => rules.unlimited = true,
        PlayRule::LandsFromGraveyard => {
            rules.graveyard.push(Grant { object: source, ability });
        }
        PlayRule::LibraryTop | PlayRule::LibraryTopLands => {
            rules.top_lands.push(Grant { object: source, ability });
        }
        PlayRule::LibraryTopSpells(_) | PlayRule::MaximumHandSize(_) | PlayRule::LookAtLibraryTop | PlayRule::While(..) => {}
    });
    // "You may play an additional land this turn" (CR 305.2, 611.2c): an effect a resolved spell or ability made, added to
    // the rest - the number a player can play is the sum of everything that increases it.
    for e in &cx.s().effects {
        if let EffectKind::ExtraLandPlays(n) = e.kind
            && e.controller == player
        {
            rules.extra += n;
        }
    }
    rules
}

/// `player`'s maximum hand size (CR 402.2), `None` when they have no maximum: seven unless an effect says otherwise - a
/// permanent they control, an emblem they have (CR 114.4), or an effect a resolved spell made - the newest of those deciding,
/// whichever way it reads (CR 613.11, 613.7).
pub fn maximum_hand_size(cx: &Cx, player: PlayerId) -> Option<u32> {
    let mut newest: Option<(u32, Option<u32>)> = None;
    let mut says = |stamp: u32, maximum: Option<u32>| {
        if newest.is_none_or(|(at, _)| stamp >= at) {
            newest = Some((stamp, maximum));
        }
    };
    for e in &cx.s().effects {
        if let EffectKind::MaximumHandSize(maximum) = e.kind
            && e.controller == player
        {
            says(e.timestamp, maximum);
        }
    }
    each_play_rule(cx, cx.cards(), player, true, |source, rule| {
        if let PlayRule::MaximumHandSize(maximum) = rule {
            says(cx.s().obj(source).timestamp, *maximum);
        }
    });
    newest.map_or(Some(7), |(_, maximum)| maximum)
}

/// Whether `player` may look at the top card of their library any time (CR 401.2): a permanent they control says so (Bolas's
/// Citadel, Mystic Forge, Case of the Locked Hothouse once solved).
pub fn may_look_at_library_top(q: &dyn Query, cards: &CardTable, player: PlayerId) -> bool {
    let mut may = false;
    each_play_rule(q, cards, player, false, |_, rule| may |= matches!(rule, PlayRule::LookAtLibraryTop));
    may
}

/// The land plays `player` may make now (CR 305.1, 305.2), each by what lets the land be played from where it is: the rules
/// from a hand and from exile on an Adventure, a rule a permanent has from the graveyard or the top of the library, a way a
/// permanent gives, or a permission an effect gave. Each permission is an option of its own (`granted_by`), so the player
/// chooses which one a land is played by: what that one says of it applies (`PlayGrant::enters_tapped`, Serra Paragon's
/// "it gains").
fn land_plays(cx: &Cx, player: PlayerId, options: &mut Vec<ActionOption>) {
    let LandRules { extra, unlimited, graveyard, top_lands, .. } = land_rules(cx, player);
    if !unlimited && cx.s().player(player).lands_played >= 1 + extra {
        return;
    }
    // Playing a land puts it onto the battlefield (CR 305.1): one a static ability keeps from entering from where it is
    // can't be played (CR 101.2), judged as the card it is there - a modal double-faced card by its front face. And a static
    // ability may forbid the play itself, whatever allows it (CR 101.2, `LandPlayRule`). One look at the battlefield finds both.
    let bans = crate::static_rules::land_play_bans(cx);
    let forbidden = |id: ObjectId, face: CardId| !bans.is_empty() && crate::static_rules::land_play_forbidden(cx, &bans, player, id, face);
    // A land card is played as itself; a modal double-faced card whose back face is a land may be played as that
    // (CR 712.12), from wherever a land could be played.
    let offer = |id: ObjectId, by: Option<Grant>, options: &mut Vec<ActionOption>| {
        if !bans.is_empty() && crate::static_rules::enter_banned(cx, &bans, id) {
            return;
        }
        // A card exiled face down is judged as the land it would be (CR 406.3a).
        let _face_up = layers::judged_face_up(cx.s(), id);
        let card = cx.s().obj(id).card;
        if cx.chars(id).is(CardType::Land) && !forbidden(id, card) {
            options.push(ActionOption { granted_by: by, ..ActionOption::on(ActionKind::PlayLand, id, 0) });
        }
        if let Some(back) = cx.card(card).back
            && cx.card(card).back_land
            && !forbidden(id, cx.card_id(back))
        {
            options.push(ActionOption { granted_by: by, ..ActionOption::on(ActionKind::PlayLand, id, WAY_BACK_FACE) });
        }
    };
    if let Some(&top) = cx.s().zone(Zone::library(player)).first() {
        for &by in &top_lands {
            offer(top, Some(by), options);
        }
    }
    for &id in cx.s().zone(Zone::hand(player)) {
        offer(id, None, options);
    }
    for &by in &graveyard {
        for &id in cx.s().zone(Zone::graveyard(player)) {
            offer(id, Some(by), options);
        }
    }
    // A land card this player exiled on an Adventure (CR 715.3d), in whichever player's exile zone it is: "for as long as
    // that card remains exiled, that player may play it" - as a land play, at the time and within the number of land plays
    // the rules allow (CR 305.1, 305.2).
    for zone in cx.s().seats().map(Zone::exile) {
        for &id in cx.s().zone(zone) {
            if cx.s().obj(id).on_adventure == Some(player) {
                offer(id, None, options);
            }
        }
    }
    // A land card in exile a permanent lets this player play (CR 305.1, 604.1): "you may play cards exiled with this", each
    // permission its own. Looked for only when exile holds a card.
    let mut exile_casts: Option<Vec<Given<ExileCast>>> = None;
    for zone in cx.s().seats().map(Zone::exile) {
        for &id in cx.s().zone(zone) {
            if cx.s().obj(id).card_copy {
                continue;
            }
            for c in exile_casts.get_or_insert_with(|| exile_casts_of(cx.cards(), cx.s(), &cx.g.log)).iter() {
                if c.what.lands && (c.what.allows)(cx, &c.asker(), id, player) {
                    offer(id, Some(c.grant), options);
                }
            }
        }
    }
    // The land cards in the graveyard that a way a permanent gives lets the player play (CR 305.1, 604): once during each of
    // their turns, say. Looked for only when the graveyard holds a land card.
    let mut granted: Option<Vec<Given<GrantedWay>>> = None;
    for &id in cx.s().zone(Zone::graveyard(player)) {
        if !cx.chars(id).is(CardType::Land) || (!bans.is_empty() && crate::static_rules::enter_banned(cx, &bans, id)) || forbidden(id, cx.s().obj(id).card) {
            continue;
        }
        for way in granted.get_or_insert_with(|| granted_ways_of(cx.cards(), cx.s(), &cx.g.log)).iter() {
            if way.what.lands && (way.what.cost)(cx, &way.asker(), id, player).is_some() {
                options.push(ActionOption { granted_by: Some(way.grant), ..ActionOption::on(ActionKind::PlayLand, id, 0) });
            }
        }
    }
    // A land a card lets this player play from where it is (CR 305.1): "you may play that card". Each permission is its own.
    for p in &cx.s().permissions {
        if p.player == player && p.grant.lands && permission_stands(cx, p, player) && !cx.s().obj(p.card).zone.is(ZoneKind::Hand) {
            offer(p.card, Some(p.by), options);
        }
    }
}

/// The spells `player` could begin to cast: from hand by its mana cost or an alternative cost, from a graveyard by a
/// cost that casts it from there (flashback). `instants` picks the instant-speed pass or the sorcery-speed one.
/// "Each player can't cast more than N spells each turn" (CR 101.2): the smallest limit any permanent on the
/// battlefield imposes. A restriction takes precedence over the permission to cast.
fn cast_limit(cx: &Cx) -> Option<u8> {
    if !cx.cards().any_cast_limit {
        return None;
    }
    cx.s()
        .battlefield()
        .iter()
        .map(|&id| {
            let obj = cx.s().obj(id);
            // A permanent that has lost all its abilities limits nothing (CR 613.1f).
            let limit = cx.card(obj.copied_card().unwrap_or(obj.card)).cast_limit;
            if limit > 0 && cx.lost_all_abilities(id) { 0 } else { limit }
        })
        .filter(|&n| n > 0)
        .min()
}

/// The faces a card may be cast as (CR 709.3, 715.3, 720.3, 712.8a, 718.3), each with the way number a cast of it has: a split
/// card's two halves (the card itself is never cast), otherwise the card and its Adventure or Omen, its back face and its
/// prototyped face, if it has them. Prototype functions wherever the card could be cast from (the ruling), the face it
/// casts judged alone (CR 718.3a).
fn faces_to_cast(def: &'static CardDef) -> impl Iterator<Item = (u8, &'static CardDef)> {
    let (split, whole) = (def.split, def.split.is_none());
    [
        split.map(|(left, _)| (WAY_SPLIT_LEFT, left)),
        split.map(|(_, right)| (WAY_SPLIT_RIGHT, right)),
        whole.then_some((0, def)),
        def.adventure.filter(|_| whole).map(|a| (WAY_ADVENTURE, a)),
        def.omen.filter(|_| whole).map(|o| (WAY_OMEN, o)),
        def.back.filter(|b| whole && b.has_cost).map(|b| (WAY_BACK_FACE, b)),
        cantrip_api::prototyped_face(def).filter(|_| whole).map(|f| (WAY_PROTOTYPE, f)),
    ]
    .into_iter()
    .flatten()
}

/// The kinds of zone from which anything forbids `player` to cast a spell now, whatever the spell is (CR 101.2, 601.3): a
/// static ability that bars them from casting (Jennifer Walters) or from casting from some zones (Drannith Magistrate), an
/// effect that does (Avatar's Wrath), or a limit on the spells each player may cast in a turn (Archon of Emeria) already
/// reached, which bars every zone. Asked of every cast alike, whether it is begun with priority or as an effect resolves,
/// with the zone the card is in as it is begun (CR 601.2a); a copy of a card is cast from the zone it is created in (CR
/// 707.12).
fn cast_barred(cx: &Cx, player: PlayerId) -> ZoneKinds {
    if cast_limit(cx).is_some_and(|limit| cx.history().casts_by(player).count() >= limit as usize) {
        return ZoneKinds::ALL;
    }
    static_rules::cast_barred(cx, player)
}

/// Whether the card's own restriction on when it may be cast ("Cast this spell only ...", CR 601.3) allows it now.
fn cast_only_if_met(cx: &Cx, face: &'static CardDef, asker: &Asker) -> bool {
    face.cast_only_if.is_none_or(|only_if| only_if(cx, asker))
}

/// One static ability of a permanent on the battlefield, of a kind its card lists apart from its `abilities`: a way it
/// gives cards to be cast (`GrantedWay`), a permission to cast cards in exile (`ExileCast`), an alternative cost it gives
/// spells (`SpellAlternative`). With the permanent, its controller ("you") and the `Grant` an action made by it names it by.
#[derive(Clone, Copy)]
struct Given<T: 'static> {
    source: ObjectId,
    controller: PlayerId,
    grant: Grant,
    what: &'static T,
}

impl<T> Given<T> {
    fn asker(&self) -> Asker {
        Asker { source: self.source, controller: self.controller, by: None, spell: false }
    }
}

/// Every one of a kind of listed static ability the permanents on the battlefield and the emblems in the command zone have
/// now (CR 114.4: an emblem's abilities function there, Tamiyo, Field Researcher's "You may cast spells from your hand
/// without paying their mana costs"), those of a permanent that has lost its abilities left out (CR 613.1f). Nothing is
/// allocated while nothing has one.
fn given<T>(
    cards: &CardTable,
    s: &GameState,
    log: &[Record],
    list: fn(&'static CardDef) -> &'static [T],
    name: fn(CardId, usize) -> AbilityRef,
) -> Vec<Given<T>> {
    let mut all = Vec::new();
    // Of the command zone, the emblems alone: an emblem has no types (CR 114.3), and anything else there (a commander) has
    // its abilities function on the battlefield only (CR 113.6).
    let emblems = s.zone(Zone::COMMAND).iter().filter(|&&id| cards.get(cards.current(s.obj(id))).types == TypeSet::NONE);
    for &id in s.battlefield().iter().chain(emblems) {
        let obj = s.obj(id);
        let card = cards.current(obj);
        let items = list(cards.get(card));
        if items.is_empty() || crate::layers::lost_all_abilities(cards, s, log, id) {
            continue;
        }
        for (k, what) in items.iter().enumerate() {
            all.push(Given { source: id, controller: obj.controller, grant: Grant { object: id, ability: name(card, k) }, what });
        }
    }
    all
}

/// The ways the permanents on the battlefield give cards in graveyards and hands to be cast (CR 604, 118.9).
fn granted_ways_of(cards: &CardTable, s: &GameState, log: &[Record]) -> Vec<Given<GrantedWay>> {
    given(cards, s, log, |d| d.granted_ways, AbilityRef::granted_way)
}

/// The permissions the permanents on the battlefield give to cast cards in exile (CR 601.3, 604.1).
fn exile_casts_of(cards: &CardTable, s: &GameState, log: &[Record]) -> Vec<Given<ExileCast>> {
    given(cards, s, log, |d| d.exile_casts, AbilityRef::exile_cast)
}

/// The alternative costs the permanents on the battlefield give the spells players cast (CR 118.9, 601.2b).
fn spell_alternatives_of(cards: &CardTable, s: &GameState, log: &[Record]) -> Vec<Given<SpellAlternative>> {
    given(cards, s, log, |d| d.spell_alternatives, AbilityRef::spell_alternative)
}

/// The listed static ability a `Grant` names, read from its card: what an action made by it is made by, whatever else the
/// battlefield holds. The action was offered as it stands, so it is there.
fn named<T>(s: &GameState, cards: &CardTable, g: Grant, list: fn(&'static CardDef) -> &'static [T], k: Option<usize>) -> Option<Given<T>> {
    let what = list(cards.get(g.ability.card)).get(k?)?;
    Some(Given { source: g.object, controller: s.obj(g.object).controller, grant: g, what })
}

fn named_way(s: &GameState, cards: &CardTable, g: Grant) -> Option<Given<GrantedWay>> {
    named(s, cards, g, |d| d.granted_ways, g.ability.as_granted_way())
}

fn named_exile_cast(s: &GameState, cards: &CardTable, g: Grant) -> Option<Given<ExileCast>> {
    named(s, cards, g, |d| d.exile_casts, g.ability.as_exile_cast())
}

fn named_alternative(s: &GameState, cards: &CardTable, g: Grant) -> Option<Given<SpellAlternative>> {
    named(s, cards, g, |d| d.spell_alternatives, g.ability.as_spell_alternative())
}

/// The permissions permanents give now to cast `card`, a card in exile, as itself (CR 601.3): each with the player it lets
/// cast it and what it is called, for the card's face to say.
pub fn exile_cast_texts(game: &crate::Game, card: ObjectId) -> Vec<(PlayerId, &'static str)> {
    let (q, cards, log): (&dyn Query, &CardTable, &[Record]) = (game, game.cards, &game.log);
    let s = q.state();
    if !s.obj(card).zone.is(ZoneKind::Exile) {
        return Vec::new();
    }
    let _face_up = layers::judged_face_up(s, card);
    exile_casts_of(cards, s, log)
        .into_iter()
        .filter(|c| (c.what.allows)(q, &c.asker(), card, c.controller))
        .map(|c| (c.controller, c.what.text))
        .collect()
}

/// Whether the face cast is an instant or a sorcery spell: what flashback asks of "the resulting spell" (CR 702.34a).
fn instant_or_sorcery(face: &CardDef) -> bool {
    face.is(CardType::Instant) || face.is(CardType::Sorcery)
}

/// Whether the additional costs a card has (CR 118.8) could be paid, whatever way it is cast.
fn extra_payable(cx: &Cx, id: ObjectId, player: PlayerId, def: &CardDef) -> bool {
    def.additional_cost.is_none_or(|extra| can_pay_other(cx, id, player, &extra))
        && (def.additional_choice.is_empty() || def.additional_choice.iter().any(|c| can_pay_other(cx, id, player, &c.cost)))
}

/// Whether a card, or the face of it that would be cast, may be cast whenever its caster could cast an instant (CR 307.5,
/// 702.8): an instant, a spell with flash, or one a permission lets them cast as though it had flash - an effect's for some
/// card types, a static ability's (Sigarda's Aid, Valley Floodcaller, Rattlechains) - judged on the spell it would be on
/// the stack (`as_though_flash`): the face alone (CR 709.3a, 712.8a, 715.3a, 720.3a, 702.103b), so "noncreature spells" finds
/// a creature's sorcery Adventure and a bestowed Aura is an Aura spell, under its caster.
struct Speed<'a> {
    cx: &'a Cx<'a>,
    card: ObjectId,
    /// The player who would cast the card, whom a conditional permission of its own is judged for.
    player: PlayerId,
}

impl Speed<'_> {
    /// The card cast as itself, with the card's own "you may cast this spell as though it had flash".
    fn whole(&self, def: &'static CardDef) -> bool {
        def.cast_as_though_flash || own_speed(def) || flash_if_met(self.cx, def, self.card, self.player) || as_though_flash(self.cx, self.card, None)
    }

    /// The card cast as `face`: a split card's half or a Room's door, an Adventure or an Omen, a back face, a bestowed Aura.
    fn face(&self, face: &'static CardDef) -> bool {
        own_speed(face) || as_though_flash(self.cx, self.card, Some(self.cx.card_id(face)))
    }

    /// The card cast as the face a way number names.
    fn of(&self, whole: &'static CardDef, way: u8, face: &'static CardDef) -> bool {
        if way == 0 { self.whole(whole) } else { self.face(face) }
    }

    /// `of`, or `None` where a permission is not yet judged on the spell the card would be (`as_though_flash_judged`): asked
    /// for (`ask`) only where it would change what is offered.
    fn of_judged(&self, whole: &'static CardDef, way: u8, face: &'static CardDef) -> Option<bool> {
        if way == 0 && (whole.cast_as_though_flash || own_speed(whole) || flash_if_met(self.cx, whole, self.card, self.player)) || way != 0 && own_speed(face) {
            return Some(true);
        }
        as_though_flash_judged(self.cx, self.card, self.face_id(way, face))
    }

    /// Asks for `of_judged` to be judged.
    fn ask(&self, way: u8, face: &'static CardDef) {
        ask_as_though_flash(self.cx, self.card, self.face_id(way, face));
    }

    /// The face a way number names, as `as_though_flash` names it: `None` for the card itself.
    fn face_id(&self, way: u8, face: &'static CardDef) -> Option<CardId> {
        (way != 0).then(|| self.cx.card_id(face))
    }
}

/// "You may cast this spell as though it had flash if [condition]" (CR 307.5, 702.8): the card's own conditional permission
/// (`CardDef::flash_if`), judged for `player`, who would cast it, now - as the casting would start (the rulings: losing what the
/// condition asks for during the casting, or in response to the spell, changes nothing).
fn flash_if_met(cx: &Cx, def: &CardDef, card: ObjectId, player: PlayerId) -> bool {
    def.flash_if.is_some_and(|condition| condition(cx, &Asker { source: card, controller: player, by: None, spell: true }))
}

/// An instant, or a spell with flash (CR 307.5, 702.8).
fn own_speed(def: &CardDef) -> bool {
    def.is(CardType::Instant) || def.keywords.has(Keyword::Flash)
}

/// The face of the card `whole` a cast by the way `way` casts (CR 709.3, 715.3, 720.3, 712.8a, 702.103b): an inset face (an
/// Adventure, an Omen), a back face, a bestowed Aura, one half of a split card, the face-down values a morph or disguise
/// casts it as (CR 702.37c, 702.168b, 708.4), the prototyped face (CR 718.3b), or the card itself.
fn cast_face(whole: &'static CardDef, way: u8) -> &'static CardDef {
    if let Some(values) = face_down_values_of(whole, way) {
        return values;
    }
    let back = if way == WAY_BACK_FACE || way == WAY_TRANSFORMED { whole.back } else { None };
    let bestow = if way == WAY_BESTOW { whole.bestow } else { None };
    let half = match (way, whole.split) {
        (WAY_SPLIT_LEFT, Some((left, _))) => Some(left),
        (WAY_SPLIT_RIGHT, Some((_, right))) => Some(right),
        _ => None,
    };
    let prototyped = if way == WAY_PROTOTYPE { cantrip_api::prototyped_face(whole) } else { None };
    whole.inset_face(way).or(back).or(bestow).or(half).or(prototyped).unwrap_or(whole)
}

/// The face-down values the card `whole` cast by its own way `way` is cast as, where that way casts it face down (CR 702.37c,
/// 702.168b, 708.4): a 2/2 creature with no text, no name, no subtypes and no mana cost for morph and megamorph
/// (`intrinsic::MORPHED`), with ward {2} for disguise (`intrinsic::DISGUISED`).
fn face_down_values_of(whole: &'static CardDef, way: u8) -> Option<&'static CardDef> {
    let w = whole.ways.get((way as usize).checked_sub(1)?)?;
    match w.kind {
        WayKind::Morph | WayKind::Megamorph => Some(&cantrip_api::intrinsic::MORPHED),
        WayKind::Disguise => Some(&cantrip_api::intrinsic::DISGUISED),
        _ => None,
    }
}

/// Any query, as a spell judged before it is cast reads the game (`AsFace`): the card has the characteristics of `face`.
struct QueryAsFace<'a> {
    q: &'a dyn Query,
    cards: &'a CardTable,
    spell: ObjectId,
    face: CardId,
}

impl Query for QueryAsFace<'_> {
    fn linked_card(&self, card: CardId) -> CardId {
        self.q.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.q.text_linked(source)
    }

    fn state(&self) -> &GameState {
        self.q.state()
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.q.card(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        if id == self.spell { layers::chars_as_face(self.cards, self.q.state(), self.q.events(), id, self.face) } else { self.q.chars(id) }
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        self.q.token_chars(card, copy)
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.q.card_id(def)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        self.q.lost_all_abilities(id)
    }

    fn events(&self) -> &[Record] {
        self.q.events()
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        self.q.can_target(asker, target)
    }
}

/// The alternative costs among `all` (`spell_alternatives_of`) that `player` may pay rather than the mana cost of `card` cast
/// as `face` (CR 118.9, 601.2b), `whole` when that is the card itself: judged on that face (CR 709.3a, 715.3a, 712.8a), in
/// the zone the card is in, the spell's characteristics those it would have on the stack under `player` where they are not
/// the card's there (`spell`, `SpellsAsCast`: Dracogenesis's "Dragon spells" finds an opponent's creature card the caster's
/// Arcane Adaptation makes a Dragon as their spell). One that two permanents of a player give (two copies of a card) is offered once, by the first,
/// since paying it is one choice.
#[allow(clippy::too_many_arguments)]
fn alternatives_for(
    q: &dyn Query,
    cards: &CardTable,
    all: &[Given<SpellAlternative>],
    player: PlayerId,
    card: ObjectId,
    face: &'static CardDef,
    whole: bool,
    spell: Option<Chars>,
) -> Vec<Given<SpellAlternative>> {
    if all.is_empty() {
        return Vec::new();
    }
    let view = (!whole).then(|| QueryAsFace { q, cards, spell: card, face: cards.id_of(face) });
    let at_face: &dyn Query = match &view {
        Some(view) => view,
        None => q,
    };
    let as_spell = spell.map(|chars| AsSpell { q: at_face, spell: card, chars });
    let judged: &dyn Query = match &as_spell {
        Some(view) => view,
        None => at_face,
    };
    let mut applying: Vec<Given<SpellAlternative>> = Vec::new();
    for given in all {
        if applying.iter().any(|a| a.controller == given.controller && std::ptr::eq(a.what, given.what)) {
            continue;
        }
        if (given.what.applies)(judged, &given.asker(), card, player) {
            applying.push(*given);
        }
    }
    applying
}

/// The player a question is waiting on, or the one with priority: whose options a menu row describes.
fn asked_player(state: &GameState) -> PlayerId {
    state.agenda.iter().rev().find_map(|f| f.pending.as_ref().map(|r| r.player)).unwrap_or(state.turn.priority)
}

/// Which faces a permission lets be cast (CR 601.3e): each is judged as the spell it would be (CR 709.3a, 715.3a, 712.8a).
#[derive(Clone, Copy)]
enum Allows {
    /// Every face.
    Any,
    /// A permanent's permission to cast cards in exile (Dawnhand Dissident's "creature spells").
    Exile(Asker, &'static ExileCast),
    /// A permanent's rule for the top of the library (Mystic Forge's "artifact spells and colorless spells").
    Top(Asker, fn(&dyn Query, &Asker, ObjectId) -> bool),
    /// A way a permanent gives that is a permission for the face's mana cost (Lurrus's "a permanent spell with mana value 2
    /// or less").
    Way(Asker, &'static GrantedWay),
}

impl Allows {
    fn face(&self, q: &dyn Query, card: ObjectId, player: PlayerId) -> bool {
        match *self {
            Allows::Any => true,
            Allows::Exile(asker, c) => (c.allows)(q, &asker, card, player),
            Allows::Top(asker, f) => f(q, &asker, card),
            Allows::Way(asker, w) => (w.cost)(q, &asker, card, player).is_some(),
        }
    }
}

/// What a permission lets be cast of a card (CR 601.3).
#[derive(Clone, Copy, PartialEq, Eq)]
enum RouteKind {
    /// The card and each face of it the permission allows, for its mana cost or any alternative cost (CR 601.2b, 118.9):
    /// a hand's, a play permission's, a permanent's, a rule's, the card's own say-so, retrace.
    Permission,
    /// A card exiled on an Adventure, by the player who exiled it (CR 715.3d): the card itself, not the Adventure.
    Adventure,
    /// A prepare spell's copy, by the prepared permanent's controller (CR 722.3): the copy, for its cost.
    Prepared,
}

/// A permission a card is cast by from where it is (CR 601.3), with what it says of the cast: what `castable_ways` offers
/// each cast by, and what `act` builds again from the action's `Grant` to cast by, so the cast is made on the terms it was
/// offered on. `by` is the `Grant` (none for the rules a player has for their own hand, a card exiled on an Adventure or a
/// prepare spell's copy, and a card's own "you may cast this card from your graveyard").
#[derive(Clone, Copy)]
struct Route {
    by: Option<Grant>,
    kind: RouteKind,
    /// What the permission says of the cast (CR 601.3, 118.9, 118.8): what to pay instead, life equal to the mana value,
    /// generic mana more, mana of any type, flashback, sorcery speed, the Adventure only.
    terms: PlayGrant,
    /// What is owed on top of every cast by it (CR 118.8): a permanent's permission's (Dawnhand Dissident), retrace's
    /// discard.
    additional: Cost,
    allows: Allows,
}

impl Route {
    /// The rules every player has for their own hand.
    const RULES: Route = Route { by: None, kind: RouteKind::Permission, terms: PlayGrant::CAST, additional: Cost::NONE, allows: Allows::Any };

    const fn permission(by: Option<Grant>, terms: PlayGrant) -> Route {
        Route { by, terms, ..Route::RULES }
    }

    /// A permanent's permission to cast cards in exile (CR 601.3, 604.1), judged on each face: what it adds is owed with every
    /// cast by it (CR 118.8), it may let mana of any type pay (CR 609.4b), and it may name life equal to the mana value as the
    /// cost paid rather than the mana cost (CR 118.9, 119.4).
    fn exile_cast(c: &Given<ExileCast>) -> Route {
        Route {
            by: Some(c.grant),
            terms: PlayGrant { any_type: c.what.any_type, life_for_mana_value: c.what.life_for_mana_value, ..PlayGrant::CAST },
            additional: c.what.additional,
            allows: Allows::Exile(c.asker(), c.what),
            ..Route::RULES
        }
    }

    /// Retrace (CR 702.81a): a cast from the graveyard for its ordinary costs and a land card discarded.
    fn retrace(card: ObjectId, printed: CardId) -> Route {
        Route { by: Some(Grant { object: card, ability: AbilityRef::retrace(printed) }), additional: RETRACE, ..Route::RULES }
    }

    /// Life equal to the face's mana value rather than its mana cost (CR 118.9, 119.4).
    fn life(&self) -> bool {
        self.terms.life_for_mana_value
    }

    /// What it names to pay instead of the mana cost (CR 118.9): its own alternative cost, so no other may be paid with it
    /// (CR 118.9a). Flashback a permission gives is one too, though its cost is the mana cost (CR 702.34a; Snapcaster
    /// Mage's ruling).
    fn imposes(&self) -> bool {
        self.terms.instead.is_some() || self.terms.life_for_mana_value || self.terms.flashback
    }

    /// The mana it has the face cast for, before what the cast adds: what it names instead, or the face's mana cost.
    fn base(&self, face: &CardDef) -> Option<ManaCost> {
        if self.life() {
            Some(ManaCost::ZERO)
        } else {
            self.terms.instead.or(face.has_cost.then_some(face.cost))
        }
    }
}

/// What one pass of `castable_ways` looks up once and every card's casts read.
struct Offer<'a> {
    cx: &'a Cx<'a>,
    player: PlayerId,
    instants: bool,
    available: &'a std::cell::OnceCell<Available>,
    /// What lets the player cast spells from their graveyard (Yawgmoth's Will), each its own permission.
    graveyard: Vec<Grant>,
    granted: std::cell::OnceCell<Vec<Given<GrantedWay>>>,
    exile_casts: std::cell::OnceCell<Vec<Given<ExileCast>>>,
    alternatives: std::cell::OnceCell<Vec<Given<SpellAlternative>>>,
}

impl<'a> Offer<'a> {
    fn available(&self) -> &Available {
        self.available.get_or_init(|| available(self.cx, self.player))
    }

    fn granted(&self) -> &[Given<GrantedWay>] {
        self.granted.get_or_init(|| granted_ways_of(self.cx.cards(), self.cx.s(), &self.cx.g.log))
    }

    fn exile_casts(&self) -> &[Given<ExileCast>] {
        self.exile_casts.get_or_init(|| exile_casts_of(self.cx.cards(), self.cx.s(), &self.cx.g.log))
    }

    fn alternatives(&self, card: ObjectId, face: &'static CardDef, whole: bool) -> Vec<Given<SpellAlternative>> {
        let all = self.alternatives.get_or_init(|| spell_alternatives_of(self.cx.cards(), self.cx.s(), &self.cx.g.log));
        if all.is_empty() {
            return Vec::new();
        }
        let spell = spell_chars(self.cx, card, (!whole).then(|| self.cx.card_id(face)));
        alternatives_for(self.cx, self.cx.cards(), all, self.player, card, face, whole, spell)
    }

    /// Whether an alternative cost another object gives lets a spell cast for it be cast as though it had flash (Primal
    /// Prayers): a card that could not be cast on the instant-speed pass may be, for it.
    fn may_flash(&self) -> bool {
        self.alternatives.get_or_init(|| spell_alternatives_of(self.cx.cards(), self.cx.s(), &self.cx.g.log)).iter().any(|a| a.what.flash)
    }

    fn speed(&'a self, card: ObjectId) -> Speed<'a> {
        Speed { cx: self.cx, card, player: self.player }
    }

    fn asker(&self, card: ObjectId) -> Asker {
        Asker { source: card, controller: self.player, by: None, spell: true }
    }

    /// Offers casting `face` of `card` (`way` its way number) by `route`, for `base` mana, with `paid_by`'s cost paid
    /// instead of the mana cost where there is one, if the cast could be paid (advice, CR 601.2g-h): what the route adds is
    /// owed with it (CR 118.8), and the cost changes apply to it (CR 601.2f).
    #[allow(clippy::too_many_arguments)]
    fn offer(&self, card: ObjectId, face: &'static CardDef, way: u8, route: &Route, base: ManaCost, paid_by: Option<Grant>, options: &mut Vec<ActionOption>) {
        let (cx, player) = (self.cx, self.player);
        let a = self.available();
        // What the permission adds is part of the total before the cost changes apply, as the cast adds it (CR 118.8, 601.2f).
        let mut base = base;
        base.generic = base.generic.saturating_add(route.terms.more);
        let base = add_cost(base, route.additional.mana);
        let covered = |targets: &[Target], earlier: &Earlier| {
            // Mana of any type pays every symbol of it (CR 609.4b), by the permission that casts it.
            least_cost(cx, a, player, card, face, base, targets, earlier, &mut |least| {
                covers_total(cx, a, player, card, face, least, route.additional.exile_from_graveyard, route.terms.any_type)
            })
                && added_life_payable(cx, a, player, card, face, targets, earlier)
        };
        let mut option = ActionOption { granted_by: route.by, paid_by, ..ActionOption::on(ActionKind::CastSpell, card, way) };
        if offer_costed(cx, a, &self.asker(card), face, base.x, &mut option, covered) {
            options.push(option);
        }
    }

    /// Offers casting `face` of `card` by `route` for its mana cost (or what the route names instead), and for each
    /// alternative cost another object gives (CR 118.9) where the route names none, each at its speed: the face's own, or
    /// "as though it had flash" where an alternative cost cast says so (Primal Prayers).
    ///
    /// `instant` is `None` where whether a permission lets it be cast as though it had flash is not yet judged on the spell
    /// (`Speed::of_judged`), on the instant-speed pass: offered as though it had not, and asked for (`ask`) only where it could
    /// then be offered more - where some cast at instant speed could be paid - since a judgment puts the card on the stack
    /// for the moment (`SpellsAsCast`).
    #[allow(clippy::too_many_arguments)]
    fn offer_costs(&self, card: ObjectId, face: &'static CardDef, way: u8, route: &Route, instant: Option<bool>, ask: impl Fn(), options: &mut Vec<ActionOption>) {
        let Some(instant) = instant else {
            let mut would = Vec::new();
            self.offer_costs_at(card, face, way, route, true, &mut would);
            if !would.is_empty() {
                ask();
                self.offer_costs_at(card, face, way, route, false, options);
            }
            return;
        };
        self.offer_costs_at(card, face, way, route, instant, options);
    }

    /// `offer_costs` at a known speed.
    fn offer_costs_at(&self, card: ObjectId, face: &'static CardDef, way: u8, route: &Route, instant: bool, options: &mut Vec<ActionOption>) {
        if instant == self.instants
            && let Some(base) = route.base(face)
        {
            self.offer(card, face, way, route, base, None, options);
        }
        if route.imposes() {
            return;
        }
        for alternative in self.alternatives(card, face, way == 0) {
            let flash = alternative.what.flash && !route.terms.sorcery_speed;
            let cost = alternative.what.cost.cost();
            if (instant || flash) == self.instants && can_pay_other(self.cx, card, self.player, &cost.and(route.additional)) {
                self.offer(card, face, way, route, cost.mana, Some(alternative.grant), options);
            }
        }
    }

    /// Offers the card's own alternative ways (CR 118.9) that `open` lets it be cast by from here, by `route`: those naming
    /// this zone of its owner's (flashback from a graveyard, evoke from a hand), or, by a permission to cast it from elsewhere,
    /// those that name no zone (Force of Will's, web-slinging, sneak; not warp, CR 702.185a).
    fn offer_ways(&self, card: ObjectId, def: &'static CardDef, route: &Route, open: impl Fn(&WayDef) -> bool, options: &mut Vec<ActionOption>) {
        let (cx, player) = (self.cx, self.player);
        // Asked only of a way that is open and goes at the card's speed.
        let speed = std::cell::OnceCell::new();
        let instant = || *speed.get_or_init(|| self.speed(card).whole(def) && !route.terms.sorcery_speed);
        let asker = self.asker(card);
        for (n, way) in def.ways.iter().enumerate() {
            if !open(way) {
                continue;
            }
            // Cast face down by morph or disguise (CR 702.37c, 702.168b, 708.4), it is the face-down spell and nothing else: what
            // a permission allows (CR 601.3e), its speed - a 2/2 creature spell with no text, so with no flash of its own, or
            // one a permission judged on that spell gives it (CR 307.5) - its additional costs (none) and how its mana may
            // be paid are that spell's.
            let face = face_down_values_of(def, (n + 1) as u8).unwrap_or(def);
            let face_down = !std::ptr::eq(face, def);
            if face_down && !matches!(route.allows, Allows::Any) && !route.allows.face(&AsFace { cx, spell: card, face: cx.card_id(face) }, card, player) {
                continue;
            }
            // Sneak and the like may be used whenever an instant could be cast, whatever the card is (CR 702.190a), unless
            // the permission says when (CR 601.3).
            let on_instants = if way.instant_speed {
                !route.terms.sorcery_speed
            } else if face_down {
                self.speed(card).face(face) && !route.terms.sorcery_speed
            } else {
                instant()
            };
            if on_instants != self.instants || (way.instant_speed && route.terms.sorcery_speed) {
                continue;
            }
            if way.condition.is_some_and(|condition| !condition(cx, &asker)) {
                continue;
            }
            // An additional cost is owed with every way of casting it (CR 118.8, 601.2f).
            if !can_pay_other(cx, card, player, &way.cost) || !extra_payable(cx, card, player, face) || !can_pay_other(cx, card, player, &route.additional) {
                continue;
            }
            let mut cost = way.cost.and(route.additional);
            cost.mana.generic = cost.mana.generic.saturating_add(route.terms.more);
            if !way_covered(cx, self.available(), player, card, face, &cost, route.terms.any_type) {
                continue;
            }
            let mut option = ActionOption { granted_by: route.by, ..ActionOption::on(ActionKind::CastSpell, card, (n + 1) as u8) };
            if offer_spell(cx, &asker, face, &mut option) {
                options.push(option);
            }
        }
    }

    /// The casts a permission (`route`) lets be made of `card` from where it is (CR 601.3): the card itself and each face of
    /// it the permission allows, for the mana cost or an alternative cost (CR 118.9), and the card's own alternative ways that
    /// `ways` opens. What the permission names instead is the one alternative cost (CR 118.9a).
    fn by_route(&self, card: ObjectId, def: &'static CardDef, route: &Route, ways: impl Fn(&WayDef) -> bool, options: &mut Vec<ActionOption>) {
        let (cx, player) = (self.cx, self.player);
        if route.by.is_some() && !can_pay_other(cx, card, player, &route.additional) {
            return;
        }
        let land = def.is(CardType::Land);
        let speed = self.speed(card);
        let mut whole_allowed = false;
        for (way, face) in faces_to_cast(def) {
            // A land card is played, never cast (CR 305.1, 305.9); an aftermath half only from a graveyard by its own say-so
            // (CR 702.127a).
            if (way == 0 && land) || face.aftermath {
                continue;
            }
            if way != 0 && route.kind != RouteKind::Permission {
                continue;
            }
            // A permission to cast it only as its Adventure (CR 715.3) casts neither the card itself nor another face of it.
            if route.terms.adventure_only && way != WAY_ADVENTURE {
                continue;
            }
            // Flashback a permission gives casts the card only "if the resulting spell is an instant or sorcery spell" (CR
            // 702.34a): judged on the face cast (CR 709.3a, 712.8a, 715.3a).
            if route.terms.flashback && !instant_or_sorcery(face) {
                continue;
            }
            // Judged as the face cast (CR 601.3e); the card itself is read as it is.
            let allowed = if way == 0 || matches!(route.allows, Allows::Any) {
                route.allows.face(cx, card, player)
            } else {
                route.allows.face(&AsFace { cx, spell: card, face: cx.card_id(face) }, card, player)
            };
            if !allowed {
                continue;
            }
            whole_allowed |= way == 0;
            // Life equal to the face's own mana value (CR 118.9, 119.4), which has to be payable.
            if route.life() && !can_pay_other(cx, card, player, &life_for_mana_value(face)) {
                continue;
            }
            if !extra_payable(cx, card, player, face) {
                continue;
            }
            let instant = if route.terms.sorcery_speed { Some(false) } else { speed.of_judged(def, way, face) };
            self.offer_costs(card, face, way, route, instant, || speed.ask(way, face), options);
        }
        // A way that casts the card face down is allowed by what the permission allows of the face-down spell (CR 601.3e,
        // 708.4), judged as it is offered; every other way by what it allows of the card.
        if !route.imposes() && route.kind != RouteKind::Prepared {
            self.offer_ways(card, def, route, |way| (whole_allowed || way.kind.face_down()) && ways(way), options);
        }
    }

    /// Every cast `player` may begin of `card` in `zone` on this pass: by the rules from their hand, by the card's own ways
    /// from where it is, by each permission to cast it from there, and by each way a permanent gives.
    fn card(&self, card: ObjectId, zone: Zone, options: &mut Vec<ActionOption>) {
        let (cx, player) = (self.cx, self.player);
        // A card exiled face down is judged as the spell it would be (CR 406.3a-b).
        let _face_up = layers::judged_face_up(cx.s(), card);
        let def = cx.card(cx.s().obj(card).card);
        // A land card is played, never cast (CR 305.1, 305.9), but one with an Adventure may be cast as that (CR 715.3:
        // Lindblum, Industrial Regency's Mage Siege), and only as that.
        if def.is(CardType::Land) && def.inset_faces().next().is_none() {
            return;
        }
        if !cast_only_if_met(cx, def, &self.asker(card)) {
            return;
        }
        let own = zone.player == player;
        match zone.kind() {
            ZoneKind::Hand if own => {
                // From the hand, the card and its faces for their costs and the ways that name the hand (CR 601.3).
                self.by_route(card, def, &Route::RULES, |way| way.from == ZoneKind::Hand, options);
                self.fused(card, def, options);
                self.overload(card, def, options);
                self.bestow(card, def, options);
            }
            ZoneKind::Graveyard if own => {
                // From the graveyard by the card's own ways that name it (flashback, CR 702.34a), an aftermath half (CR
                // 702.127a), bestow where it may be cast from there.
                self.offer_ways(card, def, &Route::RULES, |way| way.from == ZoneKind::Graveyard, options);
                self.aftermath(card, def, options);
                if def.bestow_from_graveyard {
                    self.bestow(card, def, options);
                }
            }
            _ => {}
        }
        for route in self.routes(card, zone, def) {
            self.by_route(card, def, &route, |way| way.from == ZoneKind::Hand && !way.only_from, options);
        }
        if own && matches!(zone.kind(), ZoneKind::Graveyard | ZoneKind::Hand) {
            self.granted_ways(card, def, zone, options);
        }
    }

    /// The permissions to cast `card` from `zone` beyond its owner's hand (CR 601.3), each a route of its own: the card's own
    /// say-so or an effect's for the graveyard, retrace, a way a permanent gives that is a permission, an Adventure's exile,
    /// a prepare spell's copy, a permanent's permission to cast from exile, and each play permission an effect gave.
    fn routes(&self, card: ObjectId, zone: Zone, def: &'static CardDef) -> Vec<Route> {
        let (cx, player) = (self.cx, self.player);
        let own = zone.player == player;
        let obj = cx.s().obj(card);
        let mut routes = Vec::new();
        match zone.kind() {
            ZoneKind::Graveyard if own => {
                // The card's own say-so, for its ordinary cost (CR 113.6b): "You may cast this card from your graveyard"; or a
                // rule that lets its owner cast spells from their graveyard (Yawgmoth's Will), which says nothing more.
                // Each is a permission of its own (two Yawgmoth's Wills are two).
                if def.keywords.has(Keyword::CastFromYourGraveyard) {
                    routes.push(Route::RULES);
                }
                routes.extend(self.graveyard.iter().map(|&by| Route::permission(Some(by), PlayGrant::CAST)));
                // Retrace (CR 702.81a), printed or given: a permission of the card's own, for a land card discarded. Its speed
                // is asked only of a card with retrace that could pay for it.
                if !def.is(CardType::Land)
                    && cx.chars(card).has(Keyword::Retrace)
                    && can_pay_other(cx, card, player, &RETRACE)
                    && (self.speed(card).whole(def) == self.instants || (self.instants && self.may_flash()))
                {
                    routes.push(Route::retrace(card, obj.card));
                }
                // A way a permanent gives that is a permission (Lurrus of the Dream-Den), judged on each face.
                for way in self.granted() {
                    if way.what.permission && way.what.from == ZoneKind::Graveyard {
                        routes.push(Route { by: Some(way.grant), allows: Allows::Way(way.asker(), way.what), ..Route::RULES });
                    }
                }
            }
            ZoneKind::Exile => {
                // A prepare spell's copy, by the prepared permanent's controller (CR 722.3).
                if obj.card_copy
                    && obj.prepared_from.is_some_and(|from| {
                        let f = cx.s().obj(from);
                        live(cx.s(), from) && f.zone.is(ZoneKind::Battlefield) && f.prepared && f.controller == player
                    })
                {
                    routes.push(Route { kind: RouteKind::Prepared, ..Route::RULES });
                }
                // A card exiled on an Adventure, by the player who exiled it (CR 715.3d).
                if obj.on_adventure == Some(player) {
                    routes.push(Route { kind: RouteKind::Adventure, ..Route::RULES });
                }
                // A permanent's permission to cast cards in exile (CR 601.3, 604.1), judged on each face; what it adds is owed
                // with every cast by it (CR 118.8), and it may let mana of any type pay (CR 609.4b).
                if !obj.card_copy {
                    routes.extend(self.exile_casts().iter().map(Route::exile_cast));
                }
            }
            _ => {}
        }
        // A permission to play it from where it is (CR 601.3), each its own.
        if !zone.is(ZoneKind::Hand) && !cx.s().permissions.is_empty() {
            routes.extend(permissions_for(cx, card, player).map(|p| Route::permission(Some(p.by), p.grant)));
        }
        routes
    }

    /// Overload (CR 702.96a): from the hand, for its overload cost. It has no targets, so it is offered whether or not there
    /// is anything to target.
    fn overload(&self, card: ObjectId, def: &'static CardDef, options: &mut Vec<ActionOption>) {
        let Some(overload) = def.overload else { return };
        if self.speed(card).whole(def) != self.instants {
            return;
        }
        let (cx, player, a) = (self.cx, self.player, self.available());
        if least_cost(cx, a, player, card, def, overload, &[], &Earlier::NONE, &mut |cost| covers_total(cx, a, player, card, def, cost, 0, false)) {
            options.push(ActionOption::on(ActionKind::CastSpell, card, WAY_OVERLOAD));
        }
    }

    /// Fuse (CR 702.102a): a split card with fuse in its owner's hand cast as both halves at once, the fused split spell -
    /// the card itself, with its combined characteristics (CR 702.102b, 709.4) - for its combined mana cost (CR 702.102c) or
    /// an alternative cost another object gives (the ruling: Omniscience casts it fused for nothing), at the card's speed.
    /// Only by the rules from a hand: a permission to cast it from elsewhere casts one half (the ruling).
    fn fused(&self, card: ObjectId, def: &'static CardDef, options: &mut Vec<ActionOption>) {
        if !def.fuse || !def.split.is_some_and(|(left, right)| left.spell.is_some() && right.spell.is_some()) {
            return;
        }
        let instant = self.speed(card).whole(def);
        self.offer_costs_at(card, def, WAY_FUSED, &Route::RULES, instant, options);
    }

    /// Bestow (CR 702.103a): as the Aura it becomes, which needs a creature to enchant (CR 601.2c), at the Aura's speed.
    fn bestow(&self, card: ObjectId, def: &'static CardDef, options: &mut Vec<ActionOption>) {
        let Some(bestow) = def.bestow else { return };
        let (cx, player) = (self.cx, self.player);
        if self.speed(card).face(bestow) != self.instants || !bestow.additional_cost.is_none_or(|extra| can_pay_other(cx, card, player, &extra)) {
            return;
        }
        let a = self.available();
        let mut option = ActionOption::on(ActionKind::CastSpell, card, WAY_BESTOW);
        // The Aura's target is announced with it, and the total cost follows from it (CR 601.2c, 601.2f): judged for each.
        let covered = |targets: &[Target], earlier: &Earlier| {
            least_cost(cx, a, player, card, bestow, def.bestow_cost, targets, earlier, &mut |cost| spell_covers(cx, a, player, card, bestow, &cost))
        };
        if offer_costed(cx, a, &self.asker(card), bestow, def.bestow_cost.x, &mut option, covered) {
            options.push(option);
        }
    }

    /// A split card's aftermath half (CR 702.127a): from its owner's graveyard, by the card's own say-so, at its own speed.
    fn aftermath(&self, card: ObjectId, def: &'static CardDef, options: &mut Vec<ActionOption>) {
        let Some((left, right)) = def.split else { return };
        for (way, half) in [(WAY_SPLIT_LEFT, left), (WAY_SPLIT_RIGHT, right)] {
            if half.aftermath {
                let instant = self.speed(card).face(half);
                self.offer_costs_at(card, half, way, &Route::RULES, instant, options);
            }
        }
    }

    /// The ways a permanent gives a card in its owner's graveyard or hand to be cast that are its cost as well (CR 604,
    /// 118.9: Underworld Breach's escape, mayhem, Iroh's flashback): each judged on the face that would be cast, the only
    /// spell it would be on the stack (CR 709.3, 715.4, 712.8a), so a Room's door is chosen here and a restriction on the
    /// spell ("mana value 3 or less") reads that door's characteristics. A way that is a permission is a route.
    fn granted_ways(&self, card: ObjectId, def: &'static CardDef, zone: Zone, options: &mut Vec<ActionOption>) {
        let (cx, player) = (self.cx, self.player);
        let ways = self.granted();
        if !ways.iter().any(|w| !w.what.permission && w.what.from == zone.kind()) {
            return;
        }
        let land = def.is(CardType::Land);
        let speed = self.speed(card);
        for (way_number, face) in faces_to_cast(def).filter(|&(w, _)| !(land && w == 0)) {
            if speed.of(def, way_number, face) != self.instants {
                continue;
            }
            let view = (way_number != 0).then(|| AsFace { cx, spell: card, face: cx.card_id(face) });
            let q: &dyn Query = match &view {
                Some(view) => view,
                None => cx,
            };
            for way in ways {
                if way.what.permission || way.what.from != zone.kind() {
                    continue;
                }
                // Flashback a permanent gives: only if the resulting spell is an instant or sorcery spell (CR 702.34a).
                if way.what.flashback && !instant_or_sorcery(face) {
                    continue;
                }
                let Some(cost) = (way.what.cost)(q, &way.asker(), card, player) else { continue };
                if !can_pay_other(cx, card, player, &cost) || !extra_payable(cx, card, player, face) {
                    continue;
                }
                let a = self.available();
                let covered = |targets: &[Target], earlier: &Earlier| {
                    least_cost(cx, a, player, card, face, cost.mana, targets, earlier, &mut |least| covers_total(cx, a, player, card, face, least, cost.exile_from_graveyard, false))
                        && added_life_payable(cx, a, player, card, face, targets, earlier)
                };
                let mut option = ActionOption { granted_by: Some(way.grant), ..ActionOption::on(ActionKind::CastSpell, card, way_number) };
                if offer_costed(cx, a, &self.asker(card), face, cost.mana.x, &mut option, covered) {
                    options.push(option);
                }
            }
        }
    }
}

/// The casts `player` may begin on this pass (`instants`: the instant-speed one or the sorcery-speed one): from their hand,
/// their graveyard, any player's exile and the top of their library, each by everything that lets it be cast from there
/// (CR 601.3), every permission an option of its own.
/// A cast a static ability forbids for the qualities of its spell is left out after (`casts_forbidden_on_the_stack`).
fn castable_ways(cx: &Cx, player: PlayerId, available: &std::cell::OnceCell<Available>, instants: bool, options: &mut Vec<ActionOption>) {
    let barred = cast_barred(cx, player);
    if ZoneKinds::CAST_FROM.iter().all(|&from| barred.contains(from)) {
        return;
    }
    let offer = Offer {
        cx,
        player,
        instants,
        available,
        graveyard: graveyard_play(cx, player),
        granted: std::cell::OnceCell::new(),
        exile_casts: std::cell::OnceCell::new(),
        alternatives: std::cell::OnceCell::new(),
    };
    for zone in cast_zones(cx, player, barred) {
        for &card in cx.s().zone(zone) {
            offer.card(card, zone, options);
        }
    }
    // The top card of the library, for a player a permanent lets cast spells from there (CR 601.3): for life rather than its
    // mana cost (Bolas's Citadel, CR 118.9), or for its costs (Mystic Forge, Case of the Locked Hothouse), each its own.
    if let Some(top) = library_top_castable(cx, player, barred) {
        let mut routes = Vec::new();
        each_play_rule_named(cx, cx.cards(), player, false, |source, ability, rule| {
            let by = Some(Grant { object: source, ability });
            match rule {
                PlayRule::LibraryTop => routes.push(Route::permission(by, PlayGrant { life_for_mana_value: true, ..PlayGrant::CAST })),
                PlayRule::LibraryTopSpells(f) => {
                    routes.push(Route { by, allows: Allows::Top(Asker { source, controller: player, by: None, spell: false }, *f), ..Route::RULES })
                }
                _ => {}
            }
        });
        if !routes.is_empty() {
            let def = cx.card(cx.s().obj(top).card);
            if (!def.is(CardType::Land) || def.inset_faces().next().is_some()) && cast_only_if_met(cx, def, &offer.asker(top)) {
                for route in &routes {
                    offer.by_route(top, def, route, |way| way.from == ZoneKind::Hand && !way.only_from, options);
                }
            }
        }
    }
}

/// The zones `player` may cast from other than a library (CR 601.3): any player's exile, for a card a permission lets them
/// cast from there; their own hand and graveyard. A zone a rule forbids them to cast from (`barred`, CR 101.2) is not looked in.
fn cast_zones<'a>(cx: &'a Cx, player: PlayerId, barred: ZoneKinds) -> impl Iterator<Item = Zone> + 'a {
    [Zone::hand(player), Zone::graveyard(player)].into_iter().chain(cx.s().seats().map(Zone::exile)).filter(move |zone| !barred.contains(zone.kind()))
}

/// The top card of `player`'s library, where a permanent may let them cast spells from there (CR 601.3) and no rule forbids
/// them to cast from a library (`barred`, CR 101.2).
fn library_top_castable(cx: &Cx, player: PlayerId, barred: ZoneKinds) -> Option<ObjectId> {
    if !cx.cards().any_play_rules || barred.contains(ZoneKind::Library) {
        return None;
    }
    cx.s().zone(Zone::library(player)).first().copied()
}

/// What a cast is made by, as `act` reads it back from the action's `Grant` (`ActionOption::granted_by`): a permission's
/// route, or a way a permanent gives that is the cast's cost (escape, mayhem, Iroh's flashback). `None` where the grant names
/// nothing that lets the card be cast now.
enum Through {
    Route(Route),
    Way(Given<GrantedWay>),
}

fn through(q: &dyn Query, cards: &CardTable, player: PlayerId, card: ObjectId, by: Option<Grant>) -> Option<Through> {
    let s = q.state();
    let Some(g) = by else {
        let obj = s.obj(card);
        let kind = if obj.zone.is(ZoneKind::Exile) && obj.on_adventure == Some(player) {
            RouteKind::Adventure
        } else if obj.zone.is(ZoneKind::Exile) && obj.card_copy && obj.prepared_from.is_some() {
            RouteKind::Prepared
        } else {
            RouteKind::Permission
        };
        return Some(Through::Route(Route { kind, ..Route::RULES }));
    };
    if g.ability.is_retrace() {
        return Some(Through::Route(Route::retrace(card, g.ability.card)));
    }
    if let Some(way) = named_way(s, cards, g) {
        return Some(if way.what.permission {
            Through::Route(Route { by, allows: Allows::Way(way.asker(), way.what), ..Route::RULES })
        } else {
            Through::Way(way)
        });
    }
    if let Some(c) = named_exile_cast(s, cards, g) {
        return Some(Through::Route(Route::exile_cast(&c)));
    }
    if let Some(rule) = play_rule_at(cards, g.ability) {
        let asker = Asker { source: g.object, controller: player, by: None, spell: false };
        return match rule {
            PlayRule::LibraryTop => Some(Through::Route(Route::permission(by, PlayGrant { life_for_mana_value: true, ..PlayGrant::CAST }))),
            PlayRule::LibraryTopSpells(f) => Some(Through::Route(Route { by, allows: Allows::Top(asker, *f), ..Route::RULES })),
            _ => None,
        };
    }
    if let Some(p) = s.permissions.iter().find(|p| p.card == card && p.player == player && p.by == g) {
        return Some(Through::Route(Route::permission(by, p.grant)));
    }
    s.effects
        .iter()
        .any(|e| e.kind == EffectKind::PlayFromGraveyard && e.source == g.object && e.controller == player)
        .then_some(Through::Route(Route::permission(by, PlayGrant::CAST)))
}

/// The life a cast pays instead of its mana cost (CR 118.9, 119.4), for the menu row and the seats that weigh it: a
/// permission's life equal to the face's mana value (Inside Information, Bolas's Citadel's for the top of the library). None
/// for a cast paid in mana.
pub fn life_for_cast(q: &dyn Query, cards: &CardTable, player: PlayerId, option: &ActionOption) -> Option<u8> {
    let card = option.source?;
    let Some(Through::Route(route)) = through(q, cards, player, card, option.granted_by) else { return None };
    route.life().then(|| u8::try_from(life_for_mana_value(cast_face(cards.get(q.state().obj(card).card), option.index)).life).unwrap_or(u8::MAX))
}

/// "Pay life equal to its mana value rather than pay its mana cost" (CR 118.9, 119.4): the life, as a cost, for a card or the
/// face of it cast, its X counted as 0 (CR 107.3). Bolas's Citadel's rule and Inside Information's permission pay it.
fn life_for_mana_value(def: &CardDef) -> Cost {
    Cost { life: def.cost.mana_value(), ..Cost::NONE }
}

/// The most an emerge cost could come down by right now (CR 702.119a): the mana values of the richest permanents it could
/// sacrifice. What it comes down by is not known until the player has chosen, so this is only what makes the cast worth
/// offering.
fn emerge_reduction(cx: &Cx, player: PlayerId, card: ObjectId, cost: &Cost) -> u8 {
    let Some(p) = cost.permanents.filter(|_| cost.emerge) else { return 0 };
    let asker = Asker { source: card, controller: player, by: None, spell: true };
    let mut values: Vec<u32> =
        cx.permanents_of(player).into_iter().filter(|&id| pays_permanents(cx, cost, &p, &asker, id, Some(CostFor::Spell))).map(|id| cx.mana_value(id)).collect();
    values.sort_unstable_by(|a, b| b.cmp(a));
    values.iter().take(p.count as usize).sum::<u32>().min(255) as u8
}

/// The creatures a cost that taps one to reduce its mana may tap (CR 702.180a): the controller's untapped creatures. A
/// creature with summoning sickness may be tapped, since this is not its own {T} ability (CR 302.6, 107.5).
fn reducer_candidates(cx: &Cx, controller: PlayerId) -> Vec<ObjectId> {
    cx.permanents_of(controller).into_iter().filter(|&id| !cx.s().obj(id).tapped && cx.chars(id).is(CardType::Creature)).collect()
}

/// What tapping `creature` takes off the generic mana of the total cost (CR 702.180a): its power, none if that is below 0.
fn reduction_of(cx: &Cx, creature: ObjectId) -> u8 {
    cx.chars(creature).power.unwrap_or(0).clamp(0, 255) as u8
}

/// Whether a way of casting `card` is worth offering (advice, CR 601.2g): the least its mana can come to, with emerge's
/// richest sacrifice, is covered; or, for harmonize (CR 702.180a), is covered once some untapped creature is tapped for
/// its power, that creature then not tapping for mana.
fn way_covered(cx: &Cx, a: &Available, player: PlayerId, card: ObjectId, def: &CardDef, way: &Cost, any_type: bool) -> bool {
    // The way's own reductions (emerge, harmonize) come before the least the total may come to (Trinisphere), as the cast
    // applies them (CR 601.2f).
    least_cost_parts(cx, a, player, card, def, way.mana, &[], &Earlier::NONE, &mut |mut cost, least| {
        cost.generic = cost.generic.saturating_sub(emerge_reduction(cx, player, card, way));
        // Mana of any type pays every symbol of it (CR 609.4b), as `cast` pays it.
        if covers_total(cx, a, player, card, def, at_least(cost, least), way.exile_from_graveyard, any_type) {
            return true;
        }
        way.tap_to_reduce
            && reducer_candidates(cx, player).into_iter().any(|creature| {
                let without = available_without(cx, player, Some(creature), ManaWindow::Payment);
                let mut reduced = cost;
                reduced.generic = reduced.generic.saturating_sub(reduction_of(cx, creature));
                covers_total(cx, &without, player, card, def, at_least(reduced, least), way.exile_from_graveyard, any_type)
            })
    })
}

/// Chooses the creature a cost taps to reduce its mana (CR 702.180b, 601.2b): up to one untapped creature, asked as the cost
/// is chosen, before any target. Nothing is asked when there is no creature to tap.
///
/// The prompt says what tapping each creature takes off: its power, but never more than the generic mana the total cost will
/// have (CR 702.180a), since colored mana does not come down (CR 601.2f). That total is only settled once X, a kicker, the
/// modes a payment decides, splicing and the targets are announced (CR 601.2b-f), all after this question: while any of them
/// could still change it, or the hybrid symbols could be announced into different generic mana (CR 107.4e, 118.7e), the most
/// tapping it could take is its power, and the prompt says "up to" (`up_to`). Settled, the cost changes in play are applied
/// and the prompt says exactly min(power, generic).
fn choose_reducer(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &Cost, later: bool, targeted: bool) -> R<Option<ObjectId>> {
    if !cost.tap_to_reduce {
        return Ok(None);
    }
    let eligible = reducer_candidates(cx, player);
    if eligible.is_empty() {
        return Ok(None);
    }
    // Before any target is chosen (CR 601.2b), so a change that reads the targets is not in it: `targeted` is a spell with
    // targets that a cost change (its own, or one in play) could read. Every way a reduction by hybrid mana is split and the
    // hybrid symbols are announced (CR 118.7e, 601.2b), asked after it, comes to the same generic mana, or it is unsettled
    // too. The least the total may come to (Trinisphere) is applied after the reduction (CR 601.2f), so not here.
    let (mut most, mut fewest) = (0u8, u8::MAX);
    change_any(cx, &cost_change_sources(cx), player, Costed::Spell(source), cost.mana, None, &[], None, &mut |c, _least| {
        most = most.max(c.generic);
        fewest = fewest.min(c.generic);
        false
    });
    let up_to = later || most != fewest || targeted;
    let powers = eligible
        .iter()
        .map(|&c| i16::from(if up_to { reduction_of(cx, c) } else { reduction_of(cx, c).min(most) }))
        .collect();
    let ask = RequestKind::TapForPower { source, purpose: Purpose::TapToReduce, cards: eligible, powers, minimum: 0, up_to };
    match cx.ask_core(player, ask, true)? {
        Answer::Cards(cards) => Ok(cards.first().copied()),
        _ => Err(Stop::Cancel),
    }
}

fn is_land(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Land)
}

/// The additional cost of retrace (CR 702.81a): discard a land card.
const RETRACE: Cost = Cost { discard: Discards::one(1, Some(is_land)), ..Cost::NONE };

/// Whether a permanent has a mana ability that could be activated now: one more way to spend its tap, which a creature
/// or artifact tapped to pay for convoke or improvise cannot also be used for.
fn has_mana_source(cx: &Cx, id: ObjectId, player: PlayerId) -> bool {
    let forbidders = static_rules::activation_forbidders(cx);
    let chars = cx.chars(id);
    let given = layers::acquisition_possible(cx.cards(), cx.s());
    chars.abilities().enumerate().any(|(index, ability)| {
        let by = if given { cx.link_nth(id, &chars, index) } else { None };
        activated(cx, ability).is_some_and(|def| {
            def.is_mana_ability()
                && ManaWindow::Payment.allows(def)
                && !limit_reached(cx, id, &chars, index, def)
                && can_pay_activation(cx, id, player, &def.cost, Some(CostFor::ManaAbility), by)
                && !static_rules::activation_barred(cx, &forbidders, id, def)
        })
    })
}

/// The permanents on the battlefield that give spells convoke, delve or improvise (`CardDef::grants_pay_with`, CR 601.2f-h):
/// found from the board, as the cost changers are, and empty in nearly every game.
fn pay_with_grantors(cx: &Cx) -> Vec<ObjectId> {
    cx.s()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| cx.cards().pay_with_grants[cx.cards().current(cx.s().obj(id)).0 as usize] && !cx.lost_all_abilities(id))
        .collect()
}

/// What besides mana may pay the total cost of `spell`, cast by `player` as `def` (CR 601.2f-h): its own convoke, delve and
/// improvise (CR 702.51a, 702.66a, 702.126a), and those the permanents `grantors` give it ("noncreature spells you cast have
/// improvise"), read with its characteristics as that spell - as `face` (`AsFace`) when it is judged in a zone before it is
/// cast as one of its faces. Several instances of one are redundant (CR 702.51d, 702.66c, 702.126c).
fn pay_with(cx: &Cx, grantors: &[ObjectId], player: PlayerId, spell: ObjectId, def: &CardDef, face: Option<CardId>) -> PayWith {
    let mut with = def.pay_with;
    if grantors.is_empty() {
        return with;
    }
    let view;
    let q: &dyn Query = match face {
        Some(face) => {
            view = AsFace { cx, spell, face };
            &view
        }
        None => cx,
    };
    for &source in grantors {
        for grant in cx.card(cx.cards().current(cx.s().obj(source))).grants_pay_with {
            let given = grant(q, source, player, spell);
            with.convoke |= given.convoke;
            with.delve |= given.delve;
            with.improvise |= given.improvise;
        }
    }
    with
}

/// Whether a spell's total cost could be paid (advice, CR 601.2g-h): by the mana at hand, with what else may pay for it
/// (convoke's creatures, delve's graveyard, improvise's artifacts) each paying their share, or by those alone when no
/// mana may be spent on it. A permanent that is a mana source is counted as that, not twice. `exiled` is how many
/// cards another part of the cost exiles from the graveyard, which delve cannot also use. `any_type`: mana of any type may
/// pay it (CR 609.4b), any of what is left once the others have paid their share.
#[allow(clippy::too_many_arguments)]
fn covers_total(cx: &Cx, a: &Available, player: PlayerId, card: ObjectId, def: &CardDef, cost: ManaCost, exiled: u8, any_type: bool) -> bool {
    let no_mana = def.keywords.has(Keyword::CantSpendManaToCast);
    let with = pay_with(cx, &a.grantors, player, card, def, face_of(cx, card, def));
    let any_color = !any_type && casts_with_any_color(cx, a, player, card, def);
    let as_paid = |cost: ManaCost| if any_type { cost.paid_with_any_type() } else if any_color { cost.paid_with_any_color() } else { cost };
    // Assist (CR 702.132a): another player may pay generic mana of it - as much as the one who could pay the most.
    let cost = if with.assist && !no_mana && cost.generic > 0 { ManaCost { generic: cost.generic - assist_could_pay(cx, player, card, def, cost.generic), ..cost } } else { cost };
    if !with.any() && !no_mana {
        return a.covers_for(&as_paid(cost), &spell_kind(cx, a, card, def));
    }
    let mut cost = cost;
    let mut creatures: Vec<ColorSet> = Vec::new();
    let mut generic: u32 = 0;
    if with.delve {
        let gy = cx.s().zone(Zone::graveyard(player)).iter().filter(|&&c| c != card).count();
        generic += gy.saturating_sub(exiled as usize) as u32;
    }
    if with.improvise || with.convoke {
        for id in cx.permanents_of(player) {
            if cx.s().obj(id).tapped || (!no_mana && has_mana_source(cx, id, player)) {
                continue;
            }
            let chars = cx.chars(id);
            if with.improvise && chars.is(CardType::Artifact) {
                generic += 1;
            } else if with.convoke && chars.is(CardType::Creature) {
                creatures.push(chars.colors);
            }
        }
    }
    cost = convoked(&cost, &creatures);
    cost.generic = cost.generic.saturating_sub(generic.min(255) as u8);
    if no_mana { cost.is_zero() } else { a.covers_for(&as_paid(cost), &spell_kind(cx, a, card, def)) }
}

/// The most of `generic` generic mana of the spell `def` (the card `card`) any player in the game other than `caster` could
/// pay for it with assist (CR 702.132a): with their pool and their mana abilities, as the advice counts them.
fn assist_could_pay(cx: &Cx, caster: PlayerId, card: ObjectId, def: &CardDef, generic: u8) -> u8 {
    let mut most = 0;
    for p in cx.s().turn_order_from(caster).filter(|&p| p != caster && cx.s().in_game(p)) {
        let a = available(cx, p);
        let kind = spell_kind(cx, &a, card, def);
        if let Some(n) = (most + 1..=generic).rev().find(|&n| a.covers_for(&ManaCost { generic: n, ..ManaCost::ZERO }, &kind)) {
            most = n;
        }
    }
    most
}

/// The text an option is matched by and a menu row shows: an alternative cost's, a granted way's or an ability's. A cast a
/// standing permission lets a card have (CR 601.3) says what the permission changes: "Flashback {2}{R}" for a flashback granted
/// (Snapcaster Mage), "Cast without paying its mana cost", "Cast by paying {1}{U} instead". A cast another object makes
/// possible says which, where its cost does not already (`ActionOption::granted_by`): "Mystic Forge", "Pay {E} rather than its
/// mana cost (Lurrus of the Dream-Den)", so two permissions that would both let a card be cast read apart.
pub fn describe(cards: &CardTable, state: &GameState, log: &[Record], option: &ActionOption) -> std::borrow::Cow<'static, str> {
    let Some(source) = option.source else { return "".into() };
    match option.kind {
        ActionKind::CastSpell => {
            // A card exiled face down is described as the spell it would be (CR 406.3a).
            let _face_up = layers::judged_face_up(state, source);
            let text = describe_cast(cards, state, log, option, source);
            // A fused cast says so whatever it is paid with (CR 702.102a): "Fuse: both halves - Without paying its mana cost".
            match option.index {
                WAY_FUSED if !text.starts_with("Fuse") => format!("Fuse: both halves - {text}").into(),
                _ => text,
            }
        }
        // A land a way a permanent gives lets be played says the way (Serra Paragon's); one a rule or an effect lets be played
        // names it (Bolas's Citadel, Yawgmoth's Will), as a cast does, and so does a permission where another one would let
        // the land be played too.
        ActionKind::PlayLand => {
            let printed = describe_way(cards, state, log, option, source);
            match option.granted_by {
                Some(g) if let Some(way) = named_way(state, cards, g) => way.what.text.into(),
                Some(g) if permission_named(state, source, g) => match printed {
                    "" => permission_name(cards, state, g).into(),
                    printed => format!("{printed} ({})", permission_name(cards, state, g)).into(),
                },
                _ => printed.into(),
            }
        }
        // An ability activated for a cost another permanent lets be paid rather than its mana cost (CR 118.9, 602.2b): the
        // ability's text, the cost and the permanent, so it reads apart from the ability for its own cost.
        ActionKind::ActivateAbility if let Some(g) = option.paid_by => {
            let printed = describe_way(cards, state, log, option, source);
            let reader = crate::resolve::Reader { cards, state, events: log };
            let ability = layers::chars(cards, state, log, source).abilities().nth(option.index as usize);
            let cost = ability.and_then(|a| match cards.ability(a) {
                AbilityDef::Activated(def) => static_rules::named_activation_alternative(&reader, g, asked_player(state), source, def),
                _ => None,
            });
            match cost {
                Some(cost) => format!("{printed} - paying {cost} rather than its mana cost ({})", permission_name(cards, state, g)).into(),
                None => format!("{printed} ({})", permission_name(cards, state, g)).into(),
            }
        }
        // The special action of a companion (CR 116.2g): what it costs and which card it brings in.
        ActionKind::TakeCompanion => format!("{COMPANION_COST}: Put {} in hand.", cards.get(state.obj(source).card).name).into(),
        _ => describe_way(cards, state, log, option, source).into(),
    }
}

/// What made an action possible, as the player knows it: the permanent, the spell, retrace.
fn permission_name(cards: &CardTable, state: &GameState, g: Grant) -> String {
    if g.ability.is_retrace() { "Retrace".to_string() } else { cards.get(state.obj(g.object).card).name.to_string() }
}

/// Whether a menu row names what made it possible: always a rule of a permanent's, a way it gives, retrace or an effect's
/// say-so (Yawgmoth's Will); a play permission an effect gave only where another permission for the card would let it be
/// played or cast too, so the rows read apart.
fn permission_named(state: &GameState, source: ObjectId, g: Grant) -> bool {
    g.ability.as_play_rule().is_some()
        || g.ability.as_granted_way().is_some()
        || g.ability.is_retrace()
        || !state.permissions.iter().any(|p| p.card == source && p.by == g)
        || state.permissions.iter().filter(|p| p.card == source).count() > 1
}

fn describe_cast(cards: &CardTable, state: &GameState, log: &[Record], option: &ActionOption, source: ObjectId) -> std::borrow::Cow<'static, str> {
    let whole = cards.get(state.obj(source).card);
    let face = cast_face(whole, option.index);
    // A face other than the card itself is named by its name, the prototyped face, which has the card's, by its keyword
    // line (CR 718.3): "Prototype {2}{R} — 3/2".
    let face_label = if option.index == WAY_PROTOTYPE { whole.prototype.map_or(face.name, |p| p.text) } else { face.name };
    // A text about a face other than the card itself names the face: "Stomp: Without paying its mana cost".
    let of_face = |text: &str| if std::ptr::eq(face, whole) { text.to_string() } else { format!("{face_label}: {text}") };
    // A way a permanent gives names the face it casts wherever the card could be cast as more than one: "Unholy Annex: Once
    // during each of your turns, ...".
    let of_way = |text: &'static str| -> std::borrow::Cow<'static, str> {
        if std::ptr::eq(face, whole) && whole.inset_faces().next().is_none() && !whole.back.is_some_and(|b| b.has_cost) && whole.prototype.is_none() {
            text.into()
        } else {
            format!("{face_label}: {text}").into()
        }
    };
    let reader = crate::resolve::Reader { cards, state, events: log };
    let player = asked_player(state);
    let through = option.granted_by.and_then(|_| through(&reader, cards, player, source, option.granted_by));
    let permission = |g: Grant| permission_name(cards, state, g);
    // An alternative cost another object gives (CR 118.9): its own words, then the permission it is cast by, if any.
    if let Some(alternative) = option.paid_by.and_then(|g| named_alternative(state, cards, g)) {
        let text = of_face(alternative.what.text);
        return match option.granted_by {
            Some(g) => format!("{text} ({})", permission(g)).into(),
            None => text.into(),
        };
    }
    let printed = describe_way(cards, state, log, option, source);
    let face_cost = option.index == 0 || matches!(option.index, WAY_ADVENTURE | WAY_OMEN | WAY_SPLIT_LEFT | WAY_SPLIT_RIGHT | WAY_BACK_FACE | WAY_PROTOTYPE);
    match (through, option.granted_by) {
        (None, _) | (_, None) => printed.into(),
        // A way a permanent gives (CR 118.9): its words, after the face it casts where the card has more than one.
        (Some(Through::Way(way)), _) => of_way(way.what.text),
        (Some(Through::Route(route)), Some(g)) => {
            // A permission's own alternative cost, for the face cast (CR 118.9): life equal to its mana value, nothing, other
            // mana, or flashback.
            if face_cost && route.life() {
                let life = life_for_mana_value(face).life;
                return match option.index {
                    0 => format!("Cast by paying {life} life instead").into(),
                    _ => format!("{face_label}: cast by paying {life} life instead").into(),
                };
            }
            if option.index == 0 && (route.terms.instead.is_some() || route.terms.flashback) {
                // A flashback granted costs the card's own mana cost unless the grant names another.
                let cost = route.terms.instead.unwrap_or(whole.cost);
                return if route.terms.flashback {
                    format!("Flashback {cost}").into()
                } else if cost == ManaCost::ZERO {
                    "Cast without paying its mana cost".into()
                } else {
                    format!("Cast by paying {cost} instead").into()
                };
            }
            // A permanent's permission to cast cards in exile (CR 601.3) says what it adds: "Cast by removing three counters
            // from among creatures you control"; cast for an alternative cost or as another face, the way says so too.
            if let Allows::Exile(_, cast) = route.allows {
                return if option.index == 0 { cast.text.into() } else { format!("{printed} ({})", lowercase_first(cast.text)).into() };
            }
            // A way a permanent gives that is a permission (Lurrus's): its words for a face's mana cost.
            if let Allows::Way(_, way) = route.allows
                && face_cost
            {
                return of_way(way.text);
            }
            // A play permission an effect gave names its source only where another one would let the card be cast too.
            let named = permission_named(state, source, g);
            match (printed.is_empty(), named) {
                (_, false) => printed.into(),
                (true, true) => permission(g).into(),
                (false, true) => format!("{printed} ({})", permission(g)).into(),
            }
        }
    }
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |c| c.to_lowercase().chain(chars).collect())
}

fn describe_way(cards: &CardTable, state: &GameState, log: &[Record], option: &ActionOption, source: ObjectId) -> &'static str {
    match option.kind {
        ActionKind::CastSpell if matches!(option.index, WAY_ADVENTURE | WAY_OMEN) => {
            cards.get(state.obj(source).card).inset_face(option.index).map_or("", |a| a.name)
        }
        _ if option.index == WAY_BACK_FACE => cards.get(state.obj(source).card).back.map_or("", |b| b.name),
        ActionKind::CastSpell if option.index == WAY_BESTOW => "Bestow",
        ActionKind::CastSpell if option.index == WAY_PROTOTYPE => cards.get(state.obj(source).card).prototype.map_or("", |p| p.text),
        ActionKind::CastSpell if option.index == WAY_SPLIT_LEFT => cards.get(state.obj(source).card).split.map_or("", |h| h.0.name),
        ActionKind::CastSpell if option.index == WAY_SPLIT_RIGHT => cards.get(state.obj(source).card).split.map_or("", |h| h.1.name),
        ActionKind::CastSpell if option.index == WAY_OVERLOAD => "Overload",
        ActionKind::CastSpell if option.index == WAY_FUSED => "Fuse: both halves",
        ActionKind::CastSpell if option.index > 0 => {
            cards.get(state.obj(source).card).ways.get(option.index as usize - 1).map_or("", |w| w.text)
        }
        ActionKind::ActivateAbility if option.index >= PLAYER_MANA_ABILITY => state
            .effects
            .iter()
            .filter(|e| matches!(e.kind, EffectKind::PlayerManaAbility(_)))
            .find(|e| e.source == source)
            .map_or("", |e| match e.kind {
                EffectKind::PlayerManaAbility(a) => cards.ability(a).text(),
                _ => "",
            }),
        ActionKind::ActivateAbility => {
            let chars = layers::chars(cards, state, log, source);
            chars.abilities().nth(option.index as usize).map(|a| cards.ability(a).text()).unwrap_or("")
        }
        _ => "",
    }
}

// --- Taking an action ----------------------------------------------------------------------------------------------

/// The `Act` frame: the action chosen with priority is its first answer.
pub fn act(cx: &mut Cx, player: PlayerId) -> R<Flow> {
    let Answer::Act(action) = cx.ask_core(player, RequestKind::Priority { options: Vec::new() }, false)? else {
        unreachable!("an Act frame starts with its action");
    };
    let source = action.source.expect("an action names its source");
    match action.kind {
        ActionKind::PlayLand => {
            // The land play is counted as the land is played (CR 305.1), before anything that watches it triggers:
            // Fastbond's "if it wasn't the first land you played this turn" counts this one.
            cx.st().update_player(player, |p| p.lands_played = p.lands_played.saturating_add(1));
            // CR 406.3a: a card exiled face down is turned face up just before it is played.
            ops::turn_up_to_play(cx, source);
            // The land is played by what the action names (CR 305.1): a permission that says it enters tapped makes it, and
            // no other permission for the card has any say, as every one ends as it leaves (CR 400.7).
            if !cx.s().permissions.is_empty() {
                cx.st().permissions_retain(|p| p.card != source || Some(p.by) == action.granted_by);
            }
            let way = action.granted_by.and_then(|g| named_way(cx.s(), cx.cards(), g));
            let land = if action.index == WAY_BACK_FACE {
                // A modal double-faced card played as its back face enters as that face (CR 712.14a).
                Some(ops::put_onto_battlefield_as(cx, source, player, false, true, ZoneReason::PlayLand, &[])?)
            } else {
                ops::put_onto_battlefield(cx, &[(source, player, false)], ZoneReason::PlayLand)?.first().copied()
            };
            // What it was played by is recorded with it, as the object it became wherever that is (a replacement may have
            // sent it elsewhere, CR 614.1; it was still played and the permission used): "once during each of your turns"
            // asks (Serra Paragon).
            if let Some(land) = land.filter(|&land| land != source) {
                cx.record(Event::LandPlayed { land, player, way: action.index, granted_by: Grant::of(action.granted_by) });
            }
            if let Some(land) = land.filter(|&land| cx.s().obj(land).zone.is(ZoneKind::Battlefield)) {
                // "If you do, it gains ...": what the way grants is the land's from the moment it is on the battlefield.
                if let Some(way) = way
                    && let Some(n) = way.what.gains
                {
                    let ability = AbilityRef::delayed(way.grant.ability.card, n);
                    Game::add_effect(cx, land, land, EffectKind::AddAbility(ability), Duration::Indefinite);
                }
            }
        }
        ActionKind::CastSpell => {
            // The cast is made by what the option offered it by (CR 601.2b, 601.3): that permission, on its terms - from where
            // it lets the card be cast, for what it names instead of the mana cost or with what it adds - and for the
            // alternative cost the option names, if any (CR 118.9).
            let through = through(&*cx, cx.cards(), player, source, action.granted_by).expect("an offered cast names what it is made by");
            let paid_by = action.paid_by.and_then(|g| named_alternative(cx.s(), cx.cards(), g));
            let alternative = paid_by.map(|a| a.what.cost.cost());
            let flash_alternative = paid_by.filter(|a| a.what.flash).map(|a| a.grant);
            let (route, way) = match through {
                Through::Route(route) => (route, None),
                Through::Way(way) => (Route { by: action.granted_by, ..Route::RULES }, Some(way)),
            };
            // Another object's alternative cost is offered only where the cast pays no other (CR 118.9a).
            debug_assert!(action.paid_by.is_none() || (!route.imposes() && way.is_none()), "an alternative cost beside another");
            let terms = CastTerms {
                instead: if route.life() { Some(ManaCost::ZERO) } else { route.terms.instead },
                any_type: route.terms.any_type,
                more: route.terms.more,
                // A flashback a permission gives (Snapcaster Mage, CR 702.34a).
                flashback: route.terms.flashback,
                // "If that spell would be put into your graveyard, exile it instead" (Jace, Telepath Unbound's).
                exile_instead: route.terms.exile_instead,
                ..CastTerms::default()
            };
            let by = CastBy { by: action.granted_by, way, additional: route.additional, life_for_mana_value: route.life(), alternative, flash_alternative };
            cast(cx, player, source, action.index, &action.targets, terms, by, false)?
        }
        ActionKind::ActivateAbility if action.index >= PLAYER_MANA_ABILITY => {
            activate_mana(cx, player, source, action.index, action.color, false, &[])?;
        }
        ActionKind::ActivateAbility => {
            let ability = cx.chars(source).abilities().nth(action.index as usize).expect("ability");
            let def = activated(cx, ability).expect("an activated ability");
            if def.is_mana_ability() {
                activate_mana(cx, player, source, action.index, action.color, action.mana == Some(Mana::C), &action.targets)?;
            } else {
                // Asked as the ability it is (CR 607.5): one an effect gave the source reads only its own links, from its
                // costs and targets to its effect.
                let by = cx.link_of_nth(source, action.index as usize);
                if def.special_action {
                    special_action(cx, player, source, ability, by)?;
                } else {
                    activate(cx, player, source, ability, action.index, &action.targets, action.granted_by, action.paid_by, by)?;
                }
            }
        }
        ActionKind::TakeCompanion => take_companion(cx, player, source)?,
        _ => unreachable!("not a priority action"),
    }
    // Whoever acted receives priority again, and the run of passes is broken (CR 117.3c).
    cx.st().update_turn(|t| {
        t.priority = player;
        t.passes = 0;
    });
    Ok(Flow::Done)
}

pub(crate) fn stack_item(kind: StackKind, object: ObjectId, ability: AbilityRef, controller: PlayerId, targets: &[Target]) -> StackItem {
    let mut item = StackItem {
        kind,
        object,
        ability,
        controller,
        inline_targets: [Target::None; cantrip_state::INLINE_TARGETS],
        target_count: 0,
        inline_spec_of: [0; cantrip_state::INLINE_TARGETS],
        x: 0,
        kicked: 0,
        counted: 0,
        paid: 0,
        modes: 0,
        way: 0,
        choice: 0,
        subject: Target::None,
        data: [0; 2],
        members: Span::default(),
        serial: 0,
        gift: None,
        shares: StackItem::NO_SHARES,
        spill: 0,
        exile_instead: ExileInstead::Never,
        flashback: false,
        since: 0,
        copied: false,
        cast_known: u32::MAX,
        acquired: Acquired::OWN,
    };
    // Targets announced with the action are one at most (`targets_with_action`); more are set by `set_item_targets`.
    assert!(targets.len() <= cantrip_state::INLINE_TARGETS, "a stack item made with more targets than it holds");
    item.target_count = targets.len() as u32;
    item.inline_targets[..targets.len()].copy_from_slice(targets);
    item
}

/// "The next spell you cast this turn can't be countered" (CR 611.2c, 601.2, 701.5a): each such effect of `player` marks
/// this spell as it is cast, before anyone has priority, and is used up; two before one spell both apply to it.
fn apply_next_spell_effects(cx: &mut Cx, player: PlayerId, spell: ObjectId) {
    let ours = |e: &cantrip_state::ContinuousEffect| e.kind == EffectKind::NextSpellCantBeCountered && e.controller == player;
    if !cx.s().effects.iter().any(ours) {
        return;
    }
    cx.st().update_obj(spell, |o| o.marks.cant_be_countered = true);
    let keep: Vec<bool> = cx.s().effects.iter().map(|e| !ours(e)).collect();
    let effects_before = cx.s().effects.len();
    cx.st().effects_keep(&keep);
    // An effect that ends changes the game with no event (CR 603.8).
    if cx.s().effects.len() < effects_before {
        cx.state_changed();
    }
}

/// What a cast is made by, beyond the terms an effect or a permission sets (`CastTerms`): read once, as the cast begins, from
/// the action's `Grant`s, so the cast keeps them if what gave them leaves while it is paid for (Dawnhand Dissident's ruling)
/// and is made exactly as it was offered.
#[derive(Clone, Copy)]
struct CastBy {
    /// What made the cast possible (`ActionOption::granted_by`), recorded with it (`Cast::granted_by`).
    by: Option<Grant>,
    /// A way a permanent gives that is the cast's cost (CR 118.9: escape, mayhem, Iroh's flashback), and its riders.
    way: Option<Given<GrantedWay>>,
    /// What the permission adds to every cast by it (CR 118.8): a permanent's permission's, retrace's discard.
    additional: Cost,
    /// Life equal to the face's mana value rather than its mana cost (CR 118.9, 119.4): Inside Information, Bolas's Citadel.
    life_for_mana_value: bool,
    /// What an alternative cost another object gives has paid rather than the mana cost (CR 118.9, `ActionOption::paid_by`).
    alternative: Option<Cost>,
    /// That alternative cost, where it lets the spell be cast as though it had flash (Primal Prayers): a permission of its own
    /// to cast it at a time a sorcery couldn't have been cast (CR 307.5).
    flash_alternative: Option<Grant>,
}

impl Default for CastBy {
    fn default() -> CastBy {
        CastBy { by: None, way: None, additional: Cost::NONE, life_for_mana_value: false, alternative: None, flash_alternative: None }
    }
}

/// The permissions other than the card's own "you may cast this spell as though it had flash" that let `player` cast
/// `spell` (`whole`, cast by `way` as itself) now, at a time a sorcery couldn't have been cast (CR 601.3, 307.5): each by the
/// card that gives it - an effect's (`static_rules::flash_type_grants`), a permanent's static ability (Valley Floodcaller),
/// an alternative cost that says so (Primal Prayers) - or none for a rule of the spell's own (an instant, flash, a way of its
/// own cast whenever an instant could be). One entry for each card. A permission for "a spell with certain qualities" is
/// judged on the spell as it is on the stack, under its caster, with the effects that apply to it there (CR 601.2a,
/// 601.3b): an opponent's card cast from exile is an artifact spell under Encroaching Mycosynth's "nonland permanents you
/// control are artifacts in addition to their other types, and the same is true for permanent spells you control".
fn other_flash_permissions(cx: &Cx, player: PlayerId, spell: ObjectId, whole: &'static CardDef, way: u8, alternative: Option<Grant>) -> Vec<Option<CardId>> {
    let mut others: Vec<Option<CardId>> = Vec::new();
    let mut add = |by: Option<CardId>| {
        if !others.contains(&by) {
            others.push(by);
        }
    };
    let own_way_at_instant_speed = way > 0 && whole.ways.get(way as usize - 1).is_some_and(|w| w.instant_speed);
    let chars = cx.chars(spell);
    if chars.is(CardType::Instant) || chars.has(Keyword::Flash) || own_way_at_instant_speed {
        add(None);
    }
    for source in flash_permissions(cx, cx, &static_rules::flash_granters(cx), player, spell) {
        add(Some(cx.cards().current(cx.s().obj(source))));
    }
    if let Some(grant) = alternative {
        add(Some(grant.ability.card));
    }
    others
}

/// The sources of the permissions that let `player` cast `spell` as though it had flash (CR 307.5, 601.3b, 702.8a), read
/// through `q`: an effect's for some card types (`static_rules::flash_type_grants`: Teferi, Time Raveler's "sorcery
/// spells"), and one of the `granters`' static abilities (`static_rules::flash_granters`: Valley Floodcaller's "noncreature
/// spells", Rattlechains's "Spirit spells"). Asked of the spell on the stack, as it is cast (`other_flash_permissions`) or as
/// it would be (`SpellsAsCast`), or of the card where it is where that is the same.
fn flash_permissions(cx: &Cx, q: &dyn Query, granters: &[(ObjectId, PlayerId, &'static CardDef)], player: PlayerId, spell: ObjectId) -> Vec<ObjectId> {
    let types = q.chars(spell).types;
    let mut by: Vec<ObjectId> = static_rules::flash_type_grants(cx, player).filter(|&(_, t)| types.0 & t.0 != 0).map(|(source, _)| source).collect();
    by.extend(granters.iter().filter(|&&granter| static_rules::grants_flash(q, &[granter], player, spell)).map(|&(id, _, _)| id));
    by
}

/// Whether `player` casts `spell` by its card's own "you may cast this spell as though it had flash" rather than by one of
/// `others` that would let it be cast now too (CR 601.3; Necromancy's 2022-12-08 ruling: only a cast by its own permission
/// has it sacrificed at the next cleanup step). Theirs to choose, as either may be what they want.
fn choose_own_flash(cx: &mut Cx, player: PlayerId, spell: ObjectId, others: &[Option<CardId>]) -> R<bool> {
    let modes: Vec<Label> =
        std::iter::once(Label::CastPermission { own: true, by: None }).chain(others.iter().map(|&by| Label::CastPermission { own: false, by })).collect();
    let ask = RequestKind::ChooseModes {
        source: spell,
        eligible: (0..modes.len() as u8).collect(),
        modes,
        count: 1,
        at_most: 0,
        affordable: Vec::new(),
        additional_cost: false,
        repeat: false,
    };
    Ok(match cx.ask_core(player, ask, false)? {
        Answer::Modes(chosen) => chosen.first().is_none_or(|&n| n == 0),
        _ => true,
    })
}

/// Casting a spell (CR 601.2): it moves to the stack, its costs are chosen and paid, and it becomes cast. Cast as an
/// effect resolves (`resolving`, CR 608.2g) its targets are chosen here, not announced with an action, and `instead`
/// is the cost it is cast for rather than its mana cost (CR 118.9). `way` is the card's own option it is cast by (its face,
/// its printed way), and `by` what else it is cast by.
#[allow(clippy::too_many_arguments)]
fn cast(
    cx: &mut Cx,
    player: PlayerId,
    card: ObjectId,
    way: u8,
    targets: &[Target],
    terms: CastTerms,
    by: CastBy,
    resolving: bool,
) -> R<()> {
    let CastTerms { instead, any_type, exile_instead, flashback, life, more, less, copy: _, way: _, caster: _ } = terms;
    // CR 406.3a: a card exiled face down is turned face up just before it is announced.
    ops::turn_up_to_play(cx, card);
    let printed = cx.s().obj(card).card;
    let whole = cx.card(printed);
    // Cast as its Adventure or its Omen (CR 715.3, 720.3), it is that spell and nothing else on the stack (CR 715.3b,
    // 720.3b).
    let inset = whole.inset_face(way);
    // An alternative cost another object gives is paid rather than the mana cost (CR 118.9), its X 0 (CR 107.3b), and every
    // additional cost with it (CR 118.9d).
    let instead = instead.or(by.alternative.map(|a| a.mana));
    let granted = by.way;
    // Cast as its back face (CR 712.8a), a modal double-faced card is that face and nothing else on the stack.
    let back = if way == WAY_BACK_FACE || way == WAY_TRANSFORMED { whole.back } else { None };
    // Cast bestowed (CR 702.103b), it is an Aura spell and nothing else on the stack.
    let bestow = if way == WAY_BESTOW { whole.bestow } else { None };
    // Cast prototyped (CR 718.3b), it has only its alternative mana cost, power and toughness, and their color, on the stack.
    let prototyped = if way == WAY_PROTOTYPE { cantrip_api::prototyped_face(whole) } else { None };
    // Cast as one half of a split card (CR 709.3), it is that half and nothing else on the stack.
    let half = match (way, whole.split) {
        (WAY_SPLIT_LEFT, Some((left, _))) => Some(left),
        (WAY_SPLIT_RIGHT, Some((_, right))) => Some(right),
        _ => None,
    };
    let def = cast_face(whole, way);
    let face = cx.card_id(def);
    // Life equal to the face's mana value rather than its mana cost (CR 118.9, 119.4), of the face cast (CR 715.3a).
    let (instead, life) = if by.life_for_mana_value { (Some(ManaCost::ZERO), u8::try_from(life_for_mana_value(def).life).unwrap_or(u8::MAX)) } else { (instead, life) };
    // Flashback (CR 702.34a) a permission or a permanent gave, or aftermath (CR 702.127a): exiled instead of put anywhere
    // else as it leaves the stack. The way a permanent gave is read now: the spell keeps it if the permanent leaves.
    let flashback = flashback || def.aftermath || granted.is_some_and(|w| w.what.flashback);
    let mut cost = match (way, instead) {
        (_, Some(instead)) => Cost { energy: by.alternative.map_or(0, |a| a.energy), ..Cost::mana(instead) },
        _ if let Some(granted) = granted => {
            // Judged on the face being cast, as the offer was.
            let view = AsFace { cx: &*cx, spell: card, face };
            let q: &dyn Query = if way == 0 { &*cx } else { &view };
            match (granted.what.cost)(q, &granted.asker(), card, player) {
                Some(cost) => cost,
                None => return Err(Stop::Cancel),
            }
        }
        (0, _) => Cost::mana(def.cost),
        // Casting it prototyped is no alternative cost (the ruling): its mana cost is the prototyped face's (CR 718.3b).
        (WAY_ADVENTURE | WAY_OMEN | WAY_BACK_FACE | WAY_SPLIT_LEFT | WAY_SPLIT_RIGHT | WAY_PROTOTYPE, _) => Cost::mana(def.cost),
        // Both halves' mana costs (CR 702.102c), which the whole card's combined cost is (CR 709.4b).
        (WAY_FUSED, _) => Cost::mana(def.cost),
        // Bestow is an alternative cost (CR 702.103a, 118.9): the Aura keeps the card's mana cost.
        (WAY_BESTOW, _) => Cost::mana(whole.bestow_cost),
        (WAY_OVERLOAD, _) => Cost::mana(whole.overload.unwrap_or(def.cost)),
        _ => whole.ways[way as usize - 1].cost,
    };
    // The {X} a granted way's cost has: the card's, where the cost is or includes its mana cost; none where it is cast
    // without paying its mana cost (CR 107.3b).
    let granted_x = granted.map(|_| cost.mana.x);
    // The {X} of the cost this way pays: the face's mana cost, or the card's own alternative cost (CR 118.9: flashback,
    // overload, bestow, a way of its own), X included (CR 107.3a).
    let way_x = cost.mana.x;
    cost.life += u32::from(life);
    cost.mana.generic = cost.mana.generic.saturating_add(more);

    // CR 601.2a: the card moves to the stack first. Its permanent remains prepared until the copy becomes cast
    // (CR 722.3c, 601.2i), including while its costs are paid.
    let prepared_from = cx.s().obj(card).prepared_from;
    let from = cx.s().obj(card).zone.kind();
    let cast_from = from;
    // "If you cast it any time a sorcery couldn't have been cast" (`CardDef::untimely_cleanup_rider`): whether a sorcery could
    // have been cast now (CR 307.1), read before the spell goes on the stack, and whether the card's own "you may cast this
    // spell as though it had flash" is what lets it be cast (Necromancy's 2022-12-08 ruling). A spell an effect casts as it
    // resolves is cast by that effect's permission, never by its own. Where another permission would let it be cast now as
    // well, its caster chooses which one they cast it by (CR 601.3), asked once it is on the stack (below), where the other
    // permissions are judged on the spell it is there (CR 601.2a, 601.3b).
    let untimely = !resolving
        && whole.untimely_cleanup_rider
        && whole.cast_as_though_flash
        && std::ptr::eq(def, whole)
        && !(cx.s().turn.active == player && matches!(cx.s().turn.step(), Step::PrecombatMain | Step::PostcombatMain) && cx.s().stack.is_empty());
    // Cast by morph or disguise (CR 702.37c, 702.168b), it is turned face down before it is put on the stack (CR 708.4): the
    // face-down values it is cast as (`cast_face`) are its copiable values (CR 708.2), and the spell is face down as it arrives
    // (`ops::move_together`), its controller alone able to look at it (CR 708.5).
    if def.face_down {
        cx.st().update_obj(card, |o| o.copy = Some(Copiable::of(face)));
    }
    let spell = ops::move_to_for(cx, card, Zone::STACK, ZoneReason::Cast);
    cx.st().update_obj(spell, |o| {
        o.controller = player;
        o.cast_from = Some(from);
    });
    if untimely {
        let other_permissions = other_flash_permissions(cx, player, spell, whole, way, by.flash_alternative);
        let by_own = other_permissions.is_empty() || choose_own_flash(cx, player, spell, &other_permissions)?;
        cx.st().update_obj(spell, |o| o.marks.untimely_by_own_flash = by_own);
    }
    // "If you do, it gains ...": the spell carries it to the permanent it becomes, from the way a permanent gives it was cast
    // by (Serra Paragon's, its permission or its cost).
    if let Some(g) = by.by
        && let Some(n) = named_way(cx.s(), cx.cards(), g).and_then(|w| w.what.gains)
    {
        let ability = AbilityRef::delayed(g.ability.card, n);
        cx.st().update_obj(spell, |o| o.marks.gains = Some(ability));
    }
    // A permission is used up by the cast (CR 400.7: the card is a new object).
    if !cx.s().permissions.is_empty() {
        cx.st().permissions_retain(|p| p.card != card);
    }
    if inset.is_some() || back.is_some() || bestow.is_some() || half.is_some() || prototyped.is_some() {
        cx.st().update_obj(spell, |o| {
            o.copy = Some(Copiable::of(face))
        });
    }
    let mut item = stack_item(StackKind::Spell, spell, AbilityRef { card: face, index: u8::MAX }, player, targets);
    item.way = way;
    item.exile_instead = exile_instead;
    item.flashback = flashback;
    // Overload (CR 702.96a): "each" for "target", so nothing is targeted.
    let specs: &[TargetSpec] = if way == WAY_OVERLOAD { &[] } else { cx.cards().spell_targets_of(def) };
    // CR 118.8: an additional cost is owed with every way of casting it; its mana joins the total.
    // What the permission adds (CR 118.8): a permanent's permission's, retrace's discard (CR 702.81a).
    let mut extra = def.additional_cost.unwrap_or(Cost::NONE).and(by.additional);
    cost.mana = add_cost(cost.mana, extra.mana);

    // CR 601.2b: a modal spell's modes are chosen first (CR 700.2), each adding its cost (spree, escalate) - unless how
    // many it chooses is defined by whether its optional additional cost is paid or it is kicked, which is announced in
    // the same step: then that is announced first and the modes after it (below).
    let mut mask = 0u32;
    let modes_after_costs = def.modal.is_some_and(|modal| modal.when_paid.is_some());
    if let Some(modal) = def.modal
        && !modes_after_costs
    {
        mask = cast_modes(cx, player, spell, &mut item, modal, (modal.min, modal.max), &mut cost, &mut extra)?;
    }

    // CR 601.2b: an additional cost that is one of several is chosen now, before X (as the C# engine asks it).
    if !def.additional_choice.is_empty() {
        let pick = choose_additional(cx, player, spell, face, def)?;
        // With what is owed already (a permission's additional cost, CR 118.8), not instead of it.
        extra = extra.and(pick);
        cost.mana = add_cost(cost.mana, pick.mana);
    }

    // CR 601.2b: an optional cost that is not mana (teamwork) is announced like a kicker: paid or not, before any target.
    if let Some(optional) = def.optional_cost
        && announce_optional(cx, player, spell, face, def, &optional)?
    {
        extra = extra.and(optional.cost);
        cost.mana = add_cost(cost.mana, optional.cost.mana);
        item.paid = 1;
        cx.st().update_obj(spell, |o| o.paid = 1);
    }

    // CR 702.180b, 601.2b: the creature harmonize taps is chosen as the harmonize cost is, before any target. What is announced
    // after it could still change the generic mana its power comes off (CR 601.2b-f): X, a kicker, the modes a payment
    // decides, splicing onto Arcane, a cost its targets define, and the targets themselves where a cost change could read them.
    let later = cost.mana.x > 0
        || def.kicker.is_some()
        || modes_after_costs
        || def.cost_of_targets.is_some()
        || cx.chars(spell).has_subtype(Subtype::Arcane);
    let targeted = !specs.is_empty() && (!cost_change_sources(cx).is_empty() || !def.own_cost_changes.is_empty() || lasting_changes(cx));
    let reducer = choose_reducer(cx, player, spell, &cost, later, targeted)?;

    // CR 601.2b: X, and how many times it is kicked, are announced, and the total cost follows (CR 601.2f).
    // CR 107.3a, 601.2b: X is announced for a cost that includes it. A way that casts the face itself (an Adventure,
    // CR 715.3b; an Omen, CR 720.3b; a back face, CR 712.8a; one half of a split card, CR 709.3) pays that face's own mana
    // cost, X included; a way of the card's own that is an alternative cost (CR 118.9) pays that cost, its {X} announced as
    // any cost's is (CR 107.3a), and one with no {X} has X 0 (CR 107.3b).
    // A way a permanent grants (Lurrus, Underworld Breach's escape, mayhem) casts the face for its mana cost, or a cost that
    // includes it, so X is announced (CR 107.3a); one that casts it without paying its mana cost leaves X 0
    // (CR 107.3b). Cast without paying its mana cost (`instead`, CR 118.9), X is 0 (CR 107.3b).
    // A cost paid instead that includes {X} (a miracle cost that is the card's mana cost reduced) announces it (CR 107.3a).
    let x_symbols = match (instead, granted_x) {
        (Some(instead), _) => instead.x,
        (None, Some(x)) => x,
        (None, None) => way_x,
    };
    let sacrifices_x = matches!(extra.variable, Some(VariableCost::SacrificeX(_) | VariableCost::SacrificeAnyNumber { .. } | VariableCost::SacrificeWithPowerX(_)));
    // "This spell costs {1} less to cast for each permanent sacrificed this way": generic mana off for each one announced.
    let less_each = match extra.variable {
        Some(VariableCost::SacrificeAnyNumber { less_each, .. }) => less_each,
        _ => 0,
    };
    // How many permanents "sacrifice any number of" announces (Rottenmouth Viper): no X (its mana cost has none, CR
    // 107.3), up to every permanent that could be sacrificed (CR 601.2b), kept here for the reduction and the sacrifice.
    let mut sacrificed_any_number: u32 = 0;
    // "You may exile any number of [white] cards from your hand. This spell costs {2} less to cast for each card exiled this
    // way" (the Marches): the cards are chosen as the intention to pay is announced (CR 601.2b), before X, which is announced
    // from the value their reduction pays for; the reduction comes off the total cost once it is determined (CR 601.2f), and
    // the cards are exiled as it is paid (CR 601.2h). `hand_could` is which cards could have paid as they were chosen.
    let (hand_could, hand_chosen, hand_reduction) = match extra.exile_from_hand_for_less {
        Some(for_less) => {
            let could = hand_exiles_for_less(cx, spell, player, for_less, None);
            let chosen = announce_hand_exiles(cx, player, spell, &could)?;
            let reduction = count_u32(chosen.len()).saturating_mul(u32::from(for_less.less_each));
            (could, chosen, reduction)
        }
        None => (Vec::new(), Vec::new(), 0),
    };
    // "X can't be 0" (CR 107.3a): cast where X can only be 0 (CR 107.3b: without paying its mana cost, or for an
    // alternative cost with no {X}), there is no X it may be cast with, and it can't be cast (Mind Grind's ruling).
    if x_only_zero(def.x_cant_be_zero, def.cost.x, x_symbols) {
        return Err(Stop::Cancel);
    }
    if x_symbols > 0 || def.kicker.is_some() || extra.pay_x_life || sacrifices_x {
        let base = cost.mana;
        // X paid in life is announced up to the life there is (CR 119.4), and X creatures sacrificed up to the creatures
        // there are (CR 601.2b).
        let cap: u32 = if extra.pay_x_life {
            payable_life_for(cx, player, Some(CostFor::Spell)).clamp(0, 1000) as u32
        } else if matches!(extra.variable, Some(VariableCost::SacrificeWithPowerX(_))) {
            // A power read as the cost is paid (CR 601.2h), after the mana abilities that may change it (CR 601.2g): X up
            // to the most a power can be, the payment refusing one nothing has.
            i16::MAX as u32
        } else if let Some(variable) = extra.variable.filter(|_| sacrifices_x) {
            let there = u32::try_from(variable_candidates(cx, spell, player, variable, Some(CostFor::Spell), None).len()).unwrap_or(u32::MAX);
            // An X ("sacrifice X creatures") is a u16 (`StackItem::x`); the number "any number" announces is not.
            if less_each > 0 { there } else { there.min(u32::from(u16::MAX)) }
        } else if let Some(more) = def.kicker.and_then(|k| k.extra).filter(|_| def.kicker.is_some_and(|k| k.multi)) {
            // A multikicker whose cost is more than mana (Squad--Exile four cards from your graveyard): announced as many
            // times as its whole cost can be paid (CR 601.2b, 118.3), the kicker's mana apart, which the payment judges as it
            // does for any multikicker, and no more than a count of it holds (`Cost::most_times`).
            let paid_with = extra;
            (1..=more.most_times()).take_while(|&n| can_pay_other(cx, spell, player, &paid_with.and(more.times(n as u8)))).count() as u32
        } else {
            1000
        };
        // An X that can't be 0 starts at 1 (CR 107.3a). A kicker with {X} announces 0 as not kicking it, so a kicked one
        // has X 1 or more as it is.
        let least = if def.kicker.is_none() && less_each == 0 { u32::from(least_x(def.x_cant_be_zero)) } else { 0 };
        let announced = announce_x(cx, player, spell, base, def.kicker, x_symbols, least, cap, false, less_each, hand_reduction, extra.variable.map_or("", |v| v.governs()))?;
        // Every announcement but "any number"'s is within an X's u16 (`cap`).
        let n = announced.min(u32::from(u16::MAX)) as u16;
        if let Some(k) = def.kicker {
            if k.cost.x > 0 {
                // Kicker {X} (CR 702.33a, 107.3): X is the number announced, and kicked when it is not 0.
                cost.mana = kicker_total(cost.mana, k, n);
                item.kicked = (n > 0) as u8;
                item.x = n;
                cx.st().update_obj(spell, |o| {
                    o.kicked = (n > 0) as u8;
                    o.x = n;
                });
            } else {
                cost.mana = kicker_total(cost.mana, k, n);
                // What the kicker's cost is beside mana, once for each time it is kicked (Squad--Exile four cards, CR 702.157a),
                // chosen and paid with the rest of the additional costs.
                if let Some(more) = k.extra {
                    extra = extra.and(more.times(n as u8));
                }
                item.kicked = n as u8;
                cx.st().update_obj(spell, |o| o.kicked = n as u8);
            }
        } else if less_each > 0 {
            // "Sacrifice any number of permanents" (Rottenmouth Viper) announces how many, which is no X: the spell's mana
            // cost has no {X} and nothing reads it as the value of X (CR 107.3, 601.2b).
            sacrificed_any_number = announced;
        } else {
            // What a cost paid instead was reduced by that its own generic mana could not take, off X's (CR 118.7a).
            let xs = (n as u32 * x_symbols as u32).min(255);
            cost.mana.generic = cost.mana.generic.saturating_add((xs - xs.min(less as u32)) as u8);
            item.x = n;
            cx.st().update_obj(spell, |o| o.x = n);
        }
    }
    // CR 601.2b, 700.2a: the modes of a spell that chooses "both instead" when its additional cost was paid (or "any
    // number instead" when it was kicked), now that the intention to pay is announced.
    if let Some(modal) = def.modal
        && let Some(paid_range) = modal.when_paid
    {
        let range = if item.paid > 0 || item.kicked > 0 { paid_range } else { (modal.min, modal.max) };
        mask = cast_modes(cx, player, spell, &mut item, modal, range, &mut cost, &mut extra)?;
    }
    // CR 702.47a-b: as an Arcane spell is cast, cards with splice onto Arcane may be revealed from the hand and their
    // cost paid; their spell abilities are added to this spell's, after its own, in the order chosen.
    if cx.chars(spell).has_subtype(Subtype::Arcane) {
        let hand = cx.s().zone(Zone::hand(player)).to_vec();
        let eligible: Vec<ObjectId> = hand
            .iter()
            .copied()
            .filter(|&c| cx.card(cx.s().obj(c).card).splice.is_some())
            .collect();
        if !eligible.is_empty() {
            // Any number of different cards, each once (CR 702.47a-b).
            let count = eligible.len() as u32;
            let ask = RequestKind::choose_cards(spell, Purpose::Splice, hand, eligible.clone(), count, true, "");
            let spliced = match cx.ask_core(player, ask, true)? {
                Answer::Cards(cards) => cards,
                _ => return Err(Stop::Cancel),
            };
            if !spliced.is_empty() {
                // Revealed from its announcement until the spell leaves the stack (CR 701.20a): the first objects its
                // announcement chose (`add_revealed_to_item`), in the order spliced, and how many (`resolve::spliced`).
                Game::reveal(cx, player, &spliced);
                item.members = cx.st().members_push(&spliced);
                item.data[0] = spliced.len() as u32;
            }
            for &c in &spliced {
                let splice_card = cx.s().obj(c).card;
                cost.mana = add_cost(cost.mana, cx.card(splice_card).splice.expect("a splice card"));
            }
        }
    }
    let x_life = if extra.pay_x_life { item.x } else { 0 };

    // CR 601.2b, 107.4f: which Phyrexian symbols are paid with life is announced before any target.
    announce_twobrids(cx, player, spell, &mut cost.mana, any_type)?;
    let life = announce_life(cx, player, spell, &mut cost.mana, any_type, Some(CostFor::Spell))?;
    // CR 601.2b, 702.174a: gift is an optional additional cost, announced, with the opponent it is promised to, before
    // the targets (CR 702.174m): what may be targeted can depend on it. The opponent is the only one without asking.
    if let Some(gift) = def.gift
        && cx.s().opponents(player).any(|p| !cx.s().player(p).lost)
        && matches!(cx.ask_core(player, RequestKind::May { source: spell, purpose: Purpose::PromiseGift(gift.kind) }, true)?, Answer::Yes)
    {
        let opponents: Vec<PlayerId> = cx.s().opponents(player).collect();
        let to = if opponents.len() == 1 {
            opponents[0]
        } else {
            let ask = RequestKind::ChoosePlayer { source: spell, candidates: opponents, verb: "Gift", amount: 0, subject: None };
            match cx.ask_core(player, ask, true)? {
                Answer::Player(p) => p,
                _ => return Err(Stop::Cancel),
            }
        };
        item.gift = Some(to);
        cx.st().update_obj(spell, |o| o.gift = Some(to));
    }
    // CR 601.2c: targets slot by slot, where they were not announced with the action; a slot counted by X or by the
    // times it was kicked is judged by what was announced.
    item.counted = if item.x > 0 { item.x } else { u16::from(item.kicked) };
    let counted = u32::from(item.counted);
    item.since = cx.g.log.len() as u32;
    cx.st().stack_push(item);

    let mut targets: Vec<Target> = targets.to_vec();
    let mut of: Vec<u8> = Vec::new();
    if let Some(modal) = def.modal {
        (targets, of) = choose_slots(cx, player, spell, true, &mode_specs(modal, mask), counted, item.kicked, announced_x(&item), None)?;
        set_item_targets(cx, &targets, &of);
    } else if let Some(divide) = def.divide
        && let Some(spec) = specs.first()
    {
        // CR 601.2d: the targets and their shares of a total, in one question; the total may be X.
        let total = match divide.of_x {
            Some(of_x) => u32::from(of_x(item.x)),
            None if divide.total == TargetSpec::COUNTED => u32::from(item.x),
            None => divide.total,
        };
        match ask_divided(cx, player, spell, true, spec, total, announced_x(&item), true, None)? {
            Divided::Chosen(chosen, shares) => {
                targets = chosen;
                set_item_targets(cx, &targets, &[]);
                set_item_shares(cx, &shares);
            }
            Divided::Skipped => {}
            // A total that needs a target and has none to give it is not a legal announcement.
            Divided::Impossible => return Err(Stop::Cancel),
        }
    } else if (resolving || !targets_with_cast(cx.cards(), def)) && !specs.is_empty() {
        (targets, of) = choose_slots(cx, player, spell, true, specs, counted, item.kicked, announced_x(&item), None)?;
        set_item_targets(cx, &targets, &of);
    }

    // CR 702.47c, 601.2c: the targets of spliced text are chosen after the spell's own, in the order the cards were spliced.
    let spliced = crate::resolve::spliced(cx.s(), &item);
    if !spliced.is_empty() {
        let mut all = targets.clone();
        // Targets announced with the cast are the lone slot's, the first.
        of.resize(targets.len(), 0);
        // A spliced card's slots are numbered on past the spell's own specs, in splice order (`resolve::slot_spec`), each
        // target recording its slot, so the spell's own slots need hold no set number of targets.
        let mut next = specs.len() as u8;
        for spliced in spliced {
            let specs = cx.card(spliced).spell.map_or(&[][..], |s| s.targets);
            if !specs.is_empty() {
                let (chosen, chosen_of) = choose_slots(cx, player, spell, true, specs, counted, item.kicked, announced_x(&item), None)?;
                of.extend(chosen_of.iter().map(|&s| next + s));
                all.extend(chosen);
            }
            next += specs.len() as u8;
        }
        if all.len() != targets.len() {
            targets = all;
            set_item_targets(cx, &targets, &of);
        }
    }

    // CR 601.6: the proposal (CR 601.2a-d) is complete; a spell with qualities a static ability forbids now (its X announced,
    // CR 202.3e) may not be cast, and the cast is taken back.
    let forbidders = static_rules::spell_forbidders(cx);
    if !forbidders.is_empty() && static_rules::spell_forbidden(&*cx, &forbidders, player, spell) {
        return Err(Stop::Cancel);
    }

    // CR 601.2f: the total cost, with what increases or reduces it, once the targets are announced (CR 601.2c): a change may
    // read them ("costs {2} less if it targets ..."). It is locked in here, so nothing that happens to a target later changes
    // what is paid. A change may add life as an additional cost (CR 118.8: "cost an additional 3 life to cast"), part of the
    // total, which a player can pay only with at least that much life, the rest of the life the total asks included
    // (CR 119.4, 118.3, 601.2h: no partial payment): a cast that cannot is taken back.
    // CR 601.2f: an additional cost the targets define ("collect evidence X, where X is the total mana value of the
    // permanents this spell targets") joins the total now that they are announced; one its caster can't pay is no legal
    // announcement (CR 601.2h), and the cast is taken back.
    if let Some(of_targets) = def.cost_of_targets {
        let added = of_targets(cx, &Asker { source: spell, controller: player, by: None, spell: true }, &targets);
        if !can_pay_other(cx, spell, player, &added) {
            return Err(Stop::Cancel);
        }
        extra = extra.and(added);
        cost.mana = add_cost(cost.mana, added.mana);
    }
    announce_hybrids(cx, player, spell, Costed::Spell(spell), &mut cost.mana, &targets, any_type)?;
    let split = announce_reduction(cx, player, spell, Costed::Spell(spell), &cost.mana, &targets)?;
    let (added_life, least);
    (cost.mana, added_life, least) = apply_cost_changes_before_least(cx, player, Costed::Spell(spell), cost.mana, &targets, &split);
    if added_life > 0 {
        cost.life += added_life;
        let owed = i64::from(cost.life) + i64::from(extra.life) + i64::from(life) + i64::from(x_life);
        if i64::from(payable_life_for(cx, player, Some(CostFor::Spell))) < owed {
            return Err(Stop::Cancel);
        }
    }
    // CR 702.180a, 601.2f: the total cost comes down by generic mana equal to the tapped creature's power, read as the total
    // cost is determined, after what raises it; colored mana never comes down.
    if let Some(creature) = reducer {
        cost.mana.generic = cost.mana.generic.saturating_sub(reduction_of(cx, creature));
    }
    // "This spell costs {1} less to cast for each permanent sacrificed this way" (CR 601.2f, 118.7a): a reduction by the
    // number announced, after what raises the cost.
    if less_each > 0 {
        cost.mana.generic = cost.mana.generic.saturating_sub(sacrificed_any_number.saturating_mul(u32::from(less_each)).min(255) as u8);
    }
    // "This spell costs {2} less to cast for each card exiled this way" (CR 601.2f, 118.7a): by the cards chosen, the same way.
    cost.mana.generic = cost.mana.generic.saturating_sub(hand_reduction.min(255) as u8);
    // How many a variable sacrifice takes: X announced for "sacrifice X" (Immoral Bargain), or the number announced for
    // "sacrifice any number" (Rottenmouth Viper), kept apart from X.
    let variable_n = |c: &Cost, item: &StackItem| match c.variable {
        Some(VariableCost::SacrificeAnyNumber { .. }) => sacrificed_any_number,
        _ => u32::from(item.x),
    };

    // CR 601.2f-h: permanents the cost names are chosen, the mana paid, then the rest of the cost; an additional
    // cost's choices come once the mana is paid (CR 601.2h), as the C# engine asks them, and so do tapped permanents
    // (web-slinging's), which a mana ability activated while paying may have tapped (CR 601.2g).
    let late = permanents_after_mana(&cost);
    let mut chosen = if late { Vec::new() } else { choose_permanents(cx, player, spell, &cost, Some(CostFor::Spell), None)? };
    let discards = choose_discards(cx, player, spell, &cost, None)?;
    let mut exiles = choose_exiles(cx, player, spell, &cost)?;
    let (way_sacrifices, way_exiles, way_discards) = choose_variable(cx, player, spell, &cost, variable_n(&cost, &item), Some(CostFor::Spell))?;
    exiles.extend(way_exiles);
    let hand_exiles = choose_hand_exiles(cx, player, spell, &cost, None)?;
    // CR 702.119a: emerge's mana comes down by the mana value of what was just chosen to be sacrificed. Nothing has
    // moved yet, so the permanents are still there to be read.
    if cost.emerge {
        let reduction: u32 = chosen.iter().map(|&id| cx.mana_value(id)).sum();
        cost.mana.generic = cost.mana.generic.saturating_sub(reduction.min(255) as u8);
    }
    // CR 601.2f: "once the total cost is determined, any effects that directly affect the total cost are applied" - the least
    // it may come to (Trinisphere), after every increase and reduction, before the total is locked in and paid.
    cost.mana = at_least(cost.mana, least);
    // CR 701.67b: the tapping pays only the generic mana of the waterbend cost, whatever else the total has.
    let waterbend = def.kicker.filter(|k| k.waterbend).map_or(0, |k| k.cost.generic.saturating_mul(item.kicked));
    // CR 702.180b, 601.2h: the creature is tapped as the total cost is paid. It is tapped before the mana is, since a cost
    // may be paid in any order, so it cannot also be tapped for mana: the tap it was chosen for is its own.
    if let Some(creature) = reducer {
        ops::tap(cx, creature);
    }
    let life_owed = cost.life + extra.life + life + u32::from(x_life);
    // CR 601.2f-h: what besides mana may pay for it, its own convoke, delve and improvise and what a permanent gives the spell.
    let with = pay_with(cx, &pay_with_grantors(cx), player, spell, def, None);
    let no_mana = def.keywords.has(Keyword::CantSpendManaToCast);
    // CR 702.132a: with assist, another player the caster chooses may pay generic mana of the total cost before they do.
    let assisted = if with.assist && !no_mana && cost.mana.generic > 0 { assist(cx, player, spell, cost.mana)? } else { None };
    let mut owed = cost.mana;
    if let Some(a) = &assisted {
        owed.generic -= a.generic;
    }
    let helpers = Helpers {
        with,
        waterbend,
        creatures: false,
        no_mana,
        purpose: Paying::Casting,
        reserved: &exiles,
        attackers: &[],
        life: life_owed,
        any_type,
        abilities_closed: assisted.is_some(),
        assist: if assisted.is_some() { AssistStep::CasterPays } else { AssistStep::None },
    };
    let (mut spent, mut restrictions) = pay_mana_used(cx, player, spell, owed, &helpers)?;
    // The mana the assisting player spent was spent to cast it too (CR 601.2h, 106.6).
    if let Some(a) = assisted {
        for m in Mana::ALL {
            spent.0[m as usize] += a.paid.0[m as usize];
        }
        restrictions |= a.restrictions;
    }
    // CR 106.6: a creature spell paid for with mana that gives it haste has haste once it resolves; a spell paid for with
    // mana that makes it uncounterable can't be countered.
    if restrictions & MANA_GIVES_HASTE != 0 && cx.chars(spell).is(CardType::Creature) {
        cx.st().update_obj(spell, |o| o.marks.gains_haste = true);
    }
    if restrictions & MANA_MAKES_UNCOUNTERABLE != 0 {
        cx.st().update_obj(spell, |o| o.marks.cant_be_countered = true);
    }
    apply_next_spell_effects(cx, player, spell);
    if late {
        chosen = choose_permanents(cx, player, spell, &cost, Some(CostFor::Spell), None)?;
    }
    let extra_chosen = choose_permanents(cx, player, spell, &extra, Some(CostFor::Spell), None)?;
    let beheld = choose_behold(cx, player, spell, &cost)?;
    let extra_beheld = choose_behold(cx, player, spell, &extra)?;
    let blighted = choose_blight(cx, player, spell, &cost)?;
    let extra_blighted = choose_blight(cx, player, spell, &extra)?;
    let removed = choose_counters_among(cx, player, spell, &cost)?;
    let extra_removed = choose_counters_among(cx, player, spell, &extra)?;
    let (extra_sacrifices, extra_exiles, extra_variable_discards) = choose_variable(cx, player, spell, &extra, variable_n(&extra, &item), Some(CostFor::Spell))?;
    // An optional cost that sacrifices one or more: how many were is how many times it was paid (Plumb the Forbidden's
    // linked trigger reads it).
    if matches!(extra.variable, Some(VariableCost::SacrificeOneOrMore(_))) {
        let n = count_u32(extra_sacrifices.len());
        let at = cx.s().stack.len() - 1;
        let mut with = cx.s().stack[at];
        with.paid = n;
        cx.st().stack_set(at, with);
        cx.st().update_obj(spell, |o| o.paid = n);
    }
    let extra_discards = choose_discards(cx, player, spell, &extra, None)?;
    let extra_tappers = choose_tappers(cx, player, spell, &extra)?;
    let extra_evidence = choose_exiles(cx, player, spell, &extra)?;
    let hand_exiled = match extra.exile_from_hand_for_less {
        Some(for_less) => confirm_hand_exiles(cx, player, spell, for_less, &hand_chosen, &hand_could)?,
        None => Vec::new(),
    };
    // Sneak (CR 702.190b): the permanent the spell becomes attacks what the creature its cost returns to hand is attacking
    // as the cost is paid, kept on the spell for its resolution. An instant or sorcery becomes no permanent, and keeps none.
    if granted.is_none()
        && instead.is_none()
        && way >= 1
        && whole.ways.get(way as usize - 1).is_some_and(|w| w.enters_attacking)
        && !cx.chars(spell).is(CardType::Instant)
        && !cx.chars(spell).is(CardType::Sorcery)
        && let Some(attack) = chosen.first().and_then(|&c| cx.s().attackers.iter().find(|a| a.id == c)).copied()
    {
        let at = cx.s().stack.len() - 1;
        let mut with = cx.s().stack[at];
        with.subject = attack.target;
        cx.st().stack_set(at, with);
    }
    cx.st().update_obj(spell, |o| {
        o.spent = ManaTally::of(spent);
        o.cast_way = way;
        o.x = item.x;
    });
    // The energy and life are judged again now the mana is paid: a mana ability activated while paying (CR 601.2g) may have
    // spent what they needed, and a cost that can't be paid takes the cast back (CR 601.2h, 119.4, 107.14).
    if !rest_payable(cx, player, cost.energy as u32 + extra.energy as u32, life_owed, Some(CostFor::Spell)) {
        return Err(Stop::Cancel);
    }
    // Counters put on and taken off permanents are paid first (CR 601.2h: in any order), so a permanent the cost also
    // sacrifices leaves with them as the payment left it.
    pay_blight(cx, player, &cost, blighted);
    pay_blight(cx, player, &extra, extra_blighted);
    pay_counters_among(cx, &removed);
    pay_counters_among(cx, &extra_removed);
    pay_permanents(cx, player, &cost, &chosen, None);
    pay_discards(cx, &discards);
    pay_discards(cx, &way_discards);
    pay_exiles(cx, &exiles);
    pay_exiles(cx, &hand_exiles);
    ops::sacrifice(cx, player, &way_sacrifices);
    pay_exiles(cx, &extra_exiles);
    pay_exiles(cx, &extra_evidence);
    pay_exiles(cx, &hand_exiled);
    pay_life_and_counters(cx, player, spell, &cost);
    pay_permanents(cx, player, &extra, &extra_chosen, None);
    let mut revealed = pay_behold(cx, player, &beheld);
    revealed.extend(pay_behold(cx, player, &extra_beheld));
    if let Some(at) = cx.s().stack.iter().rposition(|i| i.kind == StackKind::Spell && i.object == spell) {
        add_revealed_to_item(cx, at, &revealed);
    }
    ops::sacrifice(cx, player, &extra_sacrifices);
    pay_discards(cx, &extra_discards);
    pay_discards(cx, &extra_variable_discards);
    pay_life_and_counters(cx, player, spell, &extra);
    pay_exert_and_tappers(cx, spell, &extra, &extra_tappers);
    // CR 702.194a: the creatures tapped for the teamwork cost became tapped to pay it, which abilities of theirs trigger on.
    if item.paid > 0 && def.optional_cost.is_some_and(|o| o.cost.tap_creatures.is_some()) {
        for &creature in &extra_tappers {
            cx.emit(Event::TappedForTeamwork { creature, spell });
        }
    }
    // CR 119.4: the life paid for Phyrexian symbols and for X, with the rest of the cost.
    if life + u32::from(x_life) > 0 {
        ops::change_life(cx, player, -life_paid(life + u32::from(x_life)));
    }
    let targets = &targets[..];

    // CR 608.3g: what the way cast for leaves behind to happen at the next end step, aimed at the permanent the spell
    // becomes, which `Obj::became` follows.
    for &(rider_way, delayed) in def.end_step_riders {
        if rider_way == way {
            let since = cx.g.log.len() as u32;
            cx.st().delayed_push(cantrip_state::Delayed {
                when: DelayedWhen::Step { step: Step::End, on_turn_of: None },
                source: spell,
                controller: player,
                ability: AbilityRef::delayed(face, delayed),
                subject: Target::Object(spell),
                members: cantrip_state::Span::default(),
                since,
                acquired: Acquired::OWN,
            });
        }
    }

    // CR 601.2i: it becomes cast.
    let as_cast = cx.chars(spell);
    // Its mana value on the stack (`Query::mana_value`): X counted once for each {X} in its mana cost (CR 202.3e), none for an
    // X a cost defined without one (Immoral Bargain's, a number of permanents sacrificed), and a spell cast transformed its front
    // face's (CR 712.8c).
    let value = cx.mana_value(spell);
    // "As you cast this spell": the board once the costs are paid (CR 601.2i), asked of the face being cast.
    let fact = def.cast_fact.map_or(0, |f| f(cx, &Asker { source: spell, controller: player, by: None, spell: true }));
    // The row keeps it, which `Cast::value` reads.
    let known = LastKnown { mana_value: value, ..ops::last_known_from(cx, spell, &as_cast) };
    let known = cx.st().last_known_push(known);
    if let Some(at) = cx.s().stack.iter().rposition(|i| i.kind == StackKind::Spell && i.object == spell) {
        let item = cx.s().stack[at];
        cx.st().stack_set(at, StackItem { cast_known: known, ..item });
    }
    cx.record(Event::Cast(Cast {
        spell,
        player,
        types: as_cast.types,
        from: cast_from,
        way,
        by: Grant::of(by.by),
        fact,
    }));
    // CR 722.3c, 601.2i: as-cast facts read the completed casting first; cast triggers see preparation consumed.
    if let Some(from) = prepared_from {
        ops::become_unprepared(cx, from);
    }
    // "Until this card is cast from exile" (CR 611.2a, 400.7) ends now, as the card that was in exile becomes cast: the
    // effects about it were there while its costs were paid.
    if cast_from == ZoneKind::Exile && cx.s().effects.iter().any(|e| e.duration == Duration::UntilCastFromExile(card)) {
        let effects_before = cx.s().effects.len();
        cx.st().effects_retain(|e| e.duration != Duration::UntilCastFromExile(card));
        // An effect that ends changes the game with no event (CR 603.8).
        if cx.s().effects.len() < effects_before {
            cx.state_changed();
        }
    }
    let nth = count_u32(cx.history().casts_by(player).count());
    cx.emit(Event::SpellCast { spell, player, nth });
    ops::became_targets(cx, targets.iter().copied(), spell, player);
    ops::commit_crime(cx, player, spell, targets);
    Ok(())
}

/// What the player who assisted with a spell's cost paid (CR 702.132a): how much of its generic mana, the mana, and what that
/// mana does to the spell (`ManaRestriction::effects`).
struct Assisted {
    generic: u8,
    paid: ManaPool,
    restrictions: u8,
}

/// Assist (CR 702.132a), before any mana ability is activated while casting `spell`, whose total cost `total` has generic
/// mana: its caster may choose another player in the game; that player has a chance to activate mana abilities, then the
/// caster has theirs; then, before the caster begins to pay, the chosen player may pay any amount of the generic mana, from
/// their own pool (the mana abilities are over for both, CR 601.2g). None where the caster chooses no one. Only the generic
/// mana: colored mana is the caster's to pay whatever the helper's mana is (the ruling), and a cost change already in
/// `total` changes how much that is (the ruling).
fn assist(cx: &mut Cx, caster: PlayerId, spell: ObjectId, total: ManaCost) -> R<Option<Assisted>> {
    let others: Vec<PlayerId> = cx.s().turn_order_from(caster).filter(|&p| p != caster && cx.s().in_game(p)).collect();
    if others.is_empty() {
        return Ok(None);
    }
    if !matches!(cx.ask_core(caster, RequestKind::May { source: spell, purpose: Purpose::Assist }, true)?, Answer::Yes) {
        return Ok(None);
    }
    let helper = if others.len() == 1 {
        others[0]
    } else {
        let ask = RequestKind::ChoosePlayer { source: spell, candidates: others, verb: "Assist", amount: 0, subject: None };
        match cx.ask_core(caster, ask, true)? {
            Answer::Player(p) => p,
            _ => return Err(Stop::Cancel),
        }
    };
    let generic = ManaCost { generic: total.generic, ..ManaCost::ZERO };
    assist_window(cx, helper, spell, generic, AssistStep::HelperActivates)?;
    assist_window(cx, caster, spell, total, AssistStep::CasterActivates)?;
    // Any amount of the generic mana, as much as their pool could pay of it (any mana, restricted mana that may pay for this
    // spell, CR 106.6): announced, then paid from the pool.
    let most = u32::from(total.generic).min(spendable(cx, helper, spell, Paying::Casting).total());
    let mut n: u32 = 0;
    if most > 0 {
        loop {
            let mut options = Vec::new();
            if n < most {
                options.push(ActionOption::simple(ActionKind::IncreaseX));
            }
            if n > 0 {
                options.push(ActionOption::simple(ActionKind::DecreaseX));
            }
            options.push(ActionOption::simple(ActionKind::ConfirmCasting));
            let ask = RequestKind::PayCosts {
                source: spell,
                total: generic,
                remaining: generic,
                options,
                announcing: true,
                x: n,
                most,
                least: 0,
                loyalty: false,
                attackers: Vec::new(),
                life: 0,
                governs: "",
                assist: AssistStep::HelperAnnounces,
            };
            match cx.ask_core(helper, ask, false)? {
                Answer::Announce(value) if value <= most => {
                    n = value;
                    break;
                }
                Answer::Act(a) => match a.kind {
                    ActionKind::IncreaseX if n < most => n += 1,
                    ActionKind::DecreaseX if n > 0 => n -= 1,
                    ActionKind::ConfirmCasting => break,
                    _ => return Err(Stop::Cancel),
                },
                _ => return Err(Stop::Cancel),
            }
        }
    }
    // `most` is at most the generic mana, a u8.
    let generic_paid = n as u8;
    if generic_paid == 0 {
        return Ok(Some(Assisted { generic: 0, paid: ManaPool::default(), restrictions: 0 }));
    }
    let helpers = Helpers { purpose: Paying::Casting, abilities_closed: true, assist: AssistStep::HelperPays, ..Helpers::NONE };
    let (paid, restrictions) = pay_mana_used(cx, helper, spell, ManaCost { generic: generic_paid, ..ManaCost::ZERO }, &helpers)?;
    Ok(Some(Assisted { generic: generic_paid, paid, restrictions }))
}

/// A player's chance to activate mana abilities while a spell with assist is cast (CR 702.132a), ended by `ConfirmCasting`:
/// asked only while they have one to activate. `owed` is what the question shows: the generic mana for the player chosen to
/// help, the whole total for the caster.
fn assist_window(cx: &mut Cx, player: PlayerId, spell: ObjectId, owed: ManaCost, step: AssistStep) -> R<()> {
    loop {
        let mut options = mana_options_while_paying(cx, player, spell, Paying::Casting);
        if options.is_empty() {
            return Ok(());
        }
        options.push(ActionOption::simple(ActionKind::ConfirmCasting));
        let caster = step == AssistStep::CasterActivates;
        if caster {
            options.push(ActionOption::simple(ActionKind::CancelCasting));
        }
        let ask = RequestKind::PayCosts {
            source: spell,
            total: owed,
            remaining: owed,
            options,
            announcing: false,
            x: 0,
            most: 0,
            least: 0,
            loyalty: false,
            attackers: Vec::new(),
            life: 0,
            governs: "",
            assist: step,
        };
        match cx.ask_core(player, ask, caster)? {
            Answer::Act(action) => match action.kind {
                ActionKind::ActivateAbility => {
                    let from = action.source.expect("source");
                    activate_mana(cx, player, from, action.index, action.color, action.mana == Some(Mana::C), &action.targets)?;
                }
                ActionKind::ConfirmCasting => return Ok(()),
                _ => return Err(Stop::Cancel),
            },
            _ => return Err(Stop::Cancel),
        }
    }
}

/// Whether `player` could cast `card` now as the resolving effect has them (CR 608.2g): it is not a land and every target
/// it needs has a legal choice for them. `player` is the effect's controller, or the player it names
/// (`CastTerms::caster`).
pub fn could_cast_during_resolution(cx: &mut Cx, card: ObjectId, player: PlayerId) -> bool {
    !cast_ways_during_resolution(cx, card, player).is_empty()
}

/// Each way `player` (the resolving effect's controller, or the player it names: `CastTerms::caster`) could cast `card` now (CR 608.2g), each face judged alone (CR 709.3a, 715.3a,
/// 712.8a): the card itself unless it is a land or a split card (whose halves are what is cast, CR 709.3), a split card's two
/// halves (an aftermath half only from a graveyard, CR 702.127a), an Adventurer's Adventure, an omen card's Omen (CR 720.3a),
/// and a modal double-faced card's
/// back face. A fused cast (CR 702.102a: a fuse split card cast from its owner's hand, both halves at once, its mana value the
/// sum of theirs, CR 702.102c, 709.4b) is a way too, from a hand only: the ruling has a fuse card cast from a hand without
/// paying its mana cost cast fused.
///
/// A static ability that forbids casting spells with some qualities (CR 101.2, 601.3) and what may be targeted are judged on
/// the spell each would be on the stack (CR 601.3e, `ops::as_cast`).
pub fn cast_ways_during_resolution(cx: &mut Cx, card: ObjectId, player: PlayerId) -> Vec<CastWay> {
    let ways = cast_ways_where_it_is(cx, card, player);
    let forbidders = if ways.is_empty() { Vec::new() } else { static_rules::spell_forbidders(cx) };
    let mut kept = Vec::with_capacity(ways.len());
    for way in ways {
        let face = cast_face_id(cx, card, way.way);
        let allowed = (forbidders.is_empty() || !forbidden_as_cast(cx, &forbidders, card, face, player)) && targets_as_cast(cx, card, way.way, player);
        // The card cast face down by one of its own ways is the face-down spell (CR 708.4), judged as that: a ban the card
        // itself falls under may leave it (Void Winnower's even mana value, a face-down spell's 0, the other way about).
        if allowed || (way.way == 0 && !face_down_ways_allowed(cx, card, player, &forbidders).is_empty()) {
            kept.push(way);
        }
    }
    kept
}

/// The card's own ways that cast it face down (morph, megamorph, disguise: CR 702.37c, 702.168b) that no static ability among
/// `forbidders` forbids `player` to cast it by, judged on the face-down spell it would be (CR 601.3, 708.4, `ops::as_cast`).
fn face_down_ways_allowed(cx: &mut Cx, card: ObjectId, player: PlayerId, forbidders: &[(ObjectId, PlayerId, &'static CardDef)]) -> Vec<u8> {
    let whole = cx.card(cx.s().obj(card).card);
    let mut allowed = Vec::new();
    for n in 1..=whole.ways.len() as u8 {
        if face_down_values_of(whole, n).is_some() && (forbidders.is_empty() || !forbidden_as_cast(cx, forbidders, card, cast_face_id(cx, card, n), player)) {
            allowed.push(n);
        }
    }
    allowed
}

/// Whether `card` cast by `player` its way `way` could announce its modes and targets as the spell it would be on the stack
/// (CR 601.2a-c, `ops::as_cast`), where something reads the spell that would target (`layers::targeting_readers`):
/// judged where the card is, it was judged on everything else.
fn targets_as_cast(cx: &mut Cx, card: ObjectId, way: u8, player: PlayerId) -> bool {
    let whole = cx.card(cx.s().obj(card).card);
    let face = cast_face(whole, way);
    if way == WAY_OVERLOAD || (face.modal.is_none() && cx.cards().spell_targets_of(face).is_empty()) || layers::targeting_readers(cx.cards(), cx.s(), &cx.g.log).is_none() {
        return true;
    }
    let face_id = (!std::ptr::eq(face, whole)).then(|| cx.card_id(face));
    let asker = Asker::spell(card, player);
    ops::as_cast(cx, card, face_id, player, |cx| match face.modal {
        Some(modal) => modes_possible(cx, &asker, modal),
        None => slots_satisfiable(cx, &asker, cx.cards().spell_targets_of(face)),
    })
}

/// `cast_ways_during_resolution` judged where the card is (`layers::can_target`), but for what static abilities forbid.
fn cast_ways_where_it_is(cx: &Cx, card: ObjectId, player: PlayerId) -> Vec<CastWay> {
    let Some(serial) = cx.resolving().map(|i| i.serial) else { return Vec::new() };
    let obj = cx.s().obj(card);
    if !live(cx.s(), card) || matches!(obj.zone.kind(), ZoneKind::Battlefield | ZoneKind::Stack) {
        return Vec::new();
    }
    // Split second (CR 702.61a): no spell is cast, by a resolving effect either, while a spell with it is on the stack under
    // what resolves (Krosan Grip's ruling: "If the resolution of a triggered ability involves casting a spell, that spell can't
    // be cast if a spell with split second is on the stack").
    if cx.s().stack.iter().any(|i| i.serial != serial && i.kind == StackKind::Spell && has_split_second(cx, i.object)) {
        return Vec::new();
    }
    if cast_barred(cx, player).contains(obj.zone.kind()) {
        return Vec::new();
    }
    // A card exiled face down is judged as the spell it would be (CR 406.3a-b).
    let _face_up = layers::judged_face_up(cx.s(), card);
    let def = cx.card(obj.card);
    let asker = Asker { source: card, controller: player, by: None, spell: true };
    let castable = |face: &'static CardDef| {
        cast_only_if_met(cx, face, &asker)
            && match face.modal {
                Some(modal) => modes_possible(cx, &asker, modal),
                None => slots_satisfiable(cx, &asker, cx.cards().spell_targets_of(face)),
            }
    };
    let mut ways = Vec::new();
    if let Some((left, right)) = def.split {
        for (way, half) in [(WAY_SPLIT_LEFT, left), (WAY_SPLIT_RIGHT, right)] {
            if (!half.aftermath || obj.zone.is(ZoneKind::Graveyard)) && castable(half) {
                ways.push(CastWay { way, face: cx.card_id(half), mana_value: half.cost.mana_value() });
            }
        }
        if def.fuse && obj.zone == Zone::hand(obj.owner) && obj.owner == player && castable(def) {
            ways.push(CastWay { way: WAY_FUSED, face: obj.card, mana_value: def.cost.mana_value() });
        }
        return ways;
    }
    if !def.is(CardType::Land) && castable(def) {
        ways.push(CastWay { way: 0, face: obj.card, mana_value: cx.mana_value(card) });
    }
    for (way, inset) in def.inset_faces() {
        if castable(inset) {
            ways.push(CastWay { way, face: cx.card_id(inset), mana_value: inset.cost.mana_value() });
        }
    }
    if let Some(back) = def.back
        && back.has_cost
        && castable(back)
    {
        ways.push(CastWay { way: WAY_BACK_FACE, face: cx.card_id(back), mana_value: back.cost.mana_value() });
    }
    ways
}

/// Whether the resolving effect's controller could play `card` as a land now (CR 305.1, 608.2g): it is a land card not
/// on the battlefield or the stack, it is their own turn (CR 305.3) and they have a land play left (CR 305.2). The phase,
/// the stack and priority are not asked: the effect itself is the permission to play it now.
pub fn could_play_land_during_resolution(cx: &Cx, card: ObjectId) -> bool {
    let Some(player) = cx.resolving().map(|i| i.controller) else { return false };
    let obj = cx.s().obj(card);
    if !live(cx.s(), card) || matches!(obj.zone.kind(), ZoneKind::Battlefield | ZoneKind::Stack) || cx.s().turn.active != player {
        return false;
    }
    // One a static ability keeps from entering can't be played (CR 305.1, 101.2), nor one a static ability forbids to be
    // played (`LandPlayRule`): the effect is a permission to play it, and a restriction takes precedence (CR 101.2).
    if !cx.card(obj.card).is(CardType::Land) || cx.s().player(player).lost {
        return false;
    }
    let bans = crate::static_rules::land_play_bans(cx);
    if !bans.is_empty() && (crate::static_rules::enter_banned(cx, &bans, card) || crate::static_rules::land_play_forbidden(cx, &bans, player, card, obj.card)) {
        return false;
    }
    let LandRules { extra, unlimited, .. } = land_rules(cx, player);
    unlimited || cx.s().player(player).lands_played < 1 + extra
}

/// Plays `card` as a land as an effect resolves (CR 305.1, 608.2g): put onto the battlefield as a land play, counted
/// against the turn's land plays as the special action is (CR 305.2a). The new object, or nothing if it could not be.
pub fn play_land_during_resolution(cx: &mut Cx, card: ObjectId) -> R<Option<ObjectId>> {
    if !could_play_land_during_resolution(cx, card) {
        return Ok(None);
    }
    let Some(player) = cx.resolving().map(|i| i.controller) else { return Ok(None) };
    cx.st().update_player(player, |p| p.lands_played = p.lands_played.saturating_add(1));
    ops::turn_up_to_play(cx, card);
    // Played by the effect, not by any permission on the card (CR 305.1, 608.2g): "each land played this way enters
    // tapped" (Lightstall Inquisitor's) has no say, and every permission for the card ends as it leaves (CR 400.7).
    if !cx.s().permissions.is_empty() {
        cx.st().permissions_retain(|p| p.card != card);
    }
    let moved = ops::put_onto_battlefield(cx, &[(card, player, false)], ZoneReason::PlayLand)?;
    let land = moved.first().copied().filter(|&land| land != card);
    if let Some(land) = land {
        cx.record(Event::LandPlayed { land, player, way: 0, granted_by: Grant::NONE });
    }
    Ok(land)
}

/// Casts `card` as an effect resolves (CR 608.2g), by the player the terms name (`CastTerms::caster`) or else the effect's
/// controller: they announce it, make its choices, pay its costs (CR 601.2) and control it (CR 405.4). A cast given up
/// (CR 733.1) is undone, and nothing happens.
pub fn cast_during_resolution(cx: &mut Cx, card: ObjectId, terms: CastTerms) -> R<bool> {
    let Some(player) = terms.caster.or_else(|| cx.resolving().map(|i| i.controller)) else { return Ok(false) };
    let ways = cast_ways_during_resolution(cx, card, player);
    // CR 709.3, 715.3, 712.8a: which face is cast is chosen as it is cast, before it goes on the stack.
    let mut way = match (terms.way, ways.len()) {
        (_, 0) => return Ok(false),
        (Some(way), _) if ways.iter().any(|w| w.way == way) => way,
        (Some(_), _) => return Ok(false),
        (None, 1) => ways[0].way,
        (None, _) => {
            let modes = ways.iter().map(|w| if w.way == WAY_FUSED { Label::Fused(w.face) } else { Label::Face(w.face) }).collect();
            let ask = RequestKind::ChooseModes {
                source: card,
                modes,
                eligible: (0..ways.len() as u8).collect(),
                count: 1,
                at_most: 0,
                affordable: Vec::new(),
                additional_cost: false,
                repeat: false,
            };
            match cx.ask_core(player, ask, false)? {
                Answer::Modes(chosen) => ways[chosen.first().copied().unwrap_or(0) as usize % ways.len()].way,
                _ => ways[0].way,
            }
        }
    };
    // CR 601.2b, 118.9: the cost it is cast for is announced as it is cast, where the effect has it cast paying its costs -
    // neither "without paying its mana cost" nor for another cost, nor with flashback (CR 118.9a): its mana cost, an
    // alternative way of the card's own that is open from where it is (evoke, Force of Will's; flashback only from its
    // owner's graveyard, CR 702.34a), or an alternative cost another object gives (Jodah's, Primal Prayers'). A face with no
    // mana cost is cast only for an alternative cost (CR 118.6a), so its mana cost is no choice then, and without one it is
    // not cast at all.
    let pays_mana_cost = terms.instead.is_none() && terms.life == 0;
    let whole = cx.card(cx.s().obj(card).card);
    let face = cast_face(whole, way);
    let mut alternative: Option<Cost> = None;
    if pays_mana_cost && !terms.flashback {
        // The card itself and the face-down spell its morph or disguise would cast are judged apart (CR 601.3, 708.4): a ban
        // on the one need not be on the other.
        let (whole_forbidden, face_down_allowed) = if way == 0 && whole.ways.iter().any(|w| w.kind.face_down()) {
            let forbidders = static_rules::spell_forbidders(cx);
            let whole_forbidden = (!forbidders.is_empty() && forbidden_as_cast(cx, &forbidders, card, None, player)) || !targets_as_cast(cx, card, 0, player);
            (whole_forbidden, face_down_ways_allowed(cx, card, player, &forbidders))
        } else {
            (false, Vec::new())
        };
        // What the alternative costs read of the spell is the spell it would be on the stack (`SpellsAsCast`).
        let all = if whole_forbidden { Vec::new() } else { spell_alternatives_of(cx.cards(), cx.s(), &cx.g.log) };
        let spell = if all.is_empty() { None } else { chars_as_cast_now(cx, card, cast_face_id(cx, card, way), player) };
        // A card exiled face down is judged as the spell it would be (CR 406.3a-b); the question below is asked of the
        // card, and `cast` turns it face up.
        let (ways, alternatives) = {
            let _face_up = layers::judged_face_up(cx.s(), card);
            let obj = cx.s().obj(card);
            let asker = Asker { source: card, controller: player, by: None, spell: true };
            let ways: Vec<u8> = if way == 0 {
                whole
                    .ways
                    .iter()
                    .enumerate()
                    .filter(|&(n, w)| {
                        let open = (w.from == ZoneKind::Hand && (!w.only_from || obj.zone == Zone::hand(player)))
                            || (w.from == obj.zone.kind() && obj.zone.player == player);
                        let allowed = if w.kind.face_down() { face_down_allowed.contains(&((n + 1) as u8)) } else { !whole_forbidden };
                        open && allowed
                            && w.condition.is_none_or(|c| c(&*cx, &asker))
                            && can_pay_other(cx, card, player, &w.cost)
                            && !x_only_zero(face.x_cant_be_zero, face.cost.x, w.cost.mana.x)
                    })
                    .map(|(n, _)| (n + 1) as u8)
                    .collect()
            } else {
                Vec::new()
            };
            let alternatives: Vec<Given<SpellAlternative>> = alternatives_for(&*cx, cx.cards(), &all, player, card, face, way == 0, spell)
                .into_iter()
                .filter(|a| can_pay_other(cx, card, player, &a.what.cost.cost()))
                // An X that can't be 0 has nothing to be for a cost with no {X} (CR 107.3a-b): not a choice.
                .filter(|a| !x_only_zero(face.x_cant_be_zero, face.cost.x, a.what.cost.cost().mana.x))
                .collect();
            (ways, alternatives)
        };
        // The choices: the mana cost's own, where the face has one, then the card's ways, then each alternative cost.
        let mut choices: Vec<(Label, u8, Option<Given<SpellAlternative>>)> = Vec::new();
        if face.has_cost && !whole_forbidden {
            choices.push((Label::ManaCost, way, None));
        }
        let printed = cx.cards().current(cx.s().obj(card));
        for &n in &ways {
            choices.push((Label::Way(printed, n - 1), n, None));
        }
        for &a in &alternatives {
            let k = a.grant.ability.as_spell_alternative().unwrap_or(0) as u8;
            choices.push((Label::AlternativeCost(a.grant.ability.card, k), way, Some(a)));
        }
        // Its mana cost alone is no choice to ask about; an alternative cost is announced, even the only one (CR 601.2b).
        let chosen = match choices.len() {
            0 => return Ok(false),
            1 if choices[0].2.is_none() && choices[0].1 == way => 0,
            _ => {
                let ask = RequestKind::ChooseModes {
                    source: card,
                    eligible: (0..choices.len() as u8).collect(),
                    modes: choices.iter().map(|c| c.0.clone()).collect(),
                    count: 1,
                    at_most: 0,
                    affordable: Vec::new(),
                    additional_cost: false,
                    repeat: false,
                };
                match cx.ask_core(player, ask, false)? {
                    Answer::Modes(chosen) => chosen.first().map_or(0, |&n| n as usize).min(choices.len() - 1),
                    _ => 0,
                }
            }
        };
        let (_, index, given) = choices[chosen];
        way = index;
        alternative = given.map(|a| a.what.cost.cost());
    } else if pays_mana_cost && !face.has_cost {
        return Ok(false);
    }
    let mark = cx.s().mark();
    let log_mark = cx.g.log.len();
    // CR 707.12: it is a copy of the card that is cast, and the card stays where it is. A cast given up reverts the copy too.
    let card = if terms.copy { ops::copy_card(cx, card, player) } else { card };
    let by = CastBy { alternative, ..CastBy::default() };
    match cast(cx, player, card, way, &[], terms, by, true) {
        Ok(()) => Ok(true),
        Err(Stop::Cancel) => {
            cx.st().revert(mark);
            cx.g.log.truncate(log_mark);
            Ok(false)
        }
        Err(stop) => Err(stop),
    }
}

/// Whether the resolving effect's controller could cast `card` transformed now (CR 608.2g, 712.11a): a double-faced
/// card that is not modal (CR 712.11c-d: its back face has no mana cost of its own), judged as that back face alone.
pub fn could_cast_transformed_during_resolution(cx: &mut Cx, card: ObjectId) -> bool {
    let Some(player) = cx.resolving().map(|i| i.controller) else { return false };
    if !could_cast_transformed_where_it_is(cx, card) {
        return false;
    }
    // A ban on spells like it is judged on the back face as the spell it would be (CR 601.3e, 712.8a, `ops::as_cast`).
    let forbidders = static_rules::spell_forbidders(cx);
    if !forbidders.is_empty() && forbidden_as_cast(cx, &forbidders, card, cast_face_id(cx, card, WAY_TRANSFORMED), player) {
        return false;
    }
    targets_as_cast(cx, card, WAY_TRANSFORMED, player)
}

/// `could_cast_transformed_during_resolution` judged where the card is (`layers::can_target`), but for what static abilities
/// forbid.
fn could_cast_transformed_where_it_is(cx: &Cx, card: ObjectId) -> bool {
    let Some(player) = cx.resolving().map(|i| i.controller) else { return false };
    let obj = cx.s().obj(card);
    if !live(cx.s(), card) || matches!(obj.zone.kind(), ZoneKind::Battlefield | ZoneKind::Stack) {
        return false;
    }
    if cast_barred(cx, player).contains(obj.zone.kind()) {
        return false;
    }
    let Some(back) = cx.card(obj.card).back else { return false };
    if back.has_cost {
        return false;
    }
    let _face_up = layers::judged_face_up(cx.s(), card);
    let asker = Asker { source: card, controller: player, by: None, spell: true };
    if !cast_only_if_met(cx, back, &asker) {
        return false;
    }
    if let Some(modal) = back.modal {
        return modes_possible(cx, &asker, modal);
    }
    slots_satisfiable(cx, &asker, back.spell_targets())
}

/// Casts `card` transformed as an effect resolves, for nothing (CR 608.2g, 712.11a, 118.9). A cast given up (CR 733.1)
/// is undone, and nothing happens.
pub fn cast_transformed_during_resolution(cx: &mut Cx, card: ObjectId) -> R<bool> {
    if !could_cast_transformed_during_resolution(cx, card) {
        return Ok(false);
    }
    let Some(player) = cx.resolving().map(|i| i.controller) else { return Ok(false) };
    let mark = cx.s().mark();
    let log_mark = cx.g.log.len();
    match cast(cx, player, card, WAY_TRANSFORMED, &[], CastTerms { instead: Some(ManaCost::ZERO), ..CastTerms::default() }, CastBy::default(), true) {
        Ok(()) => Ok(true),
        Err(Stop::Cancel) => {
            cx.st().revert(mark);
            cx.g.log.truncate(log_mark);
            Ok(false)
        }
        Err(stop) => Err(stop),
    }
}

/// A slot whose targets together are judged (`TargetSpec::together`; CR 601.2c, 603.3d, 115.1), `counted` its count: the
/// candidates less any the rule refuses by itself, which no legal choice could include, and the rule with its numbers read
/// for the question (the core refuses an answer it does not call complete). A slot without one keeps its candidates and
/// asks nothing of them.
pub(crate) fn within_together(q: &dyn Query, asker: &Asker, spec: &TargetSpec, counted: u32, candidates: Vec<Target>) -> (Vec<Target>, Option<Part>) {
    if spec.together.is_none() {
        return (candidates, None);
    }
    let together = spec.together_at(counted);
    let kept = select::targets_allowed(q, together.as_slice(), asker.controller, asker.source, candidates);
    (kept, together)
}

/// Whether a slot judged by `together` has a legal choice of `least` targets among `candidates` (CR 601.2c): as many
/// candidates, and for a rule of its targets together, that many it allows together.
pub(crate) fn enough_together(q: &dyn Query, asker: &Asker, together: &Option<Part>, candidates: &[Target], least: u32) -> bool {
    candidates.len() >= least as usize
        && (together.is_none() || least == 0 || select::first_targets(q, together.as_ref(), asker.controller, asker.source, candidates, least as usize).is_some())
}

/// Each target slot's targets, asked in slot order (CR 601.2c), each a target of the slot's own (a player may be
/// chosen for two slots). A slot that chose nothing ("up to") keeps its place with `Target::None` among the slots, so
/// every target stays at the index of its slot (CR 601.2c, 115.1), as far as the last one that chose. Each target comes
/// with the index of the spec it was chosen for (`StackItem::spec_of`), so a slot of several targets may stand anywhere.
/// `x` is the X announced for the spell or ability (`resolve::announced_x`), which its filters read.
#[allow(clippy::too_many_arguments)]
fn choose_slots(cx: &mut Cx, player: PlayerId, source: ObjectId, spell: bool, specs: &[TargetSpec], counted: u32, kicked: u8, x: Option<u16>, by: Option<Linked>) -> R<(Vec<Target>, Vec<u8>)> {
    let asker = Asker { source, controller: player, by, spell };
    let slots = crate::resolve::expand_specs(cx.s(), player, specs);
    let mut targets = Vec::new();
    let mut of = Vec::new();
    for (spec, slot_player, at) in &slots {
        // CR 702.33a, 601.2c: "if this spell was kicked, instead choose ...", the kicker announced by now.
        let spec = &spec.when_kicked(kicked);
        let before = targets.len();
        'slot: {
            let candidates =
                target_candidates_given(cx, &asker, spec, if slots.len() > 1 { &targets } else { &[] }, Target::None, *slot_player, x);
            let (candidates, together) = within_together(cx, &asker, spec, counted, candidates);
            let (least, most) = spec.bounds(counted);
            // Fewer legal targets than the slot's minimum is not a legal announcement (CR 601.2c, 602.2b): given up whole.
            if !enough_together(cx, &asker, &together, &candidates, least) {
                return Err(Stop::Cancel);
            }
            if candidates.is_empty() {
                if spec.required() {
                    return Err(Stop::Cancel);
                }
                break 'slot;
            }
            if most == 0 {
                break 'slot;
            }
            // No more than there are candidates to choose (CR 601.2c: each target is a different object or player).
            let count = most.min(cantrip_state::count_u32(candidates.len()));
            let ask = RequestKind::ChooseTarget { label: spec.label, slot_player: *slot_player,
                source,
                purpose: Purpose::Targets,
                candidates,
                optional: least == 0,
                least: u32::from(least).min(count),
                count,
                // "Any number of targets" (CR 115.1d) is said so, not as "up to" the candidates there are.
                any_number: spec.is_any_number(),
                together,
            };
            match cx.ask_core(player, ask, true)? {
                Answer::Targets(chosen) => targets.extend(chosen),
                _ => return Err(Stop::Cancel),
            }
        }
        if slots.len() > 1 && spec.max <= 1 && targets.len() == before {
            targets.push(Target::None);
        }
        of.resize(targets.len(), *at);
    }
    while targets.last() == Some(&Target::None) {
        targets.pop();
    }
    of.truncate(targets.len());
    Ok((targets, of))
}

/// What asking a divided slot came to.
pub(crate) enum Divided {
    /// Nothing to ask or nothing chosen: a total of nothing, or an optional slot with no candidate or no target chosen.
    Skipped,
    /// The targets, and each one's share of the total.
    Chosen(Vec<Target>, Vec<u32>),
    /// A required slot with nothing to aim at (CR 601.2c).
    Impossible,
}

/// Asks for the targets of a slot that divides a total among them, and each target's share (CR 601.2d), in one
/// question, as the C# engine does: a total of nothing is not asked; with no candidate an optional slot takes no
/// target. `may_cancel` is for a spell or ability still being announced; `x` is the X announced for it
/// (`resolve::announced_x`).
pub(crate) fn ask_divided(cx: &mut Cx, player: PlayerId, source: ObjectId, spell: bool, spec: &TargetSpec, total: u32, x: Option<u16>, may_cancel: bool, by: Option<Linked>) -> R<Divided> {
    if total == 0 {
        return Ok(Divided::Skipped);
    }
    let asker = Asker { source, controller: player, by, spell };
    let candidates = target_candidates_given(cx, &asker, spec, &[], Target::None, None, x);
    if candidates.is_empty() {
        return Ok(if spec.min == 0 { Divided::Skipped } else { Divided::Impossible });
    }
    // 0 is no limit but the candidates: a slot counted by X (the total divided is X too) or of any number of targets.
    let max_targets = if spec.max >= TargetSpec::ANY_NUMBER { 0 } else { spec.max };
    let ask = RequestKind::ChooseDividedTargets { source, candidates, total, max_targets, optional: spec.min == 0 };
    match cx.ask_core(player, ask, may_cancel)? {
        Answer::Divided(shares) if shares.is_empty() => Ok(Divided::Skipped),
        Answer::Divided(shares) => Ok(Divided::Chosen(
            shares.iter().map(|s| s.0).collect(),
            shares.iter().map(|s| s.1).collect(),
        )),
        _ => Err(Stop::Cancel),
    }
}

/// Records the shares chosen for the item being announced, the top of the stack.
fn set_item_shares(cx: &mut Cx, shares: &[u32]) {
    let at = cx.s().stack.len() - 1;
    let mut with = cx.s().stack[at];
    cx.st().set_item_shares(&mut with, shares);
    cx.st().stack_set(at, with);
}

fn add_cost(a: ManaCost, b: ManaCost) -> ManaCost {
    let mut c = a;
    c.generic += b.generic;
    for i in 0..6 {
        c.symbols[i] += b.symbols[i];
    }
    for i in 0..5 {
        c.twobrid[i] += b.twobrid[i];
    }
    c
}

/// Records the targets chosen for the item being announced, the top of the stack, each with the spec it was chosen for
/// (`StackItem::spec_of`; none given is the first, a lone slot's).
fn set_item_targets(cx: &mut Cx, targets: &[Target], of: &[u8]) {
    let at = cx.s().stack.len() - 1;
    let mut with = cx.s().stack[at];
    let shares = cx.s().shares(&with).to_vec();
    cx.st().set_item_targets(&mut with, targets, of, &shares);
    cx.st().stack_set(at, with);
}

/// The target slots of the chosen modes, in printed order (CR 700.2c).
pub(crate) fn mode_specs(modal: &ModalDef, mask: u32) -> Vec<TargetSpec> {
    let mut specs = Vec::new();
    for (i, mode) in modal.modes.iter().enumerate() {
        for _ in 0..modal.times(mask, i) {
            specs.extend_from_slice(mode.targets);
        }
    }
    specs
}

/// The cost of choosing these modes on top of `base` (CR 601.2f): each mode's own cost (spree, CR 702.172a), and the
/// escalate cost once for each mode beyond the first (CR 702.120a).
fn modal_cost(modal: &ModalDef, mask: u32, base: ManaCost) -> ManaCost {
    let mut cost = base;
    for (i, mode) in modal.modes.iter().enumerate() {
        for _ in 0..modal.times(mask, i) {
            cost = add_cost(cost, mode.cost);
        }
    }
    add_cost(cost, escalate_cost(modal, mask).mana)
}

/// The escalate cost of a choice of modes (CR 702.120a): paid once for each mode beyond the first.
fn escalate_cost(modal: &ModalDef, mask: u32) -> Cost {
    match modal.escalate {
        Some(escalate) => escalate.times(modal.chosen(mask).saturating_sub(1)),
        None => Cost::NONE,
    }
}

/// A modal spell's modes as it is cast (CR 601.2b, 700.2): `range` of them chosen, each adding its cost (spree, escalate)
/// to the total. Returns the chosen modes, which the stack item now holds.
#[allow(clippy::too_many_arguments)]
fn cast_modes(
    cx: &mut Cx,
    player: PlayerId,
    spell: ObjectId,
    item: &mut StackItem,
    modal: &'static ModalDef,
    range: (u8, u8),
    cost: &mut Cost,
    extra: &mut Cost,
) -> R<u32> {
    let mask = choose_modes(cx, player, spell, true, item.ability, modal, range, cost.mana, None)?;
    cost.mana = modal_cost(modal, mask, cost.mana);
    item.modes = mask;
    // CR 702.120a: the rest of the escalate cost (a discard) is an additional cost, chosen and paid with the others.
    let mut escalate = escalate_cost(modal, mask);
    escalate.mana = ManaCost::ZERO;
    *extra = extra.and(escalate);
    Ok(mask)
}

/// Asks for a modal spell's or ability's modes (CR 601.2b, 602.2b, 700.2), as the C# engine's `ChooseModesPrompt`:
/// `range` (at least, at most) of the eligible modes, with the ones whose own cost could be paid beside `base` as advice.
/// Returns the chosen modes as a bit for each.
fn choose_modes(cx: &mut Cx, player: PlayerId, source: ObjectId, spell: bool, ability: AbilityRef, modal: &ModalDef, range: (u8, u8), base: ManaCost, by: Option<Linked>) -> R<u32> {
    let (least, max) = range;
    let asker = Asker { source, controller: player, by, spell };
    let eligible = eligible_modes(cx, &asker, modal);
    let costed = modal.modes.iter().any(|m| m.cost.mana_value() > 0);
    let affordable: Vec<u8> = if costed {
        eligible.iter().copied().filter(|&i| can_afford(cx, player, source, &add_cost(base, modal.modes[i as usize].cost))).collect()
    } else {
        Vec::new()
    };
    // An escalate cost the player cannot pay narrows the range to one they can.
    let mut most = max;
    // CR 700.2: the board as the spell is cast may allow more modes than it prints.
    if let Some(up_to) = cx.card(cx.s().obj(source).card).modes_up_to {
        most = most.max(up_to(cx, &asker));
    }
    if modal.escalate.is_some() {
        let payable = |cx: &Cx, n: u8| {
            let first_n = (0..usize::from(n)).fold(0u32, |mask, i| modal.with(mask, i));
            can_afford(cx, player, source, &modal_cost(modal, first_n, base)) && can_pay_other(cx, source, player, &escalate_cost(modal, first_n))
        };
        while most > least.max(1) && !payable(cx, most) {
            most -= 1;
        }
    }
    let ask = RequestKind::ChooseModes {
        source,
        modes: (0..modal.modes.len() as u8).map(|i| Label::Mode(ability, i)).collect(),
        eligible,
        count: least,
        at_most: if most > least { most } else { 0 },
        affordable,
        additional_cost: false,
        repeat: modal.repeat,
    };
    match cx.ask_core(player, ask, true)? {
        Answer::Modes(chosen) => {
            let modes = chosen.iter().fold(0u32, |mask, &i| modal.with(mask, i as usize));
            cx.record(Event::ModesChosen { source, ability, modes });
            Ok(modes)
        }
        _ => Err(Stop::Cancel),
    }
}

/// Announces the nonhybrid equivalent of a cost's monocolored hybrid symbols (CR 601.2b, 118.13a): which {2/W} are paid
/// with {2} rather than {W}, any number of them, before the cost is totalled, so a generic reduction then reduces what was
/// announced as {2} (CR 601.2f, 118.7a). They become that generic mana or their color's symbols in `cost`. The advice is
/// the fewest paid with {2} that leave a cost the player could pay; `any_type`: mana of any type may pay it (CR 609.4b).
fn announce_twobrids(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &mut ManaCost, any_type: bool) -> R<()> {
    let mut symbols: Vec<Color> = Vec::new();
    for (i, &n) in cost.twobrid.iter().enumerate() {
        for _ in 0..n {
            symbols.push(Color::ALL[i]);
        }
    }
    if symbols.is_empty() {
        return Ok(());
    }
    let n = symbols.len();
    let affordable: Vec<u8> = if n <= 8 {
        let mut masks: Vec<u32> = (0u32..1 << n).collect();
        masks.sort_by_key(|m| (m.count_ones(), *m));
        masks
            .into_iter()
            .find(|&mask| {
                let generic: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
                let mut rest = cost.with_twobrids_paid(&generic);
                // Phyrexian symbols are announced next; the advice reads them as their color.
                for (i, k) in std::mem::take(&mut rest.phyrexian).into_iter().enumerate() {
                    rest.symbols[Color::ALL[i].mana() as usize] += k;
                }
                can_afford(cx, player, source, &if any_type { rest.paid_with_any_type() } else { rest })
            })
            .map(|mask| (0..n as u8).filter(|&i| mask & (1 << i) != 0).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    // A choice of modes holds at most 255 (its indices are a byte), and a cost can hold more of these symbols (255 of each
    // color): past that they are asked about a page at a time, in order, each still the player's choice.
    const PAGE: usize = u8::MAX as usize;
    let mut generic = vec![false; n];
    for start in (0..n).step_by(PAGE) {
        let page = start..n.min(start + PAGE);
        let ask = RequestKind::ChooseModes {
            source,
            modes: symbols[page.clone()].iter().map(|&c| Label::PayGeneric(c)).collect(),
            eligible: (0..page.len() as u8).collect(),
            count: 0,
            at_most: page.len() as u8,
            affordable: if start == 0 { affordable.clone() } else { Vec::new() },
            additional_cost: true,
            repeat: false,
        };
        let chosen = match cx.ask_core(player, ask, true)? {
            Answer::Modes(chosen) => chosen,
            _ => return Err(Stop::Cancel),
        };
        for i in chosen {
            generic[page.start + i as usize] = true;
        }
    }
    *cost = cost.with_twobrids_paid(&generic);
    Ok(())
}

/// Splits each hybrid symbol of the reductions that apply to a cost (CR 118.7e): the payer chooses one half of each as the
/// reduction is applied - of {U/R} {U} or {R}, of {2/R} {R} or {2} of generic mana (`Label::ReduceAs`, `Label::ReduceGeneric`,
/// each the second half; unchosen, the first). Asked only where a reduction has such a symbol. The halves, for
/// `split_reduction`.
fn announce_reduction(cx: &mut Cx, player: PlayerId, source: ObjectId, costed: Costed, cost: &ManaCost, targets: &[Target]) -> R<Vec<bool>> {
    let (more, whole, ..) = deltas(cx, &cost_change_sources(cx), player, costed, None, targets);
    let n = reduction_choices(&whole);
    if n == 0 {
        return Ok(Vec::new());
    }
    let mut modes: Vec<Label> = whole.hybrid.iter().flatten().map(|&(a, b)| Label::ReduceAs(a, b, b)).collect();
    for (i, &k) in whole.twobrid.iter().enumerate() {
        modes.extend(std::iter::repeat_n(Label::ReduceGeneric(Color::ALL[i]), k as usize));
    }
    // Advice: the fewest second halves that leave a total the player could pay.
    let affordable: Vec<u8> = if n <= 8 {
        let mut masks: Vec<u32> = (0u32..1 << n).collect();
        masks.sort_by_key(|m| (m.count_ones(), *m));
        masks
            .into_iter()
            .find(|&mask| {
                let split: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
                let mut total = *cost;
                total.generic = (total.generic as u32 + more).min(255) as u8;
                can_afford(cx, player, source, &reduce(total, &split_reduction(&whole, &split)))
            })
            .map(|mask| (0..n as u8).filter(|&i| mask & (1 << i) != 0).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let ask = RequestKind::ChooseModes {
        source,
        modes,
        eligible: (0..n as u8).collect(),
        count: 0,
        at_most: n as u8,
        affordable,
        additional_cost: true,
        repeat: false,
    };
    let chosen = match cx.ask_core(player, ask, true)? {
        Answer::Modes(chosen) => chosen,
        _ => return Err(Stop::Cancel),
    };
    Ok((0..n as u8).map(|i| chosen.contains(&i)).collect())
}

/// Announces the half each two-color hybrid symbol of a cost is paid with (CR 601.2b), where a reduction meets it
/// (`met_hybrids`): announced as a half the reduction takes, the symbol is reduced to nothing (CR 118.7c); as the other, it
/// is paid with that half and the reduction takes generic mana instead - which a player may want, to spend mana of that
/// color ("if {R} was spent to cast this spell"). Each met symbol is announced as a half the reduction takes unless the
/// player chooses its other half (`Label::PayHybridAs`). A symbol no reduction meets is paid either way as the cost is paid,
/// which comes to the same, and is not asked about. Asked once the targets are announced, since a reduction may read
/// them (CR 601.2f): an announcement nothing reads before the total cost is determined is the same then as before them.
/// `any_type`: mana of any type may pay the cost (CR 609.4b), which the advice reads.
fn announce_hybrids(cx: &mut Cx, player: PlayerId, source: ObjectId, costed: Costed, cost: &mut ManaCost, targets: &[Target], any_type: bool) -> R<()> {
    if cost.hybrid.iter().all(Option::is_none) {
        return Ok(());
    }
    let (more, whole, ..) = deltas(cx, &cost_change_sources(cx), player, costed, None, targets);
    // A reduction by hybrid mana is split after this (CR 118.7e): a cost symbol either half could meet is asked about.
    let less = reduction_reach(&whole);
    let met = met_hybrids(cost, &less);
    if met.is_empty() {
        return Ok(());
    }
    // Each met symbol: its slot, the half it is announced as unless chosen otherwise, and its other half.
    let symbols: Vec<(usize, Mana, Mana)> = met
        .iter()
        .map(|(slot, halves)| {
            let (a, b) = cost.hybrid[*slot].expect("a hybrid symbol");
            let taken = halves[0];
            (*slot, taken, if taken == a { b } else { a })
        })
        .collect();
    let announced = |chosen: &[bool]| {
        let mut out = *cost;
        for (i, &(slot, taken, other)) in symbols.iter().enumerate() {
            out.hybrid[slot] = None;
            let half = if chosen[i] { other } else { taken };
            out.symbols[half as usize] += 1;
        }
        let kept: Vec<Option<(Mana, Mana)>> = out.hybrid.iter().copied().filter(Option::is_some).collect();
        out.hybrid = [None; 6];
        out.hybrid[..kept.len()].copy_from_slice(&kept);
        out
    };
    let n = symbols.len();
    // Advice: the fewest symbols announced as their other half that leave a total the player could pay.
    let affordable: Vec<u8> = {
        let mut masks: Vec<u32> = (0u32..1 << n).collect();
        masks.sort_by_key(|m| (m.count_ones(), *m));
        masks
            .into_iter()
            .find(|&mask| {
                let chosen: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
                let mut total = announced(&chosen);
                total.generic = (total.generic as u32 + more).min(255) as u8;
                let total = reduce(total, &less);
                can_afford(cx, player, source, &if any_type { total.paid_with_any_type() } else { total })
            })
            .map(|mask| (0..n as u8).filter(|&i| mask & (1 << i) != 0).collect())
            .unwrap_or_default()
    };
    let ask = RequestKind::ChooseModes {
        source,
        modes: symbols.iter().map(|&(slot, _, other)| {
            let (a, b) = cost.hybrid[slot].expect("a hybrid symbol");
            Label::PayHybridAs(a, b, other)
        }).collect(),
        eligible: (0..n as u8).collect(),
        count: 0,
        at_most: n as u8,
        affordable,
        additional_cost: true,
        repeat: false,
    };
    let chosen = match cx.ask_core(player, ask, true)? {
        Answer::Modes(chosen) => chosen,
        _ => return Err(Stop::Cancel),
    };
    let chosen: Vec<bool> = (0..n as u8).map(|i| chosen.contains(&i)).collect();
    *cost = announced(&chosen);
    Ok(())
}

/// Announces which Phyrexian symbols are paid with 2 life (CR 107.4f, 601.2b), as the C# engine's prompt: a choice of
/// up to as many as the life total covers, cheapest first. The symbols not paid with life become colored mana in
/// `cost`. Returns the life to pay with the rest of the cost. `any_type`: mana of any type may pay it (CR 609.4b), which
/// the advice of how few to pay with life reads. `paid` is what the cost is paid for (`payable_life_for`).
///
/// A choice of modes holds at most `PAGE` of them (its indices are a byte), and a cost can hold more Phyrexian symbols
/// than that (one of each color 255 times: a cost to attack, Norn's Annex beside other colors' taxes). Then the symbols are
/// asked about a page at a time, in order, each page up to as many as the life left over from the pages before covers:
/// every symbol is still the player's choice, and the life asked never passes what they can pay.
fn announce_life(cx: &mut Cx, player: PlayerId, source: ObjectId, cost: &mut ManaCost, any_type: bool, paid: Option<CostFor>) -> R<u32> {
    const PAGE: usize = u8::MAX as usize;
    let mut symbols: Vec<Color> = Vec::new();
    for (i, &n) in cost.phyrexian.iter().enumerate() {
        for _ in 0..n {
            symbols.push(Color::ALL[i]);
        }
    }
    // The cost with the symbols not paid with life as their colors' mana (CR 107.4f). A colored symbol past what a cost
    // holds stays at the most it holds, as every count of a cost's symbols does (`ManaCost::plus`).
    let with_colors = |with_life: &[bool]| {
        let mut rest = *cost;
        for (c, _) in symbols.iter().zip(with_life).filter(|(_, life)| !**life) {
            let mana = &mut rest.symbols[c.mana() as usize];
            *mana = mana.saturating_add(1);
        }
        rest.phyrexian = [0; 5];
        rest
    };
    let have = payable_life_for(cx, player, paid).max(0) as u32;
    let mut with_life = vec![false; symbols.len()];
    let mut life = 0u32;
    for start in (0..symbols.len()).step_by(PAGE) {
        let page = start..symbols.len().min(start + PAGE);
        // Without the life for any, every symbol left is paid with its color.
        let most = page.len().min(((have - life) / 2) as usize) as u8;
        if most == 0 {
            break;
        }
        // Advice: the fewest symbols of the page that, paid with life, leave a cost the player could pay.
        let affordable = (0..=most)
            .find(|&k| {
                let mut trial = with_life.clone();
                trial[page.start..page.start + k as usize].fill(true);
                let rest = with_colors(&trial);
                can_afford(cx, player, source, &if any_type { rest.paid_with_any_type() } else { rest })
            })
            .map(|k| (0..k).collect())
            .unwrap_or_default();
        let ask = RequestKind::ChooseModes {
            source,
            modes: symbols[page.clone()].iter().map(|&c| Label::PayLife(c)).collect(),
            eligible: (0..page.len() as u8).collect(),
            count: 0,
            at_most: most,
            affordable,
            additional_cost: true,
            repeat: false,
        };
        let chosen = match cx.ask_core(player, ask, true)? {
            Answer::Modes(chosen) => chosen,
            _ => return Err(Stop::Cancel),
        };
        for i in chosen {
            with_life[page.start + i as usize] = true;
            life += 2;
        }
    }
    *cost = with_colors(&with_life);
    Ok(life)
}

/// A cost with its kicker paid `n` times (CR 702.33); a kicker with {X} in it is paid once for the X announced, which
/// is `n`, so it adds that much generic mana (and its other symbols once, when n is not 0).
fn kicker_total(base: ManaCost, k: Kicker, n: u16) -> ManaCost {
    let mut total = base;
    if k.cost.x > 0 {
        if n > 0 {
            total = add_cost(total, ManaCost { x: 0, ..k.cost });
            total.generic = total.generic.saturating_add((n as u32 * k.cost.x as u32).min(255) as u8);
        }
        return total;
    }
    for _ in 0..n {
        total = add_cost(total, k.cost);
    }
    total
}

/// Announces whether an optional cost that is not mana is paid (CR 601.2b), as the C# engine asks it: a choice between
/// casting without it and paying it, the second on offer only when it could be paid. Whether it was chosen.
fn announce_optional(cx: &mut Cx, player: PlayerId, spell: ObjectId, face: CardId, def: &'static CardDef, optional: &OptionalCost) -> R<bool> {
    // A spell that must choose more modes when it is paid ("choose both instead") can be cast that way only when that
    // many modes could be chosen (CR 700.2a, 601.2b): paying is not on offer otherwise.
    let asker = Asker { source: spell, controller: player, by: None, spell: true };
    let modes_allow = def.modal.is_none_or(|modal| {
        modal.when_paid.is_none_or(|(least, _)| eligible_modes(cx, &asker, modal).len() >= least.max(1) as usize)
    });
    let payable = modes_allow && can_pay_other(cx, spell, player, &optional.cost);
    let ask = RequestKind::ChooseModes {
        source: spell,
        modes: (0..def.labels.len().min(2) as u8).map(|i| Label::Card(face, i)).collect(),
        eligible: if payable { vec![0, 1] } else { vec![0] },
        count: 1,
        at_most: 0,
        affordable: Vec::new(),
        additional_cost: true,
        repeat: false,
    };
    match cx.ask_core(player, ask, true)? {
        Answer::Modes(chosen) => Ok(chosen.first() == Some(&1)),
        _ => Err(Stop::Cancel),
    }
}

/// Chooses which additional cost is paid (CR 601.2b): only the ones that can be paid are offered, and a lone one is
/// taken without asking, as the C# engine does.
fn choose_additional(cx: &mut Cx, player: PlayerId, spell: ObjectId, face: CardId, def: &'static CardDef) -> R<Cost> {
    let payable: Vec<u8> = (0..def.additional_choice.len() as u8)
        .filter(|&i| can_pay_other(cx, spell, player, &def.additional_choice[i as usize].cost))
        .collect();
    let Some(&first) = payable.first() else { return Err(Stop::Cancel) };
    let mut picked = first;
    if payable.len() > 1 {
        let ask = RequestKind::ChooseModes {
            source: spell,
            modes: (0..def.additional_choice.len() as u8).map(|i| Label::Extra(face, i)).collect(),
            eligible: payable,
            count: 1,
            at_most: 0,
            affordable: Vec::new(),
            additional_cost: true,
            repeat: false,
        };
        match cx.ask_core(player, ask, true)? {
            Answer::Modes(chosen) => picked = chosen.first().copied().unwrap_or(first),
            _ => return Err(Stop::Cancel),
        }
    }
    Ok(def.additional_choice[picked as usize].cost)
}

/// Announcing X or the number of times a spell is kicked (CR 601.2b), as the C# engine asks it: a pay-costs
/// question that raises and lowers the value until confirmed.
fn announce(cx: &mut Cx, player: PlayerId, spell: ObjectId, base: ManaCost, x_symbols: u8, least: u16, cap: u16) -> R<u16> {
    // X itself is a u16 (`StackItem::x`), and `cap` keeps the number within it.
    announce_x(cx, player, spell, base, None, x_symbols, u32::from(least), u32::from(cap), false, 0, 0, "").map(|n| n as u16)
}

/// The least value X may be announced as (CR 107.3a, 601.2b): 1 for an X that can't be 0 ("X can't be 0"), else 0.
pub(crate) const fn least_x(x_cant_be_zero: bool) -> u16 {
    x_cant_be_zero as u16
}

/// Whether a cost paid for something whose X can't be 0 (CR 107.3a) leaves no X that may be announced: the printed cost
/// has an {X} (`printed_x`), but the cost paid has none (`paid_x`), so X could only be 0 (CR 107.3b for a spell cast
/// without paying its mana cost or for an alternative cost with no {X}; the same for an ability's alternative cost, CR
/// 118.9 by way of 602.2b). Such a cast or activation can't be made (Mind Grind's ruling).
pub(crate) const fn x_only_zero(x_cant_be_zero: bool, printed_x: u8, paid_x: u8) -> bool {
    x_cant_be_zero && printed_x > 0 && paid_x == 0
}

/// `cost` as the least it could be announced to come to: each {X} as the least X may be (`least_x`), in generic mana.
/// The `x` symbols are kept, for mana that pays for spells of a mana value at an X that brings it there
/// (`Available::covers_for`).
fn at_least_x(mut cost: ManaCost, x_cant_be_zero: bool) -> ManaCost {
    cost.generic = cost.generic.saturating_add(cost.x.saturating_mul(least_x(x_cant_be_zero) as u8));
    cost
}

/// `announce`, saying whether the X is a "−X" loyalty cost's (`RequestKind::PayCosts::loyalty`), how much generic mana
/// each one announced takes off the total shown (`VariableCost::SacrificeAnyNumber`'s `less_each`), and how much a
/// reduction already announced takes off it (`reduced`: the cards chosen for `ExileFromHandForLess`, CR 601.2b, 601.2f,
/// 118.7a). The value starts at `least` (1 for an X that can't be 0, CR 107.3a), or at the X the reduction pays for where
/// that is more, and never goes below `least`; with no value from `least` to `cap` there is nothing that may be announced,
/// and the spell or ability is taken back (CR 601.2b, 733.1).
#[allow(clippy::too_many_arguments)]
fn announce_x(cx: &mut Cx, player: PlayerId, spell: ObjectId, base: ManaCost, kicker: Option<Kicker>, x_symbols: u8, least: u32, cap: u32, loyalty: bool, less_each: u8, reduced: u32, governs: &'static str) -> R<u32> {
    let most: u32 = match kicker {
        Some(Kicker { multi: false, cost, .. }) if cost.x == 0 => 1,
        _ => cap,
    };
    if least > most {
        return Err(Stop::Cancel);
    }
    // The X the reduction pays for: the most X whose generic mana it takes off whole.
    let paid_for = if kicker.is_none() && x_symbols > 0 { reduced / u32::from(x_symbols) } else { 0 };
    let mut n: u32 = paid_for.clamp(least, most);
    loop {
        let mut total = base;
        match kicker {
            Some(k) => total = kicker_total(total, k, n as u16),
            // A reduction past the generic mana there is takes it all, never colored mana (CR 118.7a).
            None if x_symbols > 0 => {
                let generic = (u32::from(total.generic) + n.saturating_mul(u32::from(x_symbols))).saturating_sub(reduced);
                total.generic = generic.min(255) as u8;
            }
            // A reduction past the generic mana there is takes it all, so the most it is matters, never the rest (CR 118.7a).
            None => total.generic = total.generic.saturating_sub(n.saturating_mul(u32::from(less_each)).min(255) as u8),
        }
        let mut options = Vec::new();
        if n < most {
            options.push(ActionOption::simple(ActionKind::IncreaseX));
        }
        if n > least {
            options.push(ActionOption::simple(ActionKind::DecreaseX));
        }
        options.push(ActionOption::simple(ActionKind::ConfirmCasting));
        options.push(ActionOption::simple(ActionKind::CancelCasting));
        let ask = RequestKind::PayCosts { source: spell, total, remaining: total, options, announcing: true, x: n, most, least, loyalty, attackers: Vec::new(), life: 0, governs, assist: AssistStep::None };
        match cx.ask_core(player, ask, true)? {
            // The whole value at once, which the request's `least` and `most` bound (`validate::check`).
            Answer::Announce(value) if (least..=most).contains(&value) => return Ok(value),
            Answer::Act(a) => match a.kind {
                ActionKind::IncreaseX => n += 1,
                ActionKind::DecreaseX => n -= 1,
                ActionKind::ConfirmCasting => return Ok(n),
                _ => return Err(Stop::Cancel),
            },
            _ => return Err(Stop::Cancel),
        }
    }
}

/// The most an activation's X may be announced as where it counts targets ("untap X target lands"): no more than the legal
/// targets there are (CR 601.2c, 602.2b).
fn x_target_cap(cx: &Cx, asker: &Asker, def: &ActivatedDef) -> u16 {
    def.targets
        .iter()
        .filter(|spec| spec.min == TargetSpec::COUNTED)
        .map(|spec| u16::try_from(target_candidates(cx, asker, spec).len()).unwrap_or(u16::MAX))
        .min()
        .unwrap_or(u16::MAX)
}

/// Activating a non-mana ability (CR 602.2): on the stack, then its costs.
#[allow(clippy::too_many_arguments)]
fn activate(
    cx: &mut Cx,
    player: PlayerId,
    source: ObjectId,
    ability: AbilityRef,
    index: u8,
    targets: &[Target],
    granted: Option<Grant>,
    paid_by: Option<Grant>,
    by: Option<Linked>,
) -> R<()> {
    let def = activated(cx, ability).expect("activated");
    let paid = activating(def);
    // What gave the source this ability (CR 113.2c, 613.1f): the option names it; read now, before any cost moves it (a cost
    // may sacrifice the very permanent that gave it).
    let granted = granted.or_else(|| granted_by(cx, source, None, index as usize, ability, &std::cell::OnceCell::new()));
    // "Sacrifice [the card that gives this ability]" (CR 201.5a): that permanent, which the payer must control.
    let granter = if def.cost.sacrifice_granter {
        match granted.map(|g| g.object).filter(|&g| live(cx.s(), g) && cx.s().obj(g).zone.is(ZoneKind::Battlefield) && cx.s().obj(g).controller == player) {
            Some(g) => Some(g),
            None => return Err(Stop::Cancel),
        }
    } else {
        None
    };
    let mut item = stack_item(StackKind::Activated, source, ability, player, targets);
    // How the source acquired the ability (CR 607.5), read before any cost moves it: what it is linked by.
    item.acquired = acquired_nth(cx, source, index as usize);
    let mut mana = def.cost.mana;
    // CR 118.9, 602.2b: a static ability may let the player pay another cost rather than the ability's mana cost: the
    // option it was offered as names which (`ActionOption::paid_by`), and it is paid if it still applies.
    if let Some(g) = paid_by {
        match static_rules::named_activation_alternative(&*cx, g, player, source, def) {
            Some(alternative) => mana = alternative,
            None => return Err(Stop::Cancel),
        }
        // An alternative cost with no {X} leaves an X that can't be 0 nothing to be (CR 107.3a, 118.9).
        if x_only_zero(def.x_cant_be_zero, def.cost.mana.x, mana.x) {
            return Err(Stop::Cancel);
        }
    }

    // CR 602.2b, 601.2b: a modal ability's modes first, then X, then the Phyrexian symbols paid with life.
    let mut mask = 0u32;
    if let Some(modal) = def.modal {
        mask = choose_modes(cx, player, source, false, ability, modal, (modal.min, modal.max), mana, by)?;
        item.modes = mask;
    }
    if mana.x > 0 {
        // "X target lands" cannot be announced past the legal targets there are (CR 601.2c, 602.2b).
        let cap = x_target_cap(cx, &Asker { source, controller: player, by, spell: false }, def);
        let n = announce(cx, player, source, mana, mana.x, least_x(def.x_cant_be_zero), cap)?;
        mana.generic = mana.generic.saturating_add((u32::from(n) * u32::from(mana.x)).min(255) as u8);
        item.x = n;
    }
    // CR 606.4, 107.3, 601.2b by way of 602.2b: "−X" announces X, up to the loyalty counters there are to remove (CR 118.3).
    if def.cost.loyalty == Some(LOYALTY_MINUS_X) {
        let cap = cx.s().obj(source).counters(CounterKind::Loyalty).max(0) as u16;
        item.x = announce_x(cx, player, source, mana, None, 0, u32::from(least_x(def.x_cant_be_zero)), u32::from(cap), true, 0, 0, "")? as u16;
    }
    // CR 118.3, 602.2b: "Remove any number of [kind] counters from this" announces how many, up to the counters there are; the
    // number is the ability's X (CR 107.3), which its effect reads as "that many".
    let removes = match def.cost.variable {
        Some(VariableCost::RemoveAnyNumberOfCounters(kind)) => {
            let cap = cx.s().obj(source).counters(kind).clamp(0, i16::MAX) as u32;
            item.x = announce_x(cx, player, source, mana, None, 0, 0, cap, false, 0, 0, VariableCost::RemoveAnyNumberOfCounters(kind).governs())? as u16;
            Some((kind, item.x))
        }
        _ => None,
    };
    announce_twobrids(cx, player, source, &mut mana, false)?;
    let life = announce_life(cx, player, source, &mut mana, false, paid)?;
    // What the mana paying its cost pays for (CR 106.6): the source as the ability is activated, as it begins to be put on
    // the stack (CR 602.2a, 113.7a), before any cost is paid - a cost that sacrifices or exiles it, or a mana ability
    // activated while paying that does (CR 601.2g-h), leaves it the source it was.
    let chars = cx.chars(source);
    let paying = ability_payment(cx, source, chars.types, chars.subtypes);
    // CR 602.2a, 701.27f: put on the stack now, before its costs are paid; a slot counted by X reads the X announced.
    item.counted = item.x;
    item.since = cx.g.log.len() as u32;
    cx.st().stack_push(item);

    let mut targets: Vec<Target> = targets.to_vec();
    if let Some(modal) = def.modal {
        let of;
        (targets, of) = choose_slots(cx, player, source, false, &mode_specs(modal, mask), u32::from(item.x), 0, announced_x(&item), by)?;
        set_item_targets(cx, &targets, &of);
    } else if !targets_with_activation(def) && !def.targets.is_empty() {
        // CR 601.2c by way of 602.2b: slots that are not announced with the action are chosen now, after X (a slot
        // counted by X, as Candelabra of Tawnos's "untap X target lands", reads the X just announced).
        let of;
        (targets, of) = choose_slots(cx, player, source, false, def.targets, u32::from(item.x), 0, announced_x(&item), by)?;
        set_item_targets(cx, &targets, &of);
    }
    // CR 601.2f by way of 602.2b: the total cost, with what increases or reduces it, once the targets are announced: a
    // change may read them ("equip abilities you activate that target this creature cost {2} less"). It is locked in
    // here, so nothing that happens to a target later changes what is paid. Life a change adds (CR 118.8) is payable only
    // with that much life, the rest of the cost's included (CR 119.4, 118.3), or the activation is taken back.
    announce_hybrids(cx, player, source, Costed::Ability(source, ability, by), &mut mana, &targets, false)?;
    let split = announce_reduction(cx, player, source, Costed::Ability(source, ability, by), &mana, &targets)?;
    let added_life;
    (mana, added_life) = {
        let (cost, life, least) = change_parts_split(cx, &cost_change_sources(cx), player, Costed::Ability(source, ability, by), mana, None, &targets, &split);
        (at_least(cost, least), life)
    };
    let life = life + added_life;
    if added_life > 0 && i64::from(payable_life_for(cx, player, paid)) < i64::from(life) + i64::from(def.cost.life) {
        return Err(Stop::Cancel);
    }

    // CR 601.2g-h by way of 602.2b: the permanents the cost names are chosen with the rest of the cost, before the mana,
    // except tapped ones ("return a tapped creature you control"), chosen once the mana is paid, as a spell's are
    // (`cast`): a mana ability activated while paying may have tapped them.
    let late = permanents_after_mana(&def.cost);
    let mut chosen = if late { Vec::new() } else { choose_permanents(cx, player, source, &def.cost, paid, by)? };
    let discards = choose_discards(cx, player, source, &def.cost, by)?;
    let exiles = choose_exiles(cx, player, source, &def.cost)?;
    let counter = choose_counter_kind(cx, player, source, &def.cost)?;
    pay_mana_with(
        cx,
        player,
        source,
        mana,
        &Helpers {
            waterbend: if def.waterbend { u8::MAX } else { 0 },
            creatures: def.tap_creatures_for_mana,
            purpose: Paying::Activating(paying),
            ..Helpers::NONE
        },
    )?;
    if late {
        chosen = choose_permanents(cx, player, source, &def.cost, paid, by)?;
    }
    // The creatures a crew or saddle cost taps, once the mana is paid (CR 601.2g-h by way of 602.2b), as a spell's are
    // (`cast`): a mana ability activated while paying a cost increase (Anointed Peacekeeper's) may have tapped one, and a
    // tapped creature can't pay the cost.
    let tappers = choose_tappers(cx, player, source, &def.cost)?;
    // What an attacking creature the cost returns or sacrifices was attacking, for the effect to read (ninjutsu,
    // CR 702.49a: the ninja attacks the same player, planeswalker or battle).
    if let Some(first) = chosen.first()
        && let Some(attack) = cx.s().attackers.iter().find(|a| a.id == *first).copied()
    {
        let at = cx.s().stack.len() - 1;
        let mut with = cx.s().stack[at];
        with.subject = attack.target;
        cx.st().stack_set(at, with);
    }
    // The permanents the cost chose stay with the ability, for its effect to name ("the tapped creature's power",
    // `Query::cost_objects`, CR 602.2b, 601.2h, 608.2h).
    if !chosen.is_empty() || !tappers.is_empty() {
        let objects: Vec<ObjectId> = chosen.iter().chain(&tappers).copied().collect();
        let members = cx.st().members_push(&objects);
        let at = cx.s().stack.len() - 1;
        let mut with = cx.s().stack[at];
        with.members = members;
        cx.st().stack_set(at, with);
    }
    // Craft's materials are exiled as the cost is paid (CR 601.2h by way of 602.2b), after the mana abilities (CR 601.2g):
    // they are chosen now, from what is still there, so a Black Lotus sacrificed for the mana is no material. Too few left
    // is a cost that can't be paid, and the activation is taken back.
    let materials = choose_materials(cx, player, source, &def.cost, by)?;
    // The creature a blight puts its counters on (CR 701.68a) and the counters removed from among permanents (CR 118.3) are
    // chosen as they are paid, after the mana abilities (CR 601.2g-h by way of 602.2b), as a spell's additional cost's are:
    // from what is there once the mana is paid. With no creature left to blight the cost can't be paid (CR 701.68b).
    if def.cost.blight > 0 && blight_candidates(cx, player).is_empty() {
        return Err(Stop::Cancel);
    }
    let blighted = choose_blight(cx, player, source, &def.cost)?;
    let removed = choose_counters_among(cx, player, source, &def.cost)?;
    // What a behold chooses (CR 701.4a), as a spell's is chosen: once the mana is paid (CR 601.2g-h by way of 602.2b), from
    // what is still there - a permanent a mana ability sacrificed while paying is not. None left is a cost that can't be
    // paid, and the activation is taken back.
    if def.cost.behold.is_some_and(|(n, _, _)| behold_candidates(cx, None, player, &def.cost, source, by).len() < n as usize) {
        return Err(Stop::Cancel);
    }
    let beheld = choose_behold(cx, player, source, &def.cost)?;
    // The energy and life, judged again after the mana abilities (CR 601.2g-h by way of 602.2b), and the permanents the cost
    // sacrifices by naming them, which a static ability may forbid it to (CR 101.2, `may_sacrifice_for`).
    if !rest_payable(cx, player, def.cost.energy as u32, life + def.cost.life, paid)
        || def.cost.sacrifice_self && !may_sacrifice_for(cx, player, source, paid)
        || granter.is_some_and(|g| !may_sacrifice_for(cx, player, g, paid))
    {
        return Err(Stop::Cancel);
    }
    if life > 0 {
        ops::change_life(cx, player, -life_paid(life));
    }
    // The costs are paid together (CR 601.2h by way of 602.2b), so {T} is paid on the permanent before any other part of
    // the cost moves it: it was tapped as it left (CR 118.3), its last-known information says so (CR 608.2h), and
    // "whenever ... becomes tapped" sees the tap of a permanent still on the battlefield ("{T}, Exile this artifact",
    // "{T}, Sacrifice a creature" choosing itself).
    if def.cost.tap {
        ops::tap(cx, source);
    }
    // Counters put on and taken off permanents next, so a permanent the cost also sacrifices leaves with them.
    pay_blight(cx, player, &def.cost, blighted);
    pay_counters_among(cx, &removed);
    pay_permanents(cx, player, &def.cost, &chosen, Some(cx.linked(&item)));
    let revealed = pay_behold(cx, player, &beheld);
    let at = cx.s().stack.len() - 1;
    add_revealed_to_item(cx, at, &revealed);
    pay_discards(cx, &discards);
    pay_exiles(cx, &exiles);
    pay_life_and_counters(cx, player, source, &def.cost);
    if let Some(kind) = counter {
        counters::remove(cx, source, kind, 1);
    }
    if let Some((kind, n)) = removes
        && n > 0
    {
        counters::remove(cx, source, kind, n as i16);
    }
    pay_exert_and_tappers(cx, source, &def.cost, &tappers);
    note_activation(cx, source, index as usize, def);
    // Craft exiles its source with its materials, last of all (`pay_materials`).
    if def.cost.exile_self && def.cost.materials.is_none() {
        ops::move_to_for(cx, source, Zone::exile(cx.s().obj(source).owner), ZoneReason::Exile);
    }
    // Discarding this card to pay a cycling ability's cost is cycling it (CR 702.29c), typecycling's included (CR 702.29f);
    // any other "Discard this card" (channel, Harvester of Misery's) is a discard and nothing more.
    if def.cost.discard_self {
        if def.cycling {
            ops::cycle(cx, player, source);
        } else {
            ops::discard(cx, source);
        }
    }
    if let Some(loyalty) = def.cost.loyalty {
        let turn = cx.s().turn.number;
        let change = if loyalty == LOYALTY_MINUS_X { -(item.x as i16) } else { loyalty as i16 };
        cx.st().update_obj(source, |o| o.loyalty_turn = turn);
        if change > 0 {
            // CR 606.4, 118.3: loyalty counters put on as the cost, by the player activating it - what replaces counters being
            // put applies ("if you would put", Innkeeper's Talent), but not what replaces those an effect puts (Doubling
            // Season's ruling).
            counters::put(cx, source, CounterKind::Loyalty, change, player, counters::How::Cost);
        } else if change < 0 {
            counters::remove(cx, source, CounterKind::Loyalty, -change);
        }
    }
    if def.cost.sacrifice_self {
        ops::sacrifice(cx, player, &[source]);
    }
    if let Some(granter) = granter {
        ops::sacrifice(cx, player, &[granter]);
    }
    if def.cost.return_self {
        ops::move_to_for(cx, source, Zone::hand(cx.s().obj(source).owner), ZoneReason::Other);
    }
    if def.cost.materials.is_some() {
        assert!(def.cost.exile_self, "craft exiles its source with its materials (CR 702.167a)");
        pay_materials(cx, source, &materials);
    }
    cx.emit(Event::AbilityActivated { source, ability, player, by: Grant::of(granted) });
    ops::became_targets(cx, targets.iter().copied(), source, player);
    ops::commit_crime(cx, player, source, &targets);
    Ok(())
}

/// The mana a special action costs (CR 116.2): the ability's own, except for the two the rules word as "pay the mana cost of
/// the object": turning a face-down creature card face up costs its mana cost, with X as 0 (CR 701.40b, 107.3b), and
/// unlocking a Room's door the mana cost of that door (CR 709.5e).
/// What a special action costs `player` to take (CR 116.2): its mana cost - a face-down permanent's card's mana cost to turn
/// it face up, a Room door's to unlock it, the printed cost otherwise - with the static abilities that change the cost of
/// that special action applied (`Costed::SpecialAction`: "plotting cards from your hand costs {2} less", CR 116.2k, 118.7),
/// increases first, then reductions, none below {0}.
fn special_action_cost(cx: &Cx, player: PlayerId, source: ObjectId, ability: AbilityRef, def: &ActivatedDef) -> ManaCost {
    let printed = special_action_printed_cost(cx, source, ability, def);
    change_with(cx, &cost_change_sources(cx), player, Costed::SpecialAction(source, ability), printed, None, &[])
}

fn special_action_printed_cost(cx: &Cx, source: ObjectId, ability: AbilityRef, def: &ActivatedDef) -> ManaCost {
    let card = cx.s().obj(source).card;
    match cx.cards().turn_up(ability) {
        Some(TurnUp::ManaCost) => return ManaCost { x: 0, ..cx.card(card).cost },
        // The mana of its morph or disguise cost, X and all (CR 702.37f, 702.168e): the rest is `special_action_whole_cost`'s.
        Some(by) => return turn_up_cost(cx, source, by).mana,
        None => {}
    }
    if ability.card == cx.cards().intrinsic {
        // The Room the permanent is, which a copy of one is too (CR 709.5, 707.2).
        let room = cx.card(cx.chars(source).card);
        let door = match ability.index {
            cantrip_api::intrinsic::UNLOCK_LEFT => room.split.map(|h| h.0),
            cantrip_api::intrinsic::UNLOCK_RIGHT => room.split.map(|h| h.1),
            _ => None,
        };
        if let Some(door) = door {
            return door.cost;
        }
    }
    def.cost.mana
}

/// The morph or disguise cost a face-down permanent would have if it were face up (CR 702.37e, 702.168d), which turning it
/// face up `by` that keyword's special action costs: nothing where it would have none (the action is then not offered).
fn turn_up_cost(cx: &Cx, source: ObjectId, by: TurnUp) -> Cost {
    cx.turn_up_way(source, by).map_or(Cost::NONE, |w| w.turn_up)
}

/// The whole printed cost of a special action (CR 116.2): its mana (`special_action_printed_cost`), and for turning a
/// face-down permanent up by morph or disguise whatever else that cost asks - "Morph—Pay 5 life", "Morph—Discard a Zombie
/// card", "Morph—Return a Bird you control to its owner's hand" (CR 702.37e, 702.168d, 118.3). Paid as a cost is
/// (`special_action`), and judged as one before it is offered (`activated_options`).
fn special_action_whole_cost(cx: &Cx, source: ObjectId, ability: AbilityRef, def: &ActivatedDef) -> Cost {
    match cx.cards().turn_up(ability) {
        Some(by @ (TurnUp::Morph | TurnUp::Disguise)) => turn_up_cost(cx, source, by),
        _ => def.cost,
    }
}

/// What the special action `index` of `source` (numbered as its options are) costs `player` now, where a cost change makes
/// it differ from its printed cost (CR 116.2, 118.7): for the menu row, which says so ("costs {B}"). None otherwise, or for
/// anything that is not a special action.
pub fn special_action_costs(game: &crate::Game, player: PlayerId, source: ObjectId, index: u8) -> Option<ManaCost> {
    let ability = game.chars(source).abilities().nth(index as usize)?;
    let AbilityDef::Activated(def) = game.cards.ability(ability) else { return None };
    if !def.special_action {
        return None;
    }
    let changers = game.state.battlefield().iter().any(|&id| {
        game.cards.cost_changes[game.cards.current(game.state.obj(id)).0 as usize] && !game.lost_all_abilities(id)
    });
    if !changers {
        return None;
    }
    let mut copy = crate::Game { state: game.state.clone(), log: game.log.clone(), tree: crate::tree::Tree::new(), at: crate::tree::NodeId::ROOT, cards: game.cards, verify: false, max_events: game.max_events, history_end: None, check: crate::StateCheck::NONE, spells_as_cast: Default::default() };
    let cx = Cx { g: &mut copy, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    let printed = special_action_printed_cost(&cx, source, ability, def);
    let cost = special_action_cost(&cx, player, source, ability, def);
    (cost != printed).then_some(cost)
}

/// A special action (CR 116.2) from an ability: its mana cost is paid, and its effect done at once with no stack. The
/// effect asks no questions and has no targets.
fn special_action(cx: &mut Cx, player: PlayerId, source: ObjectId, ability: AbilityRef, by: Option<Linked>) -> R<()> {
    let def = activated(cx, ability).expect("activated");
    // Its cost changes applied as the payer announces and splits them (CR 601.2b, 118.7e), as a cast's are.
    let costed = Costed::SpecialAction(source, ability);
    let mut printed = special_action_printed_cost(cx, source, ability, def);
    // What else its cost asks (a morph cost's life, discards, permanents: CR 702.37e, 118.3), paid with the mana as an
    // activation's are (CR 601.2g-h by way of 602.2b); its mana is `printed`.
    let whole = Cost { mana: ManaCost::ZERO, ..special_action_whole_cost(cx, source, ability, def) };
    // An X in a morph or disguise cost is chosen as the special action is taken (CR 702.37f, 702.168e, 107.3a): announced
    // first, and kept on the permanent for its other abilities to read.
    let mut x = None;
    if printed.x > 0 {
        let n = announce(cx, player, source, printed, printed.x, 0, 1000)?;
        printed.generic = printed.generic.saturating_add((u32::from(n) * u32::from(printed.x)).min(255) as u8);
        printed.x = 0;
        x = Some(n);
    }
    // How each {2/C} and each Phyrexian symbol is paid is announced first, as for a cast or an activation (CR 601.2b, 107.4f,
    // 116.2b: a face-down Phyrexian Metamorph turned face up for {3}{U/P} may pay the {U/P} with 2 life).
    announce_twobrids(cx, player, source, &mut printed, false)?;
    // A special action is no activation (CR 116.2): what forbids paying life to activate abilities leaves it alone.
    let life = announce_life(cx, player, source, &mut printed, false, None)?;
    announce_hybrids(cx, player, source, costed, &mut printed, &[], false)?;
    let split = announce_reduction(cx, player, source, costed, &printed, &[])?;
    let (mana, _, least) = change_parts_split(cx, &cost_change_sources(cx), player, costed, printed, None, &[], &split);
    let mana = at_least(mana, least);
    // The rest of the cost is chosen with it (CR 601.2h by way of 602.2b): permanents (tapped ones once the mana is paid),
    // cards to discard or exile.
    let late = permanents_after_mana(&whole);
    let mut chosen = if late { Vec::new() } else { choose_permanents(cx, player, source, &whole, None, None)? };
    let discards = choose_discards(cx, player, source, &whole, None)?;
    let exiles = choose_exiles(cx, player, source, &whole)?;
    let tappers = choose_tappers(cx, player, source, &whole)?;
    let counter = choose_counter_kind(cx, player, source, &whole)?;
    if mana != ManaCost::ZERO {
        pay_mana(cx, player, source, mana)?;
    }
    if late {
        chosen = choose_permanents(cx, player, source, &whole, None, None)?;
    }
    if whole.blight > 0 && blight_candidates(cx, player).is_empty() {
        return Err(Stop::Cancel);
    }
    let blighted = choose_blight(cx, player, source, &whole)?;
    let removed = choose_counters_among(cx, player, source, &whole)?;
    if whole.behold.is_some_and(|(n, _, _)| behold_candidates(cx, None, player, &whole, source, None).len() < n as usize) {
        return Err(Stop::Cancel);
    }
    let beheld = choose_behold(cx, player, source, &whole)?;
    // The energy and life, judged again once the mana is paid (CR 601.2g-h, 119.4): a mana ability may have spent it.
    if !rest_payable(cx, player, u32::from(whole.energy), life + whole.life, None) || whole.sacrifice_self && !may_sacrifice_for(cx, player, source, None) {
        return Err(Stop::Cancel);
    }
    if life > 0 {
        ops::change_life(cx, player, -life_paid(life));
    }
    if whole.tap {
        ops::tap(cx, source);
    }
    pay_blight(cx, player, &whole, blighted);
    pay_counters_among(cx, &removed);
    pay_permanents(cx, player, &whole, &chosen, None);
    pay_behold(cx, player, &beheld);
    pay_discards(cx, &discards);
    pay_exiles(cx, &exiles);
    pay_life_and_counters(cx, player, source, &whole);
    if let Some(kind) = counter {
        counters::remove(cx, source, kind, 1);
    }
    pay_exert_and_tappers(cx, source, &whole, &tappers);
    if whole.sacrifice_self {
        ops::sacrifice(cx, player, &[source]);
    }
    if let Some(n) = x {
        cx.st().update_obj(source, |o| o.x = n);
    }
    let mut item = stack_item(StackKind::Activated, source, ability, player, &[]);
    // How the source acquired the ability (CR 607.5): what its effect is linked by.
    item.acquired = by.map_or_else(|| cx.s().copy_acquired(source), |by| by.acquired);
    // A special action's effect is done at once; a question it asked would have nowhere to be answered.
    let _ = (def.effect)(cx, &item);
    Ok(())
}

/// Activating a mana ability (CR 605.3): its costs, then its effect, at once.
pub fn activate_mana(
    cx: &mut Cx,
    player: PlayerId,
    source: ObjectId,
    index: u8,
    color: Option<Color>,
    colorless: bool,
    sacrifice: &[Target],
) -> R<()> {
    // Asked as the ability it is (CR 607.5): one an effect gave the source reads only its own links - its colors, its
    // restriction, its costs and its effect.
    let by = if index < PLAYER_MANA_ABILITY { cx.link_of_nth(source, index as usize) } else { None };
    activate_mana_as(cx, player, source, index, color, colorless, sacrifice, by)
}

#[allow(clippy::too_many_arguments)]
fn activate_mana_as(
    cx: &mut Cx,
    player: PlayerId,
    source: ObjectId,
    index: u8,
    color: Option<Color>,
    colorless: bool,
    sacrifice: &[Target],
    by: Option<Linked>,
) -> R<()> {
    if index >= PLAYER_MANA_ABILITY {
        return activate_player_mana(cx, player, source, index - PLAYER_MANA_ABILITY);
    }
    let ability = cx.chars(source).abilities().nth(index as usize).expect("ability");
    let def = activated(cx, ability).expect("activated");
    // How the source acquired the ability (CR 607.5), read before any cost moves it: what it is linked by.
    let acquired = acquired_nth(cx, source, index as usize);
    // What restricts its mana, read from the source as it is activated, before a cost may take it away (CR 106.6).
    let restricted = def.restricted.map(|r| restriction_from(cx, source, r, by));
    // And what mana paying its own mana cost pays for: the source as it is activated, before {T} or "exile this" is paid
    // (CR 602.2a, 113.7a, 601.2h: those costs may be paid in any order).
    let chars = cx.chars(source);
    let paying = ability_payment(cx, source, chars.types, chars.subtypes);
    // "Sacrifice a creature: Add ..." (Phyrexian Altar): the permanents come with the activation, as its option's
    // candidates (a mana ability never targets, CR 605.1a, so the slots are free for them). Tapped ones ("return a tapped
    // creature you control") are chosen once the ability's own mana is paid, as a spell's are (CR 605.3a, 601.2g-h): a mana
    // ability activated while paying it may have tapped them.
    let late = permanents_after_mana(&def.cost);
    let mut chosen: Vec<ObjectId> = if late { Vec::new() } else { sacrifice.iter().filter_map(|t| t.object()).collect() };
    // "Any type the sacrificed permanent could produce" (CR 106.7): read before it is gone, and the type named must be one.
    let mut adds_nothing = false;
    if def.colors_from_sacrifice && !late {
        adds_nothing = sacrificed_names(cx, &chosen, color, colorless)?;
    }
    // An {X} in the cost is announced first (CR 107.3a, 601.2b, 605.3a) and kept on the source for the effect to read.
    let mut mana = def.cost.mana;
    if mana.x > 0 {
        let n = announce(cx, player, source, mana, mana.x, least_x(def.x_cant_be_zero), 1000)?;
        mana.generic = mana.generic.saturating_add((n as u32 * mana.x as u32).min(255) as u8);
        mana.x = 0;
        cx.st().update_obj(source, |o| o.x = n);
    }
    note_activation(cx, source, index as usize, def);
    // How it was given, read before any cost moves the source (CR 113.7a).
    let granted = Grant::of(granted_by(cx, source, None, index as usize, ability, &std::cell::OnceCell::new()));
    // {T} is paid on the permanent before any other part of the cost moves it (CR 605.3a, 601.2h, 118.3): see `activate`.
    if def.cost.tap {
        ops::tap(cx, source);
    }
    if def.cost.exile_self {
        ops::move_to_for(cx, source, Zone::exile(cx.s().obj(source).owner), ZoneReason::Exile);
    }
    // A mana ability's own mana cost (Chromatic Star's {1}) is paid like any other (CR 605.3a, 601.2g).
    if mana != ManaCost::ZERO {
        pay_mana_with(cx, player, source, mana, &Helpers { purpose: Paying::Activating(paying), ..Helpers::NONE })?;
    }
    if late {
        chosen = choose_permanents(cx, player, source, &def.cost, Some(CostFor::ManaAbility), by)?;
        if def.colors_from_sacrifice {
            adds_nothing = sacrificed_names(cx, &chosen, color, colorless)?;
        }
    }
    pay_exert_and_tappers(cx, source, &def.cost, &[]);
    if def.cost.discard_hand {
        let hand = choose_discards(cx, player, source, &def.cost, by)?;
        pay_discards(cx, &hand);
    }
    pay_permanents(cx, player, &def.cost, &chosen, Some(Linked { object: source, text: cx.g.cards.linked_card(ability.card), acquired }));
    pay_life_and_counters(cx, player, source, &def.cost);
    if def.cost.sacrifice_self {
        ops::sacrifice(cx, player, &[source]);
    }
    // A mana ability is activated (CR 605.3) as any ability is, once its costs are paid (CR 602.2b, 601.2i, 605.3a), and the
    // history says so. It is emitted too: "whenever you activate an ability of a creature" (Elrond, Moon-Reader) triggers on a
    // mana ability's activation (its ruling), the trigger waiting for the next time a player would receive priority
    // (CR 603.3); the abilities that mean only the ones that use the stack ask `ActivatedDef::is_mana_ability` of it themselves.
    let activated = Event::AbilityActivated { source, ability, player, by: granted };
    if crate::settle::activation_watched(cx) {
        cx.emit(activated);
    } else {
        cx.record(activated);
    }
    let asker = Asker { source, controller: player, by, spell: false };
    // What the ability itself produces (CR 106.12a), told from every pool before and after it - its tapper's, and another's
    // it adds to ("that player adds", Spectral Searchlight) - of whatever restriction, but not what the triggered mana
    // abilities it triggers add. Where a replacement effect could apply, the production says it instead (`produce`).
    let before = if def.cost.tap { mana_held_everywhere(cx) } else { ManaPool::default() };
    // A permanent tapped for mana produces it as one event a replacement effect may change (CR 106.12b): where one could, the
    // mana the effect adds is held, not added, and proposed whole once the effect is done.
    let replaceable = def.cost.tap && !adds_nothing && crate::replace::mana_may_be_replaced(cx);
    let mut replaced = None;
    if !adds_nothing {
        // The mana is produced by the ability's source (CR 106.3), whatever has become of it paying the cost.
        let outer = cx.mana_source.replace(source);
        // Held for this production alone: a mana ability activated inside another's effect has its own.
        let outer_held = if replaceable { cx.producing.replace(Vec::new()) } else { cx.producing.take() };
        let added = match def.mana {
            // What the mana ability does, its activator does (CR 605.3b, 122.6a): the counters it puts are theirs.
            Some(effect) => cx.acting_as(player, |cx| effect(cx, &asker, color)),
            None => {
                let def = ActivatedDef { restricted, ..*def };
                add_listed(cx, player, &def, if colorless { Some(Mana::C) } else { color.map(Color::mana) });
                Ok(())
            }
        };
        let held = std::mem::replace(&mut cx.producing, outer_held);
        cx.mana_source = outer;
        added?;
        if let Some(held) = held.filter(|_| replaceable) {
            replaced = Some(produce(cx, player, source, &held));
        }
    }
    if def.cost.tap {
        let produced = match replaced {
            Some(produced) => produced,
            None => {
                let after = mana_held_everywhere(cx);
                ManaPool(std::array::from_fn(|m| after.0[m].saturating_sub(before.0[m])))
            }
        };
        tapped_for_mana(cx, player, source, produced)?;
    }
    Ok(())
}

/// The mana `source`, tapped for mana by `player`, produces (CR 106.12b): what its mana ability's effect added (`held`),
/// proposed as one event to the replacement effects, and what they leave of it added to the pool it was going to, produced by
/// `source` (CR 106.3) - the tapper's, or another player's ("that player adds", Spectral Searchlight), whose mana the effects
/// keyed to the tapper ("if you tap a permanent for mana") still change. An ability that added nothing produced no mana: no
/// event. Mana the effect gave a rider (CR 106.6) or made last (CR 106.4) keeps it, however the effects change its type and
/// amount. What is produced, into every pool, for its tap (`tapped_for_mana`, CR 106.12a).
fn produce(cx: &mut Cx, player: PlayerId, source: ObjectId, held: &[crate::Produced]) -> ManaPool {
    let mut produced = ManaPool::default();
    // One event for each pool and rider the mana has. A mana ability's mana goes to one pool with one rider (every printed
    // one); one that gave some of its mana a rider and some another, or sent it to two pools, would leave the effects nothing
    // to say which rider or pool the mana they make has.
    let mut parts: Vec<(PlayerId, Option<ManaRestriction>, bool)> = Vec::new();
    for h in held {
        if !parts.contains(&(h.to, h.restriction, h.lasting)) {
            parts.push((h.to, h.restriction, h.lasting));
        }
    }
    if crate::layers::verifying() {
        assert!(parts.len() <= 1, "{source:?}'s mana ability produced mana of two riders or to two pools in one production (CR 106.12b)");
    }
    for (to, restriction, lasting) in parts {
        let mut mana = ManaPool::default();
        for h in held.iter().filter(|h| (h.to, h.restriction, h.lasting) == (to, restriction, lasting)) {
            mana.0[h.mana as usize] += h.amount;
        }
        let result = crate::replace::mana_production(cx, ManaProduction { permanent: source, player, to, mana, restriction, lasting });
        for m in Mana::ALL {
            let n = result.mana.get(m);
            if n > 0 {
                cx.st().add_mana(result.to, m, n, source, result.restriction, result.lasting);
                produced.0[m as usize] += n;
            }
        }
    }
    produced
}

/// Whether a mana ability that adds "any type the sacrificed permanent could produce" (CR 106.7) adds nothing, the land
/// it sacrifices producing no type: read before the land is gone. The type the activation named must be one it could
/// produce, or the activation is taken back.
fn sacrificed_names(cx: &Cx, chosen: &[ObjectId], color: Option<Color>, colorless: bool) -> R<bool> {
    let (colors, can_colorless) = chosen.first().map_or((ColorSet::NONE, false), |&land| producible_types(cx, land));
    let adds_nothing = colors.is_empty() && !can_colorless;
    let fits = match (color, colorless) {
        (Some(c), false) => colors.has(c),
        (None, true) => can_colorless,
        (None, false) => adds_nothing,
        (Some(_), true) => false,
    };
    if fits { Ok(adds_nothing) } else { Err(Stop::Cancel) }
}

/// The mana in every player's pool, of every restriction and none, summed: what a mana ability adds to any of them is the
/// difference it makes.
fn mana_held_everywhere(cx: &Cx) -> ManaPool {
    let s = cx.s();
    let mut held = ManaPool::default();
    for p in s.seats() {
        for m in 0..6 {
            held.0[m] = held.0[m].saturating_add(s.player(p).pool.0[m]);
        }
    }
    for r in &s.restricted_mana {
        for m in 0..6 {
            held.0[m] = held.0[m].saturating_add(r.pool.0[m]);
        }
    }
    held
}

/// Activating a mana ability the player has of their own (CR 605.1a, 611.2a, 605.3): the nth of those that effects gave them.
/// Its cost is paid, then its mana added at once. It has no source object (CR 605.1a): `source` is only the card that gave it.
fn activate_player_mana(cx: &mut Cx, player: PlayerId, source: ObjectId, n: u8) -> R<()> {
    let Some(EffectKind::PlayerManaAbility(ability)) = cx
        .s()
        .effects
        .iter()
        .filter(|e| matches!(e.kind, EffectKind::PlayerManaAbility(_)) && e.controller == player)
        .nth(n as usize)
        .map(|e| e.kind)
    else {
        return Err(Stop::Cancel);
    };
    let def = activated(cx, ability).expect("activated");
    pay_life_and_counters(cx, player, source, &def.cost);
    // Its mana has no source (CR 605.1a, 106.3).
    let outer = cx.mana_source.replace(ObjectId::NONE);
    let added = match def.mana {
        Some(effect) => cx.acting_as(player, |cx| effect(cx, &Asker { source, controller: player, by: None, spell: false }, None)),
        None => {
            add_listed(cx, player, def, None);
            Ok(())
        }
    };
    cx.mana_source = outer;
    added?;
    Ok(())
}

/// The mana types a permanent could produce (CR 106.7): "any type of mana that an ability of that permanent would produce if the
/// ability were to resolve at that time, taking into account any applicable replacement effects in any possible order", costs
/// ignored. A mana ability that lists its mana adds that, and its mana of one color any it may name; one whose mana is its own
/// effect's (a filter land's "{U}{U}, {U}{B}, or {B}{B}", Gemstone Caverns' {C} or a color) is run on a copy of the position
/// for every answer to the questions it asks (`effect_productions`). Through the replacement effects on a tap for mana where
/// its cost has {T} (CR 106.12b).
pub(crate) fn producible_types(cx: &Cx, land: ObjectId) -> (ColorSet, bool) {
    let controller = cx.s().obj(land).controller;
    let mut types = 0u8;
    let replaceable = crate::replace::mana_may_be_replaced(cx);
    let chars = cx.chars(land);
    let given = layers::acquisition_possible(cx.cards(), cx.s());
    for (nth, ability) in chars.abilities().enumerate() {
        let Some(def) = activated(cx, ability) else { continue };
        if !def.is_mana_ability() || def.from != ZoneKind::Battlefield {
            continue;
        }
        // Asked as the ability it is (CR 607.5): one an effect gave the land reads only its own links.
        let by = if given { cx.link_nth(land, &chars, nth) } else { None };
        let any = if def.any_color { any_types(cx, land, controller, def, by) } else { 0 };
        // The mana it would produce, by each answer to its own questions: what its effect adds, or what it lists.
        let productions: Vec<[u32; 6]> = match def.mana.and_then(|effect| effect_productions(cx, land, controller, def, effect, by)) {
            Some(made) => made,
            None => {
                let k = if any != 0 { u32::from(def.adds_any.max(1)) } else { 0 };
                if k > 0 {
                    (0..6)
                        .filter(|&m| any & (1 << m) != 0)
                        .map(|m| {
                            let mut made = def.adds.0;
                            made[m] = made[m].saturating_add(k);
                            made
                        })
                        .collect()
                } else {
                    vec![def.adds.0]
                }
            }
        };
        let restriction = def.restricted.map(|r| restriction_from(cx, land, r, by));
        for made in productions {
            let outcomes: Option<Vec<[u32; 6]>> = if replaceable && def.cost.tap {
                replaced_yields(cx, land, controller, made, 0, 0, restriction).map(|o| o.into_iter().map(|p| p.mana.0).collect())
            } else {
                None
            };
            for o in outcomes.unwrap_or_else(|| vec![made]) {
                types |= (0..6).filter(|&m| o[m] > 0).fold(0u8, |t, m| t | 1 << m);
            }
        }
    }
    (ColorSet(types & ANY_COLOR), types & (1 << Mana::C as u8) != 0)
}

/// What the mana ability `def` of `id`, whose mana is its own `effect`'s, would add if it resolved now for `player` (CR 106.7):
/// its effect run on a copy of the position, once for each color the activation may name and each answer to each question it
/// asks as it adds (the colors of "in any combination of", a "you may", a choice of one), the mana held as a tap's would be
/// (`Cx::producing`). Each run is on the advice copy of the position every menu row's advice is judged on (`advice_game`):
/// it is asked from a view of the game (`&Cx`), in the middle of a run, where the working state can't be moved.
///
/// None where it asks a question whose answers `answers_to_try` does not list, or would need more than `MOST_RUNS` runs:
/// then what it lists stands for it, which may not be what it could produce - so with `CANTRIP_VERIFY` that panics, naming
/// the card and the question, and the first card to need it fails its scenarios. No printed land (the permanents "could
/// produce" is asked of: `colors_from_sacrifice`) asks more than one question of a few answers, the most being Cascading
/// Cataracts' five colors in any combination, 126 runs (`docs/known-issues.md`, KI-124).
fn effect_productions(cx: &Cx, id: ObjectId, player: PlayerId, def: &ActivatedDef, effect: ManaEffect, by: Option<Linked>) -> Option<Vec<[u32; 6]>> {
    let unjudged = |why: String| {
        if crate::layers::verifying() {
            panic!("{}: what its mana ability could produce (CR 106.7) can't be judged: {why}", cx.chars(id).name);
        }
        None
    };
    let named: Vec<Option<Color>> = if def.any_color {
        let colors = any_types(cx, id, player, def, by);
        let named: Vec<Option<Color>> = Color::ALL.into_iter().filter(|c| colors & (1 << c.mana() as u8) != 0).map(Some).collect();
        if named.is_empty() { vec![None] } else { named }
    } else {
        vec![None]
    };
    let mut out: Vec<[u32; 6]> = Vec::new();
    let mut runs = 0;
    for color in named {
        let mut waiting: Vec<Vec<Answer>> = vec![Vec::new()];
        while let Some(answers) = waiting.pop() {
            runs += 1;
            if runs > MOST_RUNS {
                return unjudged(format!("more than {MOST_RUNS} answers to try"));
            }
            let mut copy = advice_game(cx.g);
            copy.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Settle));
            let frame = copy.state.agenda.len() - 1;
            copy.state.update_frame(frame, |f| f.answers = answers.iter().cloned().map(|a| (None, a)).collect());
            let mut run = Cx {
                g: &mut copy,
                frame,
                cursor: 0,
                mana_source: Some(id),
                entering: None,
                lineage: Vec::new(),
                asked_at: None,
                doer: None,
                producing: Some(Vec::new()),
            };
            let asker = Asker { source: id, controller: player, by, spell: false };
            let done = run.acting_as(player, |run| effect(run, &asker, color)).is_ok();
            let held = run.producing.take().unwrap_or_default();
            if done {
                let mut made = [0u32; 6];
                for h in held {
                    made[h.mana as usize] += h.amount;
                }
                if !out.contains(&made) {
                    out.push(made);
                }
                continue;
            }
            let Some(request) = copy.state.agenda[frame].pending.clone() else {
                return unjudged("its effect stopped without a question".to_string());
            };
            let tries = match answers_to_try(&request.kind) {
                Ok(tries) => tries,
                Err(why) => return unjudged(why),
            };
            for answer in tries {
                let mut next = answers.clone();
                next.push(answer);
                waiting.push(next);
            }
        }
    }
    Some(out)
}

/// More runs than this is no mana ability of a land a card prints (KI-124).
const MOST_RUNS: usize = 256;

/// Every answer to a question a mana ability's effect asks as it adds its mana, where they are few enough to try
/// (`effect_productions`): colors for each mana (repeats allowed, their order no matter to the mana), "you may", one of
/// several modes or players. For any other question, or more colors than `MOST_RUNS` runs could try, why not.
fn answers_to_try(kind: &RequestKind) -> Result<Vec<Answer>, String> {
    match kind {
        RequestKind::ChooseColors { choices, count, .. } => {
            let colors: Vec<Color> = Color::ALL.into_iter().filter(|&c| choices.has(c)).collect();
            if colors.is_empty() {
                return Err("it asks a choice of colors among none".to_string());
            }
            // Each multiset of `count` colors, as a non-decreasing run of indices.
            let mut out = Vec::new();
            let mut at = vec![0usize; *count as usize];
            loop {
                out.push(Answer::Colors(at.iter().map(|&i| colors[i]).collect()));
                if out.len() > MOST_RUNS {
                    return Err(format!("it asks {count} mana in any combination of {} colors, more than {MOST_RUNS} ways", colors.len()));
                }
                let Some(i) = (0..at.len()).rev().find(|&i| at[i] + 1 < colors.len()) else { break };
                at[i] += 1;
                let v = at[i];
                for j in at.iter_mut().skip(i + 1) {
                    *j = v;
                }
            }
            Ok(out)
        }
        RequestKind::May { .. } => Ok(vec![Answer::Yes, Answer::No]),
        RequestKind::ChooseModes { eligible, count: 1, at_most: 0, .. } => Ok(eligible.iter().map(|&i| Answer::Modes(vec![i])).collect()),
        RequestKind::ChoosePlayer { candidates, .. } => Ok(candidates.iter().map(|&p| Answer::Player(p)).collect()),
        other => {
            let kind = format!("{other:?}");
            Err(format!("it asks {}, whose answers are not tried", kind.split([' ', '{', '(']).next().unwrap_or("")))
        }
    }
}

/// A permanent was tapped for mana (CR 106.12, 605.1b) and produced `produced`: the fact is recorded, and, if it produced any
/// mana, the triggered mana abilities it triggers (CR 106.12a) resolve at once, as part of the mana ability that triggered
/// them (CR 605.4a): "whenever you tap a Forest for mana, add an additional {G}". They are looked for among the permanents
/// that could have one (`mana_trigger_holders`). One that asks (a type of the mana that land produced) suspends the activation, to be run again
/// with the answer.
fn tapped_for_mana(cx: &mut Cx, player: PlayerId, source: ObjectId, produced: ManaPool) -> R<()> {
    let event = Event::TappedForMana { permanent: source, player, produced: ManaTally::of(produced) };
    cx.record(event);
    if produced.is_empty() {
        return Ok(());
    }
    let holders = mana_trigger_holders(cx);
    let mut statics = crate::settle::TriggerStatics::new(None);
    for id in holders {
        let controller = cx.s().obj(id).controller;
        let chars = cx.chars(id);
        for (nth, ability) in chars.abilities().collect::<Vec<_>>().into_iter().enumerate() {
            let AbilityDef::Triggered(t) = cx.cards().ability(ability) else { continue };
            if !t.mana_ability {
                continue;
            }
            // How it acquired the ability (CR 607.5): what the ability is linked by, and asked as.
            let acquired = acquired_nth(cx, id, nth);
            let by = layers::given_link(cx.cards(), cx.s(), id, ability, acquired);
            if let Some(fired) = t.fires(&*cx, &Asker { source: id, controller, by, spell: false }, &event) {
                // Static abilities that stop it triggering or make it trigger again apply to it as to any triggered ability
                // (CR 603.2, 603.2d): each triggering resolves at once.
                let triggering = Triggering { source: id, controller, def: t, permanent: true, looks_back: cx.cards().looks_back(ability, &event, false) };
                if statics.stop(cx, &event, &triggering) {
                    continue;
                }
                let mut again = crate::settle::Again::new();
                statics.additional(cx, &event, &triggering, &mut again);
                let mut item = stack_item(StackKind::Triggered, id, ability, controller, &[]);
                item.subject = fired.subject;
                item.data = fired.data;
                item.acquired = acquired;
                // A triggered mana ability's mana is produced by its own source (CR 106.3): Mana Flare's additional mana
                // is Mana Flare's, not the land's.
                let outer = cx.mana_source.replace(id);
                let mut added = Ok(());
                for _ in 0..=again.len() {
                    added = cx.acting_as(controller, |cx| (t.effect)(cx, &item));
                    if added.is_err() {
                        break;
                    }
                }
                cx.mana_source = outer;
                added?;
            }
        }
    }
    if cx.s().delayed.iter().any(|d| d.when.is_event()) {
        delayed_mana_abilities(cx, &event, &mut statics)?;
    }
    Ok(())
}

/// The delayed triggered mana abilities a mana ability's tap triggers: "until end of turn, whenever you tap a Mountain for
/// mana, add an additional {R}" (Molten Tide, High Tide, Bubbling Muck). A delayed triggered ability that meets CR 605.1b is
/// a mana ability, so it resolves at once as one of a permanent does (CR 605.4a), never on the stack; settle's delayed
/// abilities leave it to this. "You" in its trigger event is its own controller, the player who controlled the spell or
/// ability that created it (CR 603.7d-e), and it is stopped from triggering as any delayed ability is (CR 603.2) but
/// never made to trigger an additional time (it is no ability of a permanent). Its mana is produced by its source, the
/// spell or the source of the ability that created it (CR 106.3, 603.7d-e), wherever that has gone.
fn delayed_mana_abilities(cx: &mut Cx, event: &Event, statics: &mut crate::settle::TriggerStatics) -> R<()> {
    let delayed: Vec<cantrip_state::Delayed> = cx
        .s()
        .delayed
        .iter()
        .filter(|d| d.when.is_event() && matches!(cx.cards().ability(d.ability), AbilityDef::Triggered(t) if t.mana_ability))
        .cloned()
        .collect();
    for d in delayed {
        let AbilityDef::Triggered(t) = cx.cards().ability(d.ability) else { continue };
        // Asked as the ability that created it was acquired (CR 607.5).
        let asker = Asker { source: d.source, controller: d.controller, by: layers::given_link(cx.cards(), cx.s(), d.source, d.ability, d.acquired), spell: false };
        let Some(fired) = t.fires(&*cx, &asker, event) else { continue };
        if t.condition.is_some_and(|condition| !condition(&*cx, &asker))
            || statics.stop(
                cx,
                event,
                &Triggering { source: d.source, controller: d.controller, def: t, permanent: false, looks_back: cx.cards().looks_back(d.ability, event, false) },
            )
        {
            continue;
        }
        // "When you next" is used up by the event it triggers on (CR 603.7b).
        if d.when == DelayedWhen::NextThisTurn
            && let Some(at) = cx.s().delayed.iter().position(|x| *x == d)
        {
            cx.st().delayed_remove(at);
        }
        let mut item = stack_item(StackKind::Triggered, d.source, d.ability, d.controller, &[]);
        item.subject = fired.subject;
        item.data = fired.data;
        item.acquired = d.acquired;
        let outer = cx.mana_source.replace(d.source);
        let added = cx.acting_as(d.controller, |cx| (t.effect)(cx, &item));
        cx.mana_source = outer;
        added?;
    }
    Ok(())
}

// --- Paying ----------------------------------------------------------------------------------------------------------

/// What is still owed after spending one unit of `mana`: a colored symbol of its color first, else generic.
fn spend(owed: &mut ManaCost, mana: Mana) {
    if owed.symbols[mana as usize] > 0 {
        owed.symbols[mana as usize] -= 1;
    } else if let Some(slot) = owed.hybrid.iter().position(|h| matches!(h, Some((a, b)) if *a == mana || *b == mana)) {
        owed.hybrid[slot] = None;
    } else if owed.generic > 0 {
        owed.generic -= 1;
    }
}

/// What a payment may spend on a cost's symbols (CR 609.4b): only the mana each asks for, mana as though it were of any
/// color (Mycosynth Lattice: a colored or hybrid symbol), or mana of any type (Cruelclaw's Heist: {C} too).
#[derive(Clone, Copy, PartialEq, Eq)]
enum AnyMana {
    No,
    Color,
    Type,
}

impl AnyMana {
    /// The mana a payer is asked for: `owed` with every symbol any mana may pay as generic.
    fn view(self, owed: &ManaCost) -> ManaCost {
        match self {
            AnyMana::No => *owed,
            AnyMana::Color => owed.paid_with_any_color(),
            AnyMana::Type => owed.paid_with_any_type(),
        }
    }

    /// `spend`, where a unit may also pay a symbol of another type: its own symbol first, then another it may pay, the
    /// generic mana last, which leaves what pays only generic mana (delve, improvise) the most to pay.
    fn spend(self, owed: &mut ManaCost, mana: Mana) {
        let own = owed.symbols[mana as usize] > 0 || owed.hybrid.iter().flatten().any(|&(a, b)| a == mana || b == mana);
        if self == AnyMana::No || own {
            return spend(owed, mana);
        }
        // Mana as though of any color pays no {C}, which no color stands for.
        let types = if self == AnyMana::Type { 6 } else { 5 };
        if let Some(i) = (0..types).find(|&i| owed.symbols[i] > 0) {
            owed.symbols[i] -= 1;
        } else if let Some(slot) = owed.hybrid.iter().position(Option::is_some) {
            owed.hybrid[slot] = None;
        } else if owed.generic > 0 {
            owed.generic -= 1;
        }
    }
}

/// Whether a unit of `mana` pays anything still owed.
fn pays(owed: &ManaCost, mana: Mana) -> bool {
    owed.generic > 0
        || owed.symbols[mana as usize] > 0
        || owed.hybrid.iter().flatten().any(|(a, b)| *a == mana || *b == mana)
}

/// How the pool would pay `owed`: none (it cannot), exactly one way, or more than one.
pub(crate) enum Ways {
    None,
    One([u8; 6]),
    Many,
}

/// What of `owed` the pool does not cover, for the payer to make mana for: each colored symbol the pool has the color for
/// is covered, a hybrid or Phyrexian symbol by either of its colors, and the generic mana by what is left over. Advice for
/// the prompt, not a payment: the pool is spent once it covers the whole cost.
fn uncovered(pool: &ManaPool, owed: &ManaCost) -> ManaCost {
    let mut left = pool.0;
    let mut out = *owed;
    for mana in Mana::ALL {
        let i = mana as usize;
        let take = left[i].min(u32::from(out.symbols[i]));
        left[i] -= take;
        out.symbols[i] -= take as u8;
    }
    for slot in out.hybrid.iter_mut() {
        if let Some((a, b)) = *slot {
            if let Some(m) = [a, b].into_iter().find(|&m| left[m as usize] > 0) {
                left[m as usize] -= 1;
                *slot = None;
            }
        }
    }
    for (i, n) in out.phyrexian.iter_mut().enumerate() {
        let take = left[i].min(u32::from(*n));
        left[i] -= take;
        *n -= take as u8;
    }
    let spare: u32 = left.iter().sum();
    out.generic = out.generic.saturating_sub(spare.min(255) as u8);
    out
}

pub(crate) fn ways(pool: &ManaPool, owed: &ManaCost) -> Ways {
    // How a Phyrexian or a monocolored hybrid symbol is paid is announced before the pool pays it (`announce_life`,
    // `announce_twobrids`, `pool_pays`). One not announced is never free: mana pays a Phyrexian symbol only with its color
    // (CR 107.4f), and a {2/C} symbol is paid with its color or {2} (CR 107.4e), a choice of the payer's.
    let mut owed = *owed;
    for (color, n) in std::mem::take(&mut owed.phyrexian).into_iter().enumerate() {
        let mana = &mut owed.symbols[Color::ALL[color].mana() as usize];
        let Some(more) = mana.checked_add(n) else { return Ways::None };
        *mana = more;
    }
    if owed.twobrid != [0; 5] {
        return if pool_pays(pool, &owed, 0, false) { Ways::Many } else { Ways::None };
    }
    let owed = &owed;
    let mut left = pool.0;
    let mut spent = [0u8; 6];
    for mana in Mana::ALL {
        let need = owed.symbols[mana as usize];
        if left[mana as usize] < u32::from(need) {
            return Ways::None;
        }
        left[mana as usize] -= u32::from(need);
        spent[mana as usize] += need;
    }
    // A hybrid symbol is paid one way when only one of its colors is in the pool, and is a choice when both are.
    for (a, b) in owed.hybrid.iter().flatten() {
        match (left[*a as usize] > 0, left[*b as usize] > 0) {
            (true, false) => {
                left[*a as usize] -= 1;
                spent[*a as usize] += 1;
            }
            (false, true) => {
                left[*b as usize] -= 1;
                spent[*b as usize] += 1;
            }
            (true, true) => return Ways::Many,
            (false, false) => return Ways::None,
        }
    }
    let generic = owed.generic;
    let spare: u32 = left.iter().sum();
    if spare < generic as u32 {
        return Ways::None;
    }
    let kinds = left.iter().filter(|&&n| n > 0).count();
    if generic == 0 || spare == generic as u32 || kinds == 1 {
        let mut g = generic;
        for mana in Mana::ALL {
            let take = left[mana as usize].min(u32::from(g)) as u8;
            spent[mana as usize] += take;
            g -= take;
        }
        return Ways::One(spent);
    }
    Ways::Many
}

/// The pool units that would pay something owed, as `AssignMana` options, in the C# engine's order: colorless first,
/// then colors the cost does not name, then the colors it names. Ordinary mana a rule tells apart by its source (`told`,
/// each source with its mana, CR 106.3) is offered as that source's, after the ordinary mana of its kind no rule tells apart.
fn assign_options(pool: &ManaPool, restricted: &[(ManaRestriction, ManaPool)], told: &[(ObjectId, ManaPool)], owed: &ManaCost) -> Vec<ActionOption> {
    let named = |mana: Mana| {
        owed.symbols[mana as usize] > 0 || owed.hybrid.iter().flatten().any(|(a, b)| *a == mana || *b == mana)
    };
    let order = [Mana::C, Mana::W, Mana::U, Mana::B, Mana::R, Mana::G];
    let mut out = Vec::new();
    // Restricted mana first, then the rest.
    for (restriction, pool) in restricted {
        for mana in order {
            if pool.get(mana) > 0 && pays(owed, mana) {
                let mut option = ActionOption::simple(ActionKind::AssignMana);
                option.mana = Some(mana);
                option.restriction = Some(*restriction);
                out.push(option);
            }
        }
    }
    for pass in 0..3 {
        for mana in order {
            let kind = if mana == Mana::C { 0 } else if named(mana) { 2 } else { 1 };
            if kind != pass || pool.get(mana) == 0 || !pays(owed, mana) {
                continue;
            }
            let apart: u32 = told.iter().map(|(_, p)| p.get(mana)).sum();
            if pool.get(mana) > apart {
                let mut option = ActionOption::simple(ActionKind::AssignMana);
                option.mana = Some(mana);
                out.push(option);
            }
            for &(source, from) in told {
                if from.get(mana) > 0 {
                    let mut option = ActionOption::simple(ActionKind::AssignMana);
                    option.mana = Some(mana);
                    option.source = Some(source);
                    out.push(option);
                }
            }
        }
    }
    out
}

/// The ordinary mana in `player`'s pool that each of the `told` sources produced (CR 106.3).
fn told_pools(cx: &Cx, player: PlayerId, told: &[ObjectId]) -> Vec<(ObjectId, ManaPool)> {
    let mut out: Vec<(ObjectId, ManaPool)> = told.iter().map(|&s| (s, ManaPool::default())).collect();
    for e in cx.s().mana_from_of(player) {
        if e.restriction.is_none()
            && let Some(at) = told.iter().position(|&s| s == e.source)
        {
            for m in 0..6 {
                out[at].1 .0[m] += e.pool.0[m];
            }
        }
    }
    out
}

/// Records what a payment for `on` spent, one `Event::ManaSpent` for each source it took mana from, in the order it first
/// took from each (CR 106.3, 601.2h).
fn record_spent(cx: &mut Cx, player: PlayerId, on: ObjectId, cast: bool, taken: &[(ObjectId, Mana, u8)]) {
    let mut by: Vec<(ObjectId, ManaTally)> = Vec::new();
    for &(source, mana, n) in taken {
        let at = match by.iter().position(|(s, _)| *s == source) {
            Some(at) => at,
            None => {
                by.push((source, ManaTally::default()));
                by.len() - 1
            }
        };
        by[at].1.add(mana, u32::from(n));
    }
    for (source, mana) in by {
        cx.record(Event::ManaSpent { player, source, on, cast, mana });
    }
}

/// Activating an ability of `source`, which has these card types and subtypes as it is activated (CR 602.2b, 605.3a), as
/// far as restricted mana is concerned: and whether it is a permanent, for "abilities of [type]s" (CR 109.2), or a card in
/// another zone, which only "abilities of [type] sources" reads (CR 109.2c).
fn ability_payment(cx: &Cx, source: ObjectId, types: TypeSet, subtypes: SubtypeSet) -> ManaPayment {
    ManaPayment::Ability { source: types, subtypes, on_battlefield: cx.s().obj(source).zone.is(ZoneKind::Battlefield) }
}

/// What paying for `source` is, as far as restricted mana is concerned (CR 106.6): casting it, when it is a spell on the
/// stack being cast; activating one of its abilities; or anything else. Only casting a spell is limited to spells (CR
/// 601.2g): paying "unless its controller pays" (CR 118.12) or a copy's cost for the spell on the stack is not casting it;
/// and only an ability's activation is activating one (CR 602.2b, 605.3a), never a special action (CR 116.2). An
/// ability's source is judged as it was activated (`Paying::Activating`), however its costs have moved it since (an ability
/// of a land card in the hand is an ability of a land source).
fn payment_for(cx: &Cx, player: PlayerId, source: ObjectId, purpose: Paying) -> ManaPayment {
    match purpose {
        Paying::Casting if cx.s().obj(source).zone.is(ZoneKind::Stack) => {
            // The subtypes, only where some of the player's mana is limited by them.
            spell_payment(cx, source, cx.s().restricted_of(player).any(|(r, _)| r.names_subtypes()))
        }
        Paying::Activating(payment) => payment,
        _ => ManaPayment::Other,
    }
}

/// What paying for `source` is, as a "spend mana as though it were mana of any color" rule asks (CR 609.4b), as `payment_for`
/// judges it: casting a spell only while it is a spell on the stack being cast.
fn spending_for(cx: &Cx, source: ObjectId, purpose: Paying) -> SpendingFor {
    match purpose {
        Paying::Casting if cx.s().obj(source).zone.is(ZoneKind::Stack) => SpendingFor::Casting,
        Paying::Activating(_) => SpendingFor::Activating,
        _ => SpendingFor::Other,
    }
}

/// Casting `spell`, a spell on the stack, as it is there (CR 601.2g): its card types, supertypes, mana value with X as
/// announced (CR 202.3e: 0 for a copy with no mana cost or of a back face, CR 707.9b, 202.3b, and for a face-down spell,
/// CR 708.4; the cast half's alone, CR 709.3b), whether its mana cost has {X} and the zone it was cast from (CR 601.2a); its
/// subtypes only `with_subtypes`, where some mana at hand is limited by them.
fn spell_payment(cx: &Cx, spell: ObjectId, with_subtypes: bool) -> ManaPayment {
    let chars = cx.chars(spell);
    let subtypes = if with_subtypes { chars.subtypes } else { SubtypeSet::NONE };
    ManaPayment::Spell {
        types: chars.types,
        subtypes,
        legendary: chars.supertypes.legendary,
        mana_value: cx.mana_value(spell),
        x: cx.mana_cost(spell).x > 0,
        from: cx.s().obj(spell).cast_from.unwrap_or(ZoneKind::Stack),
    }
}

/// The restriction a mana ability of `source` puts on the mana it adds (CR 106.6): a subtype its text names is read
/// through the source's words (CR 612: Artificial Evolution makes Maelstrom of the Spirit Dragon's mana for Elf spells),
/// and "the chosen type" is the creature type chosen for it as it entered (CR 607.2a, 614.12a), or none if none was.
pub fn restriction_from(cx: &Cx, source: ObjectId, restriction: ManaRestriction, by: Option<Linked>) -> ManaRestriction {
    let ManaRestriction::Subtypes(mut s) = restriction else { return restriction };
    if s.chosen {
        s.subtypes = [cx.s().chosen_subtype(by.unwrap_or_else(|| cx.text_linked(source))), None];
        s.chosen = false;
    } else {
        let words = cantrip_api::words(cx.s(), source);
        s.subtypes = s.subtypes.map(|t| t.map(|t| words.subtype(t)));
    }
    ManaRestriction::Subtypes(s)
}

/// The restricted mana `player` has that may pay for `source`, with its restriction.
fn restricted_for(cx: &Cx, player: PlayerId, source: ObjectId, purpose: Paying) -> Vec<(ManaRestriction, ManaPool)> {
    if !cx.s().has_restricted_mana(player) {
        return Vec::new();
    }
    let payment = payment_for(cx, player, source, purpose);
    cx.s().restricted_of(player).filter(|(restriction, _)| restriction.may_pay(&payment)).collect()
}

/// The pool a cost for `source` may be paid from: unrestricted mana and the restricted mana that may pay for it.
fn spendable(cx: &Cx, player: PlayerId, source: ObjectId, purpose: Paying) -> ManaPool {
    let mut pool = cx.s().player(player).pool;
    for (_, restricted) in restricted_for(cx, player, source, purpose) {
        for m in Mana::ALL {
            pool.0[m as usize] += restricted.0[m as usize];
        }
    }
    pool
}

/// Takes `spent` out of the pool, restricted mana first (it is what the C# engine prefers spending), each unit's source
/// pushed onto `taken`. What the mana taken does to what it pays for (`ManaRestriction::effects`).
fn take_mana(cx: &mut Cx, player: PlayerId, source: ObjectId, purpose: Paying, spent: &[u8; 6], taken: &mut Vec<(ObjectId, Mana, u8)>) -> u8 {
    let restricted = restricted_for(cx, player, source, purpose);
    let mut used = 0u8;
    for mana in Mana::ALL {
        let mut need = spent[mana as usize];
        for (restriction, _) in &restricted {
            let take = cx.st().spend_mana(player, Some(*restriction), mana, need, cantrip_state::ManaPick::Any, taken);
            if take > 0 {
                used |= restriction.effects();
            }
            need -= take;
        }
        if need > 0 {
            cx.st().spend_mana(player, None, mana, need, cantrip_state::ManaPick::Any, taken);
        }
    }
    used
}

/// The mana abilities `player` may activate while paying for `source` (CR 601.2g, 605.3a), each whose mana is restricted
/// so that none of it may pay this cost marked as paying nothing (CR 106.6).
fn mana_options_while_paying(cx: &Cx, player: PlayerId, source: ObjectId, purpose: Paying) -> Vec<ActionOption> {
    let mut options = Vec::new();
    mana_ability_options(cx, player, ManaWindow::Payment, &mut options);
    if options.iter().any(|o| o.restriction.is_some()) {
        let payment = payment_for(cx, player, source, purpose);
        for option in &mut options {
            option.pays_nothing = option.restriction.is_some_and(|r| !r.may_pay(&payment));
        }
    }
    options
}

/// What besides mana may pay a spell's total cost (CR 601.2f-h): convoke (CR 702.51a), delve (CR 702.66a), improvise
/// (CR 702.126a), and whether mana may be spent on it at all ("You can't spend mana to cast this spell", CR 601.2h).
#[derive(Clone, Copy)]
pub(crate) struct Helpers<'a> {
    pub with: PayWith,
    /// Waterbend (CR 701.67a-b): how many of the generic mana an untapped artifact or creature tapped may pay {1} of.
    pub waterbend: u8,
    /// An untapped creature tapped pays {1} of the generic mana (Heirloom Epic).
    pub creatures: bool,
    pub no_mana: bool,
    /// What the mana pays for (CR 106.6): casting `source`, a spell (CR 601.2g), activating one of its abilities (CR
    /// 602.2b, 605.3a), or anything else.
    pub purpose: Paying,
    /// Cards another part of the cost was already chosen to exile (escape's), which delve may not also exile.
    pub reserved: &'a [ObjectId],
    /// The attacking creatures whose cost to attack this pays (CR 508.1h-j), for the question to name.
    pub attackers: &'a [ObjectId],
    /// The life the total cost asks besides its mana (CR 119.4, 601.2h), paid with it once the mana is: for the question to
    /// say.
    pub life: u32,
    /// "Mana of any type can be spent to cast that spell" (CR 609.4b): any mana pays any of its symbols.
    pub any_type: bool,
    /// The time for activating mana abilities is over (CR 601.2g): the payment of a spell with assist, whose players each had
    /// a window of their own before anything was paid (CR 702.132a).
    pub abilities_closed: bool,
    /// Which step of a payment with assist this is (CR 702.132a), for the question to say.
    pub assist: AssistStep,
}

impl Helpers<'static> {
    pub const NONE: Helpers<'static> =
        Helpers { with: PayWith::NONE, waterbend: 0, creatures: false, no_mana: false, purpose: Paying::Other, reserved: &[], attackers: &[], life: 0, any_type: false, abilities_closed: false, assist: AssistStep::None };
}

/// What a mana payment is for, as far as restricted mana is concerned (CR 106.6): see `payment_for`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Paying {
    Casting,
    /// Activating an ability, its source as it was when the activation began (CR 602.2a, 113.7a).
    Activating(ManaPayment),
    Other,
}

impl Helpers<'_> {
    /// Whether anything besides mana may pay.
    pub fn any(&self) -> bool {
        self.with.any() || self.waterbend > 0 || self.creatures
    }
}

/// Pays a mana cost (CR 601.2g-h) with nothing but mana, for anything but casting a spell or activating an ability.
pub fn pay_mana(cx: &mut Cx, player: PlayerId, source: ObjectId, total: ManaCost) -> R<ManaPool> {
    pay_mana_with(cx, player, source, total, &Helpers::NONE)
}

/// The pips of a cost a creature tapped for convoke may pay (CR 702.51a): a colored symbol of a color it has, or either
/// color of a hybrid symbol it has one of. Colorless is not a color, so never {C} (CR 105.1).
fn convoke_pips(cost: &ManaCost) -> Vec<[Option<Mana>; 2]> {
    let mut pips = Vec::new();
    for mana in Mana::ALL {
        for _ in 0..cost.symbols[mana as usize] {
            pips.push([Some(mana), None]);
        }
    }
    for (a, b) in cost.hybrid.iter().flatten() {
        pips.push([Some(*a), Some(*b)]);
    }
    pips
}

/// Which creature (by index) pays each pip when as many pips as possible are paid by creatures of their colors.
fn convoke_match(pips: &[[Option<Mana>; 2]], creatures: &[ColorSet]) -> Vec<Option<usize>> {
    fn accepts(pip: &[Option<Mana>; 2], colors: ColorSet) -> bool {
        pip.iter().flatten().any(|m| m.color().is_some_and(|c| colors.has(c)))
    }
    fn assign(p: usize, pips: &[[Option<Mana>; 2]], creatures: &[ColorSet], paid_by: &mut [Option<usize>], seen: &mut [bool]) -> bool {
        for c in 0..creatures.len() {
            if seen[c] || !accepts(&pips[p], creatures[c]) {
                continue;
            }
            seen[c] = true;
            let holder = paid_by.iter().position(|&x| x == Some(c));
            if holder.is_none_or(|q| assign(q, pips, creatures, paid_by, seen)) {
                paid_by[p] = Some(c);
                return true;
            }
        }
        false
    }
    let mut paid_by = vec![None; pips.len()];
    for p in 0..pips.len() {
        let mut seen = vec![false; creatures.len()];
        assign(p, pips, creatures, &mut paid_by, &mut seen);
    }
    paid_by
}

/// What is left of a cost once creatures with these colors have been tapped for convoke (CR 702.51a): each pays a
/// symbol of a color it has where one can, and {1} otherwise, which leaves the most for mana to pay.
fn convoked(cost: &ManaCost, creatures: &[ColorSet]) -> ManaCost {
    if creatures.is_empty() {
        return *cost;
    }
    let pips = convoke_pips(cost);
    let paid_by = convoke_match(&pips, creatures);
    let matched = paid_by.iter().flatten().count();
    let mut left = *cost;
    left.generic = left.generic.saturating_sub(u8::try_from(creatures.len() - matched).unwrap_or(u8::MAX));
    // The pips come in the cost's order, colored symbols first and hybrid symbols after.
    let mut at = 0;
    for mana in Mana::ALL {
        let mut kept = 0;
        for _ in 0..cost.symbols[mana as usize] {
            if paid_by[at].is_none() {
                kept += 1;
            }
            at += 1;
        }
        left.symbols[mana as usize] = kept;
    }
    for slot in 0..4 {
        if cost.hybrid[slot].is_some() {
            if paid_by[at].is_some() {
                left.hybrid[slot] = None;
            }
            at += 1;
        }
    }
    left
}

/// Whether creatures with these colors could all be tapped for convoke towards a cost: each paying a different symbol
/// of a color it has, and the rest no more than the generic mana.
fn can_convoke(cost: &ManaCost, creatures: &[ColorSet]) -> bool {
    let pips = convoke_pips(cost);
    let matched = convoke_match(&pips, creatures).iter().flatten().count();
    creatures.len() - matched <= cost.generic as usize
}

/// The permanents and cards that could pay something of `due` instead of mana right now (CR 601.2h), in the order the
/// C# engine offers them: delve's graveyard cards, then improvise's artifacts, then convoke's creatures; each once.
fn pay_candidates(
    cx: &Cx,
    player: PlayerId,
    source: ObjectId,
    helpers: &Helpers,
    cost: &ManaCost,
    due: &ManaCost,
    tapped: &[ColorSet],
    delved: &[ObjectId],
    bent: u8,
) -> Vec<ActionOption> {
    let mut out: Vec<ActionOption> = Vec::new();
    if helpers.with.delve && due.generic > 0 {
        for &id in cx.s().zone(Zone::graveyard(player)) {
            if id != source && !helpers.reserved.contains(&id) && !delved.contains(&id) {
                out.push(ActionOption::on(ActionKind::ExileToPay, id, PAY_DELVE));
            }
        }
    }
    if helpers.with.improvise || helpers.with.convoke || helpers.waterbend > 0 || helpers.creatures {
        for id in cx.permanents_of(player) {
            if cx.s().obj(id).tapped || out.iter().any(|o| o.source == Some(id)) {
                continue;
            }
            let chars = cx.chars(id);
            if helpers.waterbend > bent && due.generic > 0 && (chars.is(CardType::Artifact) || chars.is(CardType::Creature)) {
                out.push(ActionOption::on(ActionKind::TapToPay, id, PAY_WATERBEND));
            } else if helpers.creatures && due.generic > 0 && chars.is(CardType::Creature) {
                out.push(ActionOption::on(ActionKind::TapToPay, id, PAY_WATERBEND));
            } else if helpers.with.improvise && due.generic > 0 && chars.is(CardType::Artifact) {
                out.push(ActionOption::on(ActionKind::TapToPay, id, PAY_IMPROVISE));
            } else if helpers.with.convoke && chars.is(CardType::Creature) {
                let mut with = tapped.to_vec();
                with.push(chars.colors);
                if can_convoke(cost, &with) {
                    out.push(ActionOption::on(ActionKind::TapToPay, id, PAY_CONVOKE));
                }
            }
        }
    }
    out
}

/// Pays a mana cost (CR 601.2g-h): silently when the pool pays it in exactly one way and nothing else could pay any of
/// it, otherwise by asking, with the player's mana abilities, the pool's mana and whatever else may pay (convoke's
/// creatures, delve's cards, improvise's artifacts) offered, until nothing is owed. Once one of those has paid, no
/// mana ability is offered any more: the cost is being paid, which comes after every mana ability (CR 601.2g-h).
pub(crate) fn pay_mana_with(cx: &mut Cx, player: PlayerId, source: ObjectId, total: ManaCost, helpers: &Helpers) -> R<ManaPool> {
    Ok(pay_mana_used(cx, player, source, total, helpers)?.0)
}

/// `pay_mana_with`, and what the mana spent does to what it paid for (`ManaRestriction::effects`, CR 106.6).
pub(crate) fn pay_mana_used(cx: &mut Cx, player: PlayerId, source: ObjectId, mut total: ManaCost, helpers: &Helpers) -> R<(ManaPool, u8)> {
    // A Phyrexian symbol not yet announced is announced now, each paid with its color or 2 life (CR 107.4f), before any mana
    // ability is activated to pay it, and its life is paid with the mana. A cost to attack is the one that gets here so
    // (CR 508.1h-j: the total is determined, then mana abilities are activated, then all of it is paid): a spell's or an
    // ability's symbols are announced as it is proposed (CR 118.13a, 601.2b), and a cost paid as one resolves announces its
    // own (`pay_announced`, CR 118.13b). A payment that is neither for a cast nor for an activation is what reaches here
    // with one, so no ban on paying life to cast or activate (`payable_life_for`) applies to it.
    let life = if total.phyrexian != [0; 5] { announce_life(cx, player, source, &mut total, helpers.any_type, None)? } else { 0 };
    // A monocolored hybrid symbol not yet announced (a cost changed after the announcement) is announced now, as the cost
    // is paid (CR 118.13b).
    if total.twobrid != [0; 5] {
        announce_twobrids(cx, player, source, &mut total, helpers.any_type)?;
    }
    // CR 609.4b: a player who may spend mana as though it were mana of any color for this cost pays each colored symbol with
    // any mana, and one who may spend mana of any type on it every symbol; the cost is not changed, nor is what was spent.
    // What is owed keeps the cost's symbols, since what pays instead of mana pays only some of them (delve and improvise
    // generic mana, convoke a symbol of its creature's color, CR 702.66a, 702.126a, 702.51a); the mana is asked for as the
    // payment's view of what is left.
    let any = if helpers.any_type {
        AnyMana::Type
    } else if static_rules::spend_any_color_for(cx, player, source, spending_for(cx, source, helpers.purpose)) {
        AnyMana::Color
    } else {
        AnyMana::No
    };
    let mut owed = total;
    let mut paid = ManaPool::default();
    let mut used = 0u8;
    // What has paid instead of mana: creatures tapped for convoke (by their colors), and cards exiled or artifacts
    // tapped for {1} each.
    let mut convoking: Vec<ColorSet> = Vec::new();
    let mut generic_paid: u8 = 0;
    // The cards delve exiles: one payment (CR 702.66a, 601.2h), exiled together when the cost is paid, so that a
    // "one or more cards are put into exile" ability triggers once (CR 603.2c).
    let mut delved: Vec<ObjectId> = Vec::new();
    // How many of the generic mana tapping has paid for as waterbend (CR 701.67b), against what waterbend may pay.
    let mut bent: u8 = 0;
    // The mana spent, unit by unit, with the source of each (CR 106.3).
    let mut taken: Vec<(ObjectId, Mana, u8)> = Vec::new();
    loop {
        let mut cost = owed;
        cost.generic = cost.generic.saturating_sub(generic_paid);
        let left = convoked(&cost, &convoking);
        if left.is_zero() {
            break;
        }
        let due = any.view(&left);
        let pool = if helpers.no_mana { ManaPool::default() } else { spendable(cx, player, source, helpers.purpose) };
        let candidates =
            if helpers.any() { pay_candidates(cx, player, source, helpers, &cost, &left, &convoking, &delved, bent) } else { Vec::new() };
        let mut decided = ways(&pool, &due);
        // Ordinary mana a rule tells apart by its source (CR 106.3), by the source that produced it.
        let told = if helpers.no_mana { Vec::new() } else { static_rules::mana_told_apart(cx, cx.cards(), player) };
        let told_mana = told_pools(cx, player, &told);
        // Ordinary mana and restricted mana of a kind are different units (CR 106.6): spending only some of that kind
        // is the player's choice, even when the pool pays the cost one way. So is spending only some of the mana of a kind
        // that came from sources a rule tells apart (CR 106.3, 601.2h).
        if let Ways::One(spent) = decided
            && !told.is_empty()
        {
            let plain = cx.s().player(player).pool;
            let split = Mana::ALL.iter().any(|&m| {
                let apart: u32 = told_mana.iter().map(|(_, p)| p.get(m)).sum();
                let groups = told_mana.iter().filter(|(_, p)| p.get(m) > 0).count() + usize::from(plain.get(m) > apart);
                groups > 1 && spent[m as usize] > 0 && u32::from(spent[m as usize]) < pool.get(m)
            });
            if split {
                decided = Ways::Many;
            }
        }
        if let Ways::One(spent) = decided {
            let player_state = cx.s().player(player);
            let restricted = restricted_for(cx, player, source, helpers.purpose);
            let mixed = Mana::ALL.iter().any(|&m| {
                let held_restricted: u32 = restricted.iter().map(|(_, p)| p.get(m)).sum();
                held_restricted > 0
                    && player_state.pool.get(m) > 0
                    && spent[m as usize] > 0
                    && u32::from(spent[m as usize]) < player_state.pool.get(m) + held_restricted
            });
            if mixed {
                decided = Ways::Many;
            }
        }
        let spend_pool = |cx: &mut Cx, paid: &mut ManaPool, used: &mut u8, taken: &mut Vec<(ObjectId, Mana, u8)>, spent: [u8; 6]| {
            *used |= take_mana(cx, player, source, helpers.purpose, &spent, taken);
            for mana in Mana::ALL {
                paid.0[mana as usize] += u32::from(spent[mana as usize]);
            }
        };
        if let Ways::One(spent) = decided
            && candidates.is_empty()
        {
            spend_pool(cx, &mut paid, &mut used, &mut taken, spent);
            break;
        }
        let paying = !convoking.is_empty() || generic_paid > 0 || helpers.abilities_closed;
        let mut options = if paying { Vec::new() } else { mana_options_while_paying(cx, player, source, helpers.purpose) };
        if matches!(decided, Ways::One(_)) {
            // The pool's one way of paying, which the player may take or pay some other way instead.
            options.push(ActionOption::simple(ActionKind::ConfirmCasting));
        } else if !helpers.no_mana {
            let restricted = restricted_for(cx, player, source, helpers.purpose);
            options.extend(assign_options(&cx.s().player(player).pool, &restricted, &told_mana, &due));
        }
        options.extend(candidates);
        if helpers.assist != AssistStep::HelperPays {
            options.push(ActionOption::simple(ActionKind::CancelCasting));
        }
        let shown = if helpers.any() {
            let mut t = total;
            t.generic = t.generic.saturating_sub(generic_paid);
            convoked(&t, &convoking)
        } else {
            total
        };
        let ask = RequestKind::PayCosts { source, total: shown, remaining: due, options, announcing: false, x: 0, most: 0, least: 0, loyalty: false, attackers: helpers.attackers.to_vec(), life: helpers.life + u32::from(life), governs: "", assist: helpers.assist };
        // The player assisting pays for a spell that is not theirs to take back (CR 702.132a).
        let answer = cx.ask_core(player, ask, helpers.assist != AssistStep::HelperPays)?;
        match answer {
            Answer::Act(action) => match action.kind {
                ActionKind::ActivateAbility => {
                    let from = action.source.expect("source");
                    activate_mana(cx, player, from, action.index, action.color, action.mana == Some(Mana::C), &action.targets)?;
                }
                ActionKind::AssignMana => {
                    let mana = action.mana.expect("mana");
                    match action.restriction {
                        // Only mana of a restriction that may pay for this (CR 106.6): `validate` holds the answer
                        // to the options, which offer no other.
                        Some(r) => {
                            used |= r.effects();
                            cx.st().spend_mana(player, Some(r), mana, 1, cantrip_state::ManaPick::Any, &mut taken);
                        }
                        // Mana of the source the option names, or of none a rule tells apart (CR 106.3).
                        None => {
                            let pick = match action.source {
                                Some(from) => cantrip_state::ManaPick::From(from),
                                None => cantrip_state::ManaPick::Except(&told),
                            };
                            cx.st().spend_mana(player, None, mana, 1, pick, &mut taken);
                        }
                    }
                    paid.0[mana as usize] += 1;
                    any.spend(&mut owed, mana);
                }
                ActionKind::ConfirmCasting => {
                    let Ways::One(spent) = decided else { return Err(Stop::Cancel) };
                    spend_pool(cx, &mut paid, &mut used, &mut taken, spent);
                    break;
                }
                ActionKind::TapToPay => {
                    let id = action.source.expect("source");
                    let colors = cx.chars(id).colors;
                    ops::tap(cx, id);
                    if action.index == PAY_CONVOKE {
                        convoking.push(colors);
                        // CR 702.51c: it convoked the spell, for "creatures that convoked it" to ask the history.
                        cx.record(Event::Convoked { creature: id, spell: source });
                    } else {
                        generic_paid += 1;
                        if action.index == PAY_WATERBEND {
                            bent = bent.saturating_add(1);
                        }
                    }
                }
                ActionKind::ExileToPay => {
                    let id = action.source.expect("source");
                    delved.push(id);
                    generic_paid += 1;
                }
                _ => return Err(Stop::Cancel),
            },
            _ => return Err(Stop::Cancel),
        }
    }
    // The life its Phyrexian symbols take, paid with the mana (CR 508.1j: all costs, no partial payment): a mana ability
    // that cost life while paying (Mana Confluence) may have left too little, which gives the whole payment up.
    if !rest_payable(cx, player, 0, life, None) {
        return Err(Stop::Cancel);
    }
    if life > 0 {
        ops::change_life(cx, player, -life_paid(life));
    }
    if !taken.is_empty() {
        let cast = helpers.purpose == Paying::Casting && cx.s().obj(source).zone.is(ZoneKind::Stack);
        record_spent(cx, player, source, cast, &taken);
    }
    if !delved.is_empty() {
        ops::exile_all(cx, &delved);
    }
    Ok((paid, used))
}

/// Life being paid, as the change to a life total: never more than the payer has (CR 119.4, judged before it is paid), so
/// it is a life total's width.
fn life_paid(life: u32) -> i32 {
    i32::try_from(life).unwrap_or(i32::MAX)
}

/// Whether `player` could pay `life` life now (CR 119.4): any amount of 0 (CR 119.4b), otherwise only with a life total
/// at least that high, and not while they can't lose life or their life total can't change (CR 119.8).
pub fn can_pay_life(cx: &Cx, player: PlayerId, life: u32) -> bool {
    // CR 119.4b: 0 life can always be paid.
    life == 0 || i64::from(payable_life(cx, player)) >= i64::from(life)
}

/// Whether `player` could pay `life` life now for a cost paid for `paid` (CR 119.4): as `can_pay_life` says, and any amount
/// but 0 not at all where a static ability says life can't be paid for it (`payable_life_for`).
pub fn can_pay_life_for(cx: &Cx, player: PlayerId, life: u32, paid: Option<CostFor>) -> bool {
    // CR 119.4b: 0 life can always be paid, "even if an effect says players can't pay life".
    life == 0 || i64::from(payable_life_for(cx, player, paid)) >= i64::from(life)
}

/// The most life `player` may pay for a cost paid for `paid` (CR 119.4): `payable_life`, or none at all to cast a spell or
/// activate an ability while a static ability says life can't be paid for it (CR 101.2, `CardDef::cost_bans`: Yasharn,
/// Implacable Earth; Karn's Sylex, for abilities that aren't mana abilities). `None` is a payment for anything else - a cost
/// paid as a spell or ability resolves (Yasharn's ruling: "Other things may still cause players to pay life"), a special
/// action's - which they leave alone.
pub fn payable_life_for(cx: &Cx, player: PlayerId, paid: Option<CostFor>) -> i32 {
    if paid.is_some_and(|f| static_rules::cost_banned(cx, cx.cards(), player, f, CostPayment::Life)) {
        0
    } else {
        payable_life(cx, player)
    }
}

/// Whether `player` may sacrifice `id` to pay a cost paid for `paid` (CR 701.21a, 118.3): only a permanent they control ("a
/// player can't sacrifice ... a permanent they don't control"); not a permanent that can't be sacrificed at all (CR 101.2, `static_rules::cant_be_sacrificed`: Hithlain Rope), whatever the cost is for; not to cast
/// a spell or activate an ability while a static ability says it can't be sacrificed for that (`CardDef::cost_bans`:
/// Yasharn, Implacable Earth's nonland permanents, Angel of Jubilation's creatures), which leave a cost paid for anything
/// else (`None`) alone; and, for a cost a resolving spell or ability has the player pay ("unless [they] sacrifice", "you
/// may sacrifice ... If you do", CR 118.12), not when that spell or ability can't cause them to sacrifice (CR 101.2,
/// Tamiyo, Collector of Tales's ruling: "something happens unless you sacrifice" can't be paid).
fn may_sacrifice_for(cx: &Cx, player: PlayerId, id: ObjectId, paid: Option<CostFor>) -> bool {
    cx.s().obj(id).controller == player
        && !static_rules::cant_be_sacrificed(cx, cx.cards(), id)
        && match paid {
            Some(f) => !static_rules::cost_banned(cx, cx.cards(), player, f, CostPayment::Sacrifice(id)),
            None => resolution_may_cause(cx, player, CauseKind::Sacrifice),
        }
}

/// Whether the spell or ability resolving now, if any, may cause `player` to do `kind` (CR 101.2, `cantrip_api::may_cause`):
/// what a cost it has them pay may take (CR 118.12). Nothing is resolving for a special action's cost, which it leaves alone.
fn resolution_may_cause(cx: &Cx, player: PlayerId, kind: CauseKind) -> bool {
    cx.effect_controller().is_none_or(|by| cantrip_api::may_cause(cx, by, player, kind))
}

/// The most life `player` may pay (CR 119.4): their life total, or none at all while they can't lose life (CR 119.8: "a cost
/// that involves having that player pay life can't be paid") or their life total can't change (Teferi's Protection's ruling:
/// "You can't pay a cost that includes the payment of any amount of life other than 0 life").
pub fn payable_life(cx: &Cx, player: PlayerId) -> i32 {
    if static_rules::cant_lose_life(cx, cx.cards(), player) || ops::life_total_cant_change(cx.s(), player) {
        0
    } else {
        cx.s().player(player).life
    }
}

/// A cost paid as a spell or ability resolves (CR 118.12, 608.2d): "[payer] may pay [cost]. If they do", and "unless
/// [payer] pays [cost]" (CR 118.12a), which means the same. It is one payment of the whole cost, its mana and its life
/// together, with whatever else it has - permanents, cards, counters, energy - chosen and paid as an activated ability's
/// are. The payer may activate mana abilities while asked, when the cost has mana in it (CR 605.3a, 118.3c); paying is
/// offered once the pool covers the mana, the life total covers the life (CR 118.3) and every other part can be paid, and
/// the life is judged as it is paid (CR 119.4): a player without that much life is not asked at all, and one whose mana
/// ability takes it below that (Mana Confluence's life, a painland's damage) can no longer pay, and is offered only
/// declining. An {X} the text does not define is chosen by the payer as they pay (CR 107.3f), as `may_pay_x` chooses it.
/// Whether it was paid.
pub fn may_pay(cx: &mut Cx, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: &Cost, question: &'static Question) -> R<bool> {
    let x = (cost.mana.x > 0).then_some(0);
    Ok(pay_during_resolution(cx, payer, source, subject, cost, question, x, None)?.is_some())
}

/// `may_pay` for a cost whose variable part's size the payer sets as they pay it (`VariableCost::DiscardOneOrMore`): how many
/// it took, or `None` declined.
pub fn may_pay_counted(cx: &mut Cx, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: &Cost, question: &'static Question) -> R<Option<u16>> {
    pay_during_resolution(cx, payer, source, subject, cost, question, None, None)
}

/// The most X a payer may raise "you may pay {X}" to while choosing it: past what any game could produce, a bound for a
/// player who raises without paying.
const MOST_X_PAID: u16 = 1000;

/// "You may pay {X}" as a spell or ability resolves (CR 118.12, 608.2d), X chosen by the payer as they pay it, since the
/// text does not define it (CR 107.3f): one question (`RequestKind::UnlessPays` with its `x`) raises and lowers X, makes mana
/// (CR 605.3a), and pays {X} once the pool covers it, or declines. Any X may be chosen, 0 included (paying {0} is still
/// paying, CR 118.5). The X paid, or `None` when the payer declined: "when you do" (CR 603.12) asks whether it was paid, and
/// "X" afterwards is the X paid. `cost` is the cost with its {X} (and any other mana or life beside it).
pub fn may_pay_x(cx: &mut Cx, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: &Cost, question: &'static Question) -> R<Option<u16>> {
    pay_during_resolution(cx, payer, source, subject, cost, question, Some(0), None)
}

/// "You may pay [mana] up to `most` times" (`ANY_NUMBER_OF_TIMES`: "any number of times") as a spell or ability resolves
/// (CR 118.12, 608.2d): one payment, its number chosen by the payer as they pay it (CR 603.12a: "a choice to pay a cost
/// multiple times"), in one question that raises and lowers it from 1 to `most`, makes mana (CR 605.3a), and pays the
/// mana that many times over once the pool covers it, or declines. How many times it was paid, 0 when the payer declined:
/// "when you pay this cost one or more times" triggers once for any number above 0, given that number (CR 603.12a,
/// `Game::reflexive_trigger_counted`). A number whose cost the mana cost's shape can't hold (`ManaCost::times`) is not
/// offered.
pub fn may_pay_times(cx: &mut Cx, payer: PlayerId, source: ObjectId, subject: ObjectId, mana: ManaCost, most: u16, question: &'static Question) -> R<u16> {
    if most == 0 {
        return Ok(0);
    }
    let cost = Cost { mana, ..Cost::NONE };
    Ok(pay_during_resolution(cx, payer, source, subject, &cost, question, Some(1), Some(most))?.unwrap_or(0))
}

/// The payment `may_pay`, `may_pay_x` and `may_pay_times` ask: with `x` the X chosen so far for a cost with an {X}, else
/// none (a cost without one has an X of 0). With `times` (`Some(most)`), `x` is instead how many times the cost's mana is
/// paid, 1 to `most`, and the mana paid is that many times over (`ManaCost::times`). What was paid: the X or the number of
/// times (0 for a cost with neither), or `None` declined.
///
/// Every part of the cost is paid as a cost is (CR 118.3, 118.12): mana abilities may be activated while the payer is asked
/// (CR 605.3a), then the payer chooses what the other parts take - the permanents, cards and counters, tapped permanents
/// judged once the mana is made (CR 601.2g-h) - with the same choices a spell's or an ability's cost asks, and the whole is
/// paid at once. Paying is offered only while every part can be paid now. A choice the payer gives up, or a mana ability
/// whose activation is taken back, is undone and the payer asked again (CR 733.1).
fn pay_during_resolution(
    cx: &mut Cx,
    payer: PlayerId,
    source: ObjectId,
    subject: ObjectId,
    cost: &Cost,
    question: &'static Question,
    mut x: Option<u16>,
    times: Option<u16>,
) -> R<Option<u16>> {
    // A part no payment during a resolution can have - a spell's or an activated ability's own - can't be paid, and
    // nothing is asked.
    if !resolution_parts(cost) {
        return Ok(None);
    }
    // "Pay X life" (CR 119.4) is paid with the X chosen; without an {X} to choose it, X is 0.
    let life_of = |x: Option<u16>| if cost.pay_x_life { cost.life + u32::from(x.unwrap_or(0)) } else { cost.life };
    if !can_pay_life(cx, payer, life_of(x)) {
        return Ok(None);
    }
    // CR 118.13b: how a monocolored hybrid symbol is paid is chosen immediately before the cost is paid. A cost paid a number
    // of times is announced as it is paid, that many times over (`pay_mana`). A Phyrexian symbol is announced as the payer
    // pays (`pay_announced`): a spell's mana cost another player pays (Ice Cave) may have one.
    let announced;
    let cost = if cost.mana.twobrid != [0; 5] && times.is_none() {
        let mut mana = cost.mana;
        announce_twobrids(cx, payer, source, &mut mana, false)?;
        announced = Cost { mana, ..*cost };
        &announced
    } else {
        cost
    };
    // The cost paid the number of times chosen so far, changed only to a number the cost's shape holds that many times over
    // (`ManaCost::times`); once, it is the cost itself.
    let mut times_total = cost.mana;
    loop {
        // The cost as X now stands: each {X} is X generic mana (CR 107.3f).
        let total = match x {
            Some(_) if times.is_some() => times_total,
            Some(n) => {
                let generic = u32::from(cost.mana.generic) + u32::from(n) * u32::from(cost.mana.x);
                ManaCost { generic: generic.min(255) as u8, x: 0, ..cost.mana }
            }
            None => cost.mana,
        };
        let life = life_of(x);
        // Once a mana ability has cost the payer the life the payment needs, or what another part takes, there is nothing
        // left to do but decline - or, where X sets the life, lower X.
        let mut options = Vec::new();
        let pool = spendable(cx, payer, source, Paying::Other);
        // Mana spent as though it were mana of any color where a rule lets the payer for this payment (CR 609.4b), as the
        // payment itself asks (`pay_mana_used`, `spending_for`): no spell is cast and no ability activated by it.
        let any_color = static_rules::spend_any_color_for(cx, payer, source, SpendingFor::Other);
        let remaining = uncovered(&pool, &if any_color { total.paid_with_any_color() } else { total });
        let life_payable = can_pay_life(cx, payer, life);
        let payable = life_payable && resolution_rest_payable(cx, payer, source, cost, x.unwrap_or(0));
        if payable {
            if total != ManaCost::ZERO || x.is_some() {
                options = mana_options_while_paying(cx, payer, source, Paying::Other);
            }
            if total == ManaCost::ZERO || pool_pays(&pool, &total, payable_life(cx, payer) - life_paid(life), any_color) {
                options.push(ActionOption::simple(ActionKind::PayCost));
            }
        }
        if let (Some(n), Some(most)) = (x, times) {
            // The number of times: from 1 to `most`, and no more than the cost's shape holds that many times over.
            if n < most && cost.mana.times(u32::from(n) + 1).is_some() {
                options.push(ActionOption::simple(ActionKind::IncreaseX));
            }
            if n > 1 {
                options.push(ActionOption::simple(ActionKind::DecreaseX));
            }
        } else if let Some(n) = x
            && (payable || cost.pay_x_life || cost.variable.is_some())
        {
            // The generic mana a cost holds is a byte: X goes no higher than that leaves room for.
            if n < MOST_X_PAID && u32::from(total.generic) + u32::from(cost.mana.x) <= 255 {
                options.push(ActionOption::simple(ActionKind::IncreaseX));
            }
            if n > 0 {
                options.push(ActionOption::simple(ActionKind::DecreaseX));
            }
        }
        options.push(ActionOption::simple(ActionKind::DeclineCost));
        let ask = RequestKind::UnlessPays { source, subject, total, remaining, life, life_payable, question, options, x, times };
        let answer = cx.ask_core(payer, ask, false)?;
        let Answer::Act(action) = answer else { return Ok(None) };
        match action.kind {
            ActionKind::ActivateAbility => {
                let from = action.source.expect("source");
                // A mana ability whose activation is taken back (its cost found unpayable once its mana is paid) is undone,
                // and the payer asked again.
                let (mark, log_mark) = (cx.s().mark(), cx.g.log.len());
                match activate_mana(cx, payer, from, action.index, action.color, action.mana == Some(Mana::C), &action.targets) {
                    Ok(()) => {}
                    Err(Stop::Cancel) => {
                        cx.st().revert(mark);
                        cx.g.log.truncate(log_mark);
                    }
                    Err(stop) => return Err(stop),
                }
            }
            // A number of times moves only to one the cost's shape holds (the options offer no other).
            ActionKind::IncreaseX | ActionKind::DecreaseX if times.is_some() => {
                let n = x.unwrap_or(1);
                let to = if action.kind == ActionKind::IncreaseX { n.saturating_add(1) } else { n.saturating_sub(1) };
                if let Some(t) = cost.mana.times(u32::from(to)).filter(|_| to >= 1) {
                    x = Some(to);
                    times_total = t;
                }
            }
            ActionKind::IncreaseX => x = x.map(|n| n + 1),
            ActionKind::DecreaseX => x = x.map(|n| n.saturating_sub(1)),
            ActionKind::PayCost => {
                let (mark, log_mark) = (cx.s().mark(), cx.g.log.len());
                match pay_announced(cx, payer, source, cost, total, life, x.unwrap_or(0)) {
                    // Without an X, what was paid is how many the variable part took (`may_pay_counted`), if it has one.
                    Ok(taken) => return Ok(Some(x.unwrap_or(taken))),
                    // A choice given up: nothing was paid, and the payer is asked again.
                    Err(Stop::Cancel) => {
                        cx.st().revert(mark);
                        cx.g.log.truncate(log_mark);
                    }
                    Err(stop) => return Err(stop),
                }
            }
            _ => return Ok(None),
        }
    }
}

/// Whether `pool` pays `owed` now, however its symbols that can be paid in more than one way are paid (CR 118.13): each
/// Phyrexian symbol with its color or 2 life, at most `life_room` life between them (CR 107.4f), and each monocolored hybrid
/// symbol with its color or {2} (CR 107.4e). Symbols of one kind and color are alike, so each choice is how many of them
/// are paid each way. `any_color`: the payer may spend mana as though it were mana of any color for it (CR 609.4b), so once
/// those symbols are announced each colored one is paid with any mana (`ManaCost::paid_with_any_color`), as `pay_mana_used`
/// pays it.
pub(crate) fn pool_pays(pool: &ManaPool, owed: &ManaCost, life_room: i32, any_color: bool) -> bool {
    fn go(pool: &ManaPool, rest: ManaCost, life_room: i32, any_color: bool, at: usize) -> bool {
        if at == 10 {
            return !matches!(ways(pool, &if any_color { rest.paid_with_any_color() } else { rest }), Ways::None);
        }
        let (color, phyrexian) = (at % 5, at < 5);
        let n = if phyrexian { rest.phyrexian[color] } else { rest.twobrid[color] };
        let mana = Color::ALL[color].mana() as usize;
        // `other` of them paid the other way: with 2 life, or with {2}.
        (0..=n).any(|other| {
            let mut next = rest;
            let Some(symbols) = next.symbols[mana].checked_add(n - other) else { return false };
            next.symbols[mana] = symbols;
            let room = if phyrexian {
                next.phyrexian[color] = 0;
                life_room - 2 * i32::from(other)
            } else {
                next.twobrid[color] = 0;
                let Some(generic) = other.checked_mul(2).and_then(|two| next.generic.checked_add(two)) else { return false };
                next.generic = generic;
                life_room
            };
            room >= 0 && go(pool, next, room, any_color, at + 1)
        })
    }
    go(pool, *owed, life_room, any_color, 0)
}

/// Pays a resolution's cost with `total` its mana and `life` its life: its Phyrexian symbols are announced first, each
/// paid with its color or 2 life, immediately before the cost is paid (CR 118.13b, 107.4f), and the life they take is paid
/// with the rest. A choice of more life than the payer can pay gives the payment up, to be asked again.
fn pay_announced(cx: &mut Cx, payer: PlayerId, source: ObjectId, cost: &Cost, mut total: ManaCost, mut life: u32, x: u16) -> R<u16> {
    if total.phyrexian != [0; 5] {
        life += announce_life(cx, payer, source, &mut total, false, None)?;
        if !rest_payable(cx, payer, 0, life, None) {
            return Err(Stop::Cancel);
        }
    }
    pay_resolution_cost(cx, payer, source, cost, total, life, x)
}

/// Chooses what the parts of a resolution's cost other than mana and life take, then pays the whole (CR 118.3, 601.2h):
/// the mana from the pool, the life, and each other part as an activated ability's cost pays it (`activate`). `x` is the
/// X paid, for a "sacrifice X" part. How many the variable part took ("discard one or more"), 0 for none.
fn pay_resolution_cost(cx: &mut Cx, payer: PlayerId, source: ObjectId, cost: &Cost, total: ManaCost, life: u32, x: u16) -> R<u16> {
    // A cost paid as a spell or ability resolves is paid for no cast or activation (CR 118.12): nothing that forbids paying
    // for those touches it (Yasharn, Implacable Earth's ruling).
    let chosen = choose_permanents(cx, payer, source, cost, None, None)?;
    let discards = choose_discards(cx, payer, source, cost, None)?;
    let exiles = choose_exiles(cx, payer, source, cost)?;
    let hand_exiles = choose_hand_exiles(cx, payer, source, cost, None)?;
    let (sacrifices, variable_exiles, variable_discards) = choose_variable(cx, payer, source, cost, u32::from(x), None)?;
    let taken = u16::try_from(sacrifices.len() + variable_exiles.len() + variable_discards.cards.len()).unwrap_or(u16::MAX);
    let tappers = choose_tappers(cx, payer, source, cost)?;
    let counter = choose_counter_kind(cx, payer, source, cost)?;
    let beheld = choose_behold(cx, payer, source, cost)?;
    let blighted = choose_blight(cx, payer, source, cost)?;
    let removed = choose_counters_among(cx, payer, source, cost)?;
    if total != ManaCost::ZERO {
        pay_mana(cx, payer, source, total)?;
    }
    if life > 0 {
        ops::change_life(cx, payer, -life_paid(life));
    }
    // {T} first, so a permanent the cost also moves was tapped as it left (CR 118.3), as `activate` pays it.
    if cost.tap {
        ops::tap(cx, source);
    }
    pay_blight(cx, payer, cost, blighted);
    pay_counters_among(cx, &removed);
    pay_permanents(cx, payer, cost, &chosen, None);
    // Revealed for the effect paying it, as part of that effect (CR 701.20a): no reveal of the effect's own lasts past the
    // instruction it is for (`ops::seen_by_all`).
    pay_behold(cx, payer, &beheld);
    pay_discards(cx, &discards);
    pay_discards(cx, &variable_discards);
    pay_exiles(cx, &exiles);
    pay_exiles(cx, &hand_exiles);
    ops::sacrifice(cx, payer, &sacrifices);
    pay_exiles(cx, &variable_exiles);
    // The life is paid above, with any X of it.
    pay_life_and_counters(cx, payer, source, &Cost { life: 0, ..*cost });
    if let Some(kind) = counter {
        counters::remove(cx, source, kind, 1);
    }
    pay_exert_and_tappers(cx, source, cost, &tappers);
    if cost.exile_self {
        ops::move_to_for(cx, source, Zone::exile(cx.s().obj(source).owner), ZoneReason::Exile);
    }
    if cost.discard_self {
        ops::discard(cx, source);
    }
    if cost.sacrifice_self {
        ops::sacrifice(cx, payer, &[source]);
    }
    if cost.return_self {
        ops::move_to_for(cx, source, Zone::hand(cx.s().obj(source).owner), ZoneReason::Other);
    }
    Ok(taken)
}

/// Whether a cost has only parts a payment during a resolution can have (CR 118.12). A loyalty cost is a loyalty
/// ability's (CR 606.4); "sacrifice the permanent that gives this ability" a granted ability's (CR 201.5a); emerge,
/// tapping a creature for a spell's reduction and craft's materials are a spell's or an activation's own (CR 702.119a,
/// 702.180a, 702.167a); exerting is done as a creature attacks (CR 701.43a). None of these is a cost a resolving spell or
/// ability asks for, so one is never paid. Every field is named, so a new kind of cost part is looked at here.
fn resolution_parts(cost: &Cost) -> bool {
    let Cost {
        mana: _,
        life: _,
        energy: _,
        tap: _,
        sacrifice_self: _,
        sacrifice_granter,
        discard_self: _,
        permanents: _,
        loyalty,
        remove_counters: _,
        put_counters: _,
        remove_any_counter: _,
        remove_counters_among: _,
        exile_self: _,
        discard: _,
        pay_x_life: _,
        exile_from_graveyard: _,
        exile_from_graveyard_of: _,
        exert,
        tap_creatures: _,
        emerge,
        exile_from_hand: _,
        exile_from_hand_for_less: _,
        tap_permanents: _,
        unattach_self: _,
        return_self: _,
        variable: _,
        discard_hand: _,
        collect_evidence: _,
        behold: _,
        blight: _,
        tap_to_reduce,
        materials,
    } = *cost;
    !sacrifice_granter && loyalty.is_none() && !exert && !emerge && !tap_to_reduce && materials.is_none()
}

/// Whether the parts of a resolution's cost other than mana and life could be paid now (CR 118.3), `x` the X chosen: the
/// parts that name the source ("tap this", "sacrifice this", "discard this card") only while it is where they need it and
/// its payer's, and the rest as an activated ability's cost is judged (`can_pay_activation`). {T} here is no activated
/// ability's, so summoning sickness (CR 302.6) has no say.
fn resolution_rest_payable(cx: &Cx, payer: PlayerId, source: ObjectId, cost: &Cost, x: u16) -> bool {
    let obj = cx.s().obj(source);
    let live = live(cx.s(), source);
    let yours_on_battlefield = live && obj.zone.is(ZoneKind::Battlefield) && obj.controller == payer;
    let in_your = |zone: Zone| live && obj.zone == zone;
    if (cost.tap && !(yours_on_battlefield && !obj.tapped))
        || ((cost.sacrifice_self || cost.return_self || cost.unattach_self) && !yours_on_battlefield)
        || (cost.discard_self && !in_your(Zone::hand(payer)))
        || (cost.exile_self && !(yours_on_battlefield || in_your(Zone::hand(payer)) || in_your(Zone::graveyard(payer))))
        || ((cost.remove_counters.is_some() || cost.remove_any_counter || cost.put_counters.is_some()) && !live)
    {
        return false;
    }
    // A discard the resolving spell or ability can't cause the payer to make can't pay (CR 101.2, 118.3; Tamiyo, Collector of
    // Tales's ruling), as a sacrifice can't (`may_sacrifice_for`). Discarding an empty hand discards nothing.
    let discards = cost.discard_self
        || !cost.discard.is_none()
        || (cost.discard_hand && !cx.s().zone(Zone::hand(payer)).is_empty())
        || matches!(cost.variable, Some(VariableCost::DiscardOneOrMore(_)));
    if discards && !resolution_may_cause(cx, payer, CauseKind::Discard) {
        return false;
    }
    // "Sacrifice X" takes X of them (CR 107.3f).
    if let Some(variable @ (VariableCost::SacrificeX(_) | VariableCost::SacrificeAnyNumber { .. })) = cost.variable
        && variable_candidates(cx, source, payer, variable, None, None).len() < usize::from(x)
    {
        return false;
    }
    can_pay_activation(cx, source, payer, &Cost { tap: false, life: 0, mana: ManaCost::ZERO, ..*cost }, None, None)
}

/// Whether an object is somewhere it can be found by its id: still the same object.
#[allow(dead_code)]
fn is_live(cx: &Cx, id: ObjectId) -> bool {
    live(cx.s(), id)
}

#[cfg(test)]
#[path = "priority_only_tests.rs"]
mod priority_only_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Game;

    const ME: PlayerId = PlayerId(0);
    const THEM: PlayerId = PlayerId(1);

    /// "You may cast Dragon spells without paying their mana costs" (Dracogenesis): spells "you" cast, judged on the face.
    fn dragon_spells_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, player: PlayerId) -> bool {
        player == asker.controller && q.chars(spell).has_subtype(Subtype::Dragon)
    }
    /// "You may pay {W}{U}{B}{R}{G} rather than pay the mana cost for spells that you cast" (Jodah, Archmage Eternal).
    fn spells_you_cast(_: &dyn Query, asker: &Asker, _: ObjectId, player: PlayerId) -> bool {
        player == asker.controller
    }
    fn any_card(_: &dyn Query, _: &Asker, _: ObjectId, _: PlayerId) -> bool {
        true
    }
    fn any_spell(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
        true
    }

    static GENESIS: CardDef = CardDef {
        name: "Alternative Test Genesis",
        types: TypeSet::of(&[CardType::Enchantment]),
        spell_alternatives: &[SpellAlternative { text: "Without paying its mana cost", applies: dragon_spells_you_cast, cost: InsteadOfManaCost::WithoutPaying, flash: false }],
        ..CardDef::BLANK
    };
    static ARCHMAGE: CardDef = CardDef {
        name: "Alternative Test Archmage",
        types: TypeSet::of(&[CardType::Enchantment]),
        spell_alternatives: &[SpellAlternative { text: "Pay {W}{U}{B}{R}{G} rather than pay its mana cost", applies: spells_you_cast, cost: InsteadOfManaCost::Mana(mana("{W}{U}{B}{R}{G}")), flash: false }],
        ..CardDef::BLANK
    };
    /// "You may cast spells from among cards in exile" (a permanent's permission, CR 601.3).
    static EXILER: CardDef = CardDef {
        name: "Alternative Test Exiler",
        types: TypeSet::of(&[CardType::Enchantment]),
        exile_casts: &[ExileCast { text: "Cast from exile", allows: any_card, additional: Cost::NONE, any_type: false, lands: false, life_for_mana_value: false }],
        ..CardDef::BLANK
    };
    /// "You may cast spells from the top of your library" (CR 601.3).
    static FORGE: CardDef = CardDef {
        name: "Alternative Test Forge",
        types: TypeSet::of(&[CardType::Artifact]),
        play_rules: &[PlayRule::LibraryTopSpells(any_spell)],
        ..CardDef::BLANK
    };
    static DRAGON: CardDef = CardDef {
        name: "Alternative Test Dragon",
        types: TypeSet::of(&[CardType::Creature]),
        subtypes: SubtypeSet::of(&[Subtype::Dragon]),
        cost: mana("{6}{R}{R}"),
        has_cost: true,
        power: Some(5),
        toughness: Some(5),
        ..CardDef::BLANK
    };
    /// A Dragon with no mana cost (CR 118.6a, 202.1b).
    static COSTLESS: CardDef = CardDef {
        name: "Alternative Test Costless Dragon",
        types: TypeSet::of(&[CardType::Creature]),
        subtypes: SubtypeSet::of(&[Subtype::Dragon]),
        has_cost: false,
        power: Some(4),
        toughness: Some(4),
        ..CardDef::BLANK
    };
    static BEAR: CardDef = CardDef {
        name: "Alternative Test Bear",
        types: TypeSet::of(&[CardType::Creature]),
        cost: mana("{1}{G}"),
        has_cost: true,
        power: Some(2),
        toughness: Some(2),
        ..CardDef::BLANK
    };

    /// A game in the first player's precombat main phase, the stack empty, with each of `cards` in the zone beside it and
    /// owned by the player beside it, nothing else anywhere and no mana: what each is, in order.
    fn table(cards: &[(&'static CardDef, Zone, PlayerId)]) -> (Game, Vec<ObjectId>) {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&GENESIS, &ARCHMAGE, &EXILER, &FORGE, &DRAGON, &COSTLESS, &BEAR])));
        let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        game.state.update_turn(|t| {
            t.number = 2;
            t.active = ME;
            t.priority = ME;
            t.set_step(Step::PrecombatMain);
        });
        let ids = cards
            .iter()
            .map(|&(def, zone, owner)| {
                let id = game.state.push_obj(Obj::new(table.id_of(def), owner, zone));
                game.state.zone_push(zone, id);
                id
            })
            .collect();
        (game, ids)
    }

    fn cx(game: &mut Game) -> Cx<'_> {
        Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
    }

    static PREPARE_SPELL: CardDef = CardDef {
        name: "Preparation Test Spell",
        types: TypeSet::of(&[CardType::Instant]),
        has_cost: true,
        ..CardDef::BLANK
    };
    static PREPARATION: CardDef = CardDef { name: "Preparation Test Creature", prepare: Some(&PREPARE_SPELL), ..BEAR };

    fn preparation() -> (Game, ObjectId, ObjectId) {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&PREPARATION, &PREPARE_SPELL, &BEAR])));
        let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        game.state.update_turn(|t| {
            t.number = 2;
            t.set_step(Step::PrecombatMain);
        });
        let creature = game.state.push_obj(Obj::new(table.id_of(&PREPARATION), ME, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, creature);
        cantrip_api::Game::become_prepared(&mut cx(&mut game), creature);
        let copy = game.state.zone(Zone::exile(ME))[0];
        (game, creature, copy)
    }

    /// CR 722.3b/c, 704.5e: becoming unprepared ends only this permanent's copy's permission and exemption, without
    /// performing state-based actions in the middle of an effect: its link to the permanent is severed, so a new
    /// preparation cannot revive the expired copy. Both changes to the object rows undo and redo together.
    #[test]
    fn becoming_unprepared_expires_the_old_copy_even_if_the_permanent_prepares_again_before_state_based_actions() {
        let (mut game, creature, copy) = preparation();
        let other = game.state.push_obj(Obj::new(game.cards.id_of(&PREPARATION), THEM, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, other);
        cantrip_api::Game::become_prepared(&mut cx(&mut game), other);
        let other_copy = game.state.zone(Zone::exile(THEM))[0];
        assert_eq!(casts(&mut game, ME, copy), [(0, None)]);
        let mark = game.state.mark();
        cantrip_api::Game::become_unprepared(&mut cx(&mut game), creature);
        assert!(!game.state.obj(creature).prepared && game.state.obj(copy).prepared_from.is_none());
        assert_eq!(game.state.zone(Zone::exile(ME)), &[copy]);
        assert!(casts(&mut game, ME, copy).is_empty());
        let (undos, redos) = game.state.step_back(mark);
        assert!(game.state.obj(creature).prepared && game.state.obj(copy).prepared_from == Some(creature));
        game.state.step_forward(undos, redos);
        assert!(!game.state.obj(creature).prepared && game.state.obj(copy).prepared_from.is_none());
        let mark = game.state.mark();
        cantrip_api::Game::become_unprepared(&mut cx(&mut game), creature);
        assert_eq!(game.state.mark(), mark);
        cantrip_api::Game::become_prepared(&mut cx(&mut game), creature);
        let new_copy = game.state.zone(Zone::exile(ME))[1];
        assert!(casts(&mut game, ME, copy).is_empty());
        assert_eq!(casts(&mut game, ME, new_copy), [(0, None)]);
        crate::settle::state_based_actions_alone(&mut cx(&mut game)).expect("no choices");
        assert_eq!(game.state.zone(Zone::exile(ME)), &[new_copy]);
        assert!(!live(&game.state, copy));
        assert!(game.state.obj(other).prepared && game.state.obj(other_copy).prepared_from == Some(other));
        assert_eq!(game.state.zone(Zone::exile(THEM)), &[other_copy]);
    }

    /// CR 722.3b: removing the designation does not ask whether the permanent still has a prepare spell or abilities.
    #[test]
    fn becoming_unprepared_works_after_a_copy_effect_removes_the_prepare_spell() {
        let (mut game, creature, copy) = preparation();
        let bear = game.cards.id_of(&BEAR);
        game.state.update_obj(creature, |o| o.copy = Some(Copiable::of(bear)));
        cantrip_api::Game::add_effect(&mut cx(&mut game), creature, creature, EffectKind::LoseAllAbilities, Duration::Indefinite);
        assert_eq!(cx(&mut game).chars(creature).card, bear);
        assert!(cx(&mut game).lost_all_abilities(creature));
        cantrip_api::Game::become_unprepared(&mut cx(&mut game), creature);
        assert!(!game.state.obj(creature).prepared && game.state.obj(copy).prepared_from.is_none());
        crate::settle::state_based_actions_alone(&mut cx(&mut game)).expect("no choices");
        assert!(game.state.zone(Zone::exile(ME)).is_empty());
    }

    /// CR 400.7, 702.26b: neither a stale reference nor an object treated as absent can be made unprepared.
    #[test]
    fn becoming_unprepared_ignores_a_previous_object_and_a_phased_out_permanent() {
        let (mut game, creature, _) = preparation();
        let held = ops::return_to_hand(&mut cx(&mut game), &[creature])[0];
        let returned = ops::put_onto_battlefield(&mut cx(&mut game), &[(held, ME, false)], ZoneReason::Other).expect("no choices")[0];
        cantrip_api::Game::become_prepared(&mut cx(&mut game), returned);
        let mark = game.state.mark();
        cantrip_api::Game::become_unprepared(&mut cx(&mut game), creature);
        cantrip_api::Game::become_unprepared(&mut cx(&mut game), held);
        assert_eq!(game.state.mark(), mark);
        assert!(game.state.obj(returned).prepared);
        crate::phasing::phase_out(&mut cx(&mut game), &[returned], None, false);
        let mark = game.state.mark();
        cantrip_api::Game::become_unprepared(&mut cx(&mut game), returned);
        assert_eq!(game.state.mark(), mark);
        assert!(game.state.obj(returned).prepared);
    }

    fn cost_while_prepared(q: &dyn Query, _: &Asker, _: &[Target]) -> Cost {
        assert!(q.state().battlefield().iter().any(|&id| q.obj(id).prepared), "its permanent is still prepared during CR 601.2f");
        Cost::NONE
    }
    static COST_CHECKED_PREPARE: CardDef = CardDef { name: "Preparation Cost Test Spell", cost_of_targets: Some(cost_while_prepared), ..PREPARE_SPELL };
    static COST_CHECKED_CREATURE: CardDef = CardDef { name: "Preparation Cost Test Creature", prepare: Some(&COST_CHECKED_PREPARE), ..BEAR };

    /// CR 722.3c, 601.2i: the designation stays through cost determination and payment, and is gone by the cast event.
    /// The cast copy remains a spell on the stack when state-based actions check the now-unprepared permanent.
    #[test]
    fn casting_a_prepare_copy_unprepares_its_permanent_only_after_paying_its_costs() {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&COST_CHECKED_CREATURE, &COST_CHECKED_PREPARE])));
        let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
        let creature = game.state.push_obj(Obj::new(table.id_of(&COST_CHECKED_CREATURE), ME, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, creature);
        let mut cx = cx(&mut game);
        cantrip_api::Game::become_prepared(&mut cx, creature);
        let copy = cx.s().zone(Zone::exile(ME))[0];
        cast(&mut cx, ME, copy, 0, &[], CastTerms::default(), CastBy::default(), false).expect("cast");
        assert!(!cx.s().obj(creature).prepared);
        assert_eq!(cx.s().stack.len(), 1);
        let spell = cx.s().stack[0].object;
        crate::settle::state_based_actions_alone(&mut cx).expect("no choices");
        assert!(live(cx.s(), spell) && cx.s().obj(spell).zone.is(ZoneKind::Stack));
    }

    /// The casts `player` is offered of `card` now: each one's way number, and the permanent whose alternative cost it pays.
    fn casts(game: &mut Game, player: PlayerId, card: ObjectId) -> Vec<(u8, Option<ObjectId>)> {
        let mut cx = cx(game);
        priority_options(&mut cx, player)
            .into_iter()
            .filter(|o| o.kind == ActionKind::CastSpell && o.source == Some(card))
            .map(|o| (o.index, o.paid_by.map(|g| g.object)))
            .collect()
    }

    /// The cast of `card` for the first alternative cost `by` gives (`ActionOption::paid_by`).
    fn paying(game: &Game, card: ObjectId, by: ObjectId) -> ActionOption {
        let grant = Grant { object: by, ability: AbilityRef::spell_alternative(game.state.obj(by).card, 0) };
        ActionOption { paid_by: Some(grant), ..ActionOption::on(ActionKind::CastSpell, card, 0) }
    }

    /// What a cast for an alternative cost of no mana is made by.
    fn for_nothing() -> CastBy {
        CastBy { alternative: Some(Cost::NONE), ..CastBy::default() }
    }

    /// A permission an effect gave.
    fn permission(card: ObjectId, grant: PlayGrant) -> PlayPermission {
        PlayPermission { card, player: ME, grant, by: Grant { object: card, ability: AbilityRef::default() }, turn: 2 }
    }

    /// CR 118.9, 601.2b: a Dragon in hand that no mana could pay for is offered without paying its mana cost, and only that;
    /// a spell that is no Dragon is offered nothing (and its mana cost can't be paid either).
    #[test]
    fn an_alternative_cost_is_offered_for_a_spell_whose_mana_cost_cannot_be_paid() {
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::hand(ME), ME), (&BEAR, Zone::hand(ME), ME)]);
        assert_eq!(casts(&mut game, ME, ids[1]), vec![(0, Some(ids[0]))]);
        assert!(casts(&mut game, ME, ids[2]).is_empty());
        let option = paying(&game, ids[1], ids[0]);
        let cx = cx(&mut game);
        assert_eq!(describe(cx.cards(), cx.s(), &cx.g.log, &option), "Without paying its mana cost");
    }

    /// CR 118.9: "spells you cast" is its controller's alone; the opponent's Dragon is offered nothing by it.
    #[test]
    fn an_alternative_cost_is_its_controllers_alone() {
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, THEM), (&DRAGON, Zone::hand(ME), ME)]);
        assert!(casts(&mut game, ME, ids[1]).is_empty());
    }

    /// Only the alternative costs that apply to the cast are offered: an opponent's four Genesis give the player nothing; and
    /// two of the player's give one choice, by the first, paying it once, not two.
    #[test]
    fn alternative_costs_are_numbered_among_those_that_apply_once_each() {
        let opponents = [(&GENESIS, Zone::BATTLEFIELD, THEM); 4];
        let mut cards = opponents.to_vec();
        cards.extend([(&GENESIS, Zone::BATTLEFIELD, ME), (&GENESIS, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::hand(ME), ME)]);
        let (mut game, ids) = table(&cards);
        assert_eq!(casts(&mut game, ME, ids[6]), vec![(0, Some(ids[4]))]);
        let option = paying(&game, ids[6], ids[4]);
        let mut cx = cx(&mut game);
        assert_eq!(describe(cx.cards(), cx.s(), &cx.g.log, &option), "Without paying its mana cost");
        cast(&mut cx, ME, ids[6], 0, &[], CastTerms::default(), for_nothing(), false).expect("cast");
        assert_eq!(cx.s().stack.len(), 1);
    }

    /// CR 601.3, 118.9: it gives no permission. A Dragon in a graveyard nothing lets be cast is offered nothing; one a
    /// permission lets be cast from there ("you may cast it") is offered for the alternative cost, beside nothing else, the
    /// mana cost being unpayable.
    #[test]
    fn an_alternative_cost_grants_no_permission_and_stands_beside_one() {
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::graveyard(ME), ME)]);
        assert!(casts(&mut game, ME, ids[1]).is_empty());
        game.state.permission_push(permission(ids[1], PlayGrant::CAST));
        assert_eq!(casts(&mut game, ME, ids[1]), vec![(0, Some(ids[0]))]);
    }

    /// CR 118.9a: a cast already for an alternative cost - a permission's "without paying its mana cost", flashback, life
    /// for its mana value - is not offered for another.
    #[test]
    fn an_alternative_cost_is_not_combined_with_another() {
        for grant in [
            PlayGrant { instead: Some(ManaCost::ZERO), ..PlayGrant::CAST },
            PlayGrant { flashback: true, ..PlayGrant::CAST },
            PlayGrant { life_for_mana_value: true, ..PlayGrant::CAST },
        ] {
            let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::graveyard(ME), ME)]);
            game.state.permission_push(permission(ids[1], grant));
            assert!(!casts(&mut game, ME, ids[1]).contains(&(0, Some(ids[0]))), "{grant:?}");
        }
    }

    /// CR 601.3, 604.1: a permanent's permission to cast cards in exile, and its rule to cast spells from the top of a
    /// library, are casts the alternative cost stands beside as well.
    #[test]
    fn an_alternative_cost_stands_beside_a_cast_from_exile_and_from_a_library() {
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&EXILER, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::exile(ME), ME)]);
        assert_eq!(casts(&mut game, ME, ids[2]), vec![(0, Some(ids[0]))]);
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::library(ME), ME)]);
        assert!(casts(&mut game, ME, ids[1]).is_empty());
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&FORGE, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::library(ME), ME)]);
        assert_eq!(casts(&mut game, ME, ids[2]), vec![(0, Some(ids[0]))]);
    }

    /// CR 118.9: a mana cost to pay rather than the mana cost is offered as its own cast once that mana could be paid,
    /// beside the ordinary cast, and each of two such costs is a cast of its own.
    #[test]
    fn a_mana_alternative_is_offered_when_its_mana_can_be_paid() {
        let (mut game, ids) = table(&[(&ARCHMAGE, Zone::BATTLEFIELD, ME), (&GENESIS, Zone::BATTLEFIELD, ME), (&BEAR, Zone::hand(ME), ME), (&DRAGON, Zone::hand(ME), ME)]);
        assert!(casts(&mut game, ME, ids[2]).is_empty());
        assert_eq!(casts(&mut game, ME, ids[3]), vec![(0, Some(ids[1]))]);
        game.state.update_player(ME, |p| p.pool = ManaPool([1, 1, 1, 1, 2, 0]));
        assert_eq!(casts(&mut game, ME, ids[2]), vec![(0, None), (0, Some(ids[0]))]);
        assert_eq!(casts(&mut game, ME, ids[3]), vec![(0, Some(ids[0])), (0, Some(ids[1]))]);
        let option = paying(&game, ids[2], ids[0]);
        let cx = cx(&mut game);
        assert_eq!(describe(cx.cards(), cx.s(), &cx.g.log, &option), "Pay {W}{U}{B}{R}{G} rather than pay its mana cost");
    }

    /// CR 118.9, 601.2f-h: cast for the alternative cost, the Dragon goes on the stack with nothing paid, from the graveyard a
    /// permission casts it from, and the permission is used up.
    #[test]
    fn a_spell_cast_for_an_alternative_cost_pays_it_rather_than_its_mana_cost() {
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::graveyard(ME), ME)]);
        game.state.permission_push(permission(ids[1], PlayGrant::CAST));
        let mut cx = cx(&mut game);
        cast(&mut cx, ME, ids[1], 0, &[], CastTerms::default(), for_nothing(), false).expect("cast");
        let s = cx.s();
        assert_eq!(s.stack.len(), 1);
        let spell = s.stack[0].object;
        assert_eq!(s.obj(spell).card, cx.cards().id_of(&DRAGON));
        assert_eq!(s.obj(spell).cast_from, Some(ZoneKind::Graveyard));
        assert_eq!(s.stack[0].way, 0);
        assert!(s.permissions.is_empty());
    }

    /// CR 608.2g, 601.2b, 118.9: an effect that has a card cast as it resolves, paying its costs ("you may cast that card"),
    /// lets it be cast for the alternative cost, which is announced as it is cast: its caster is asked whether to pay the mana
    /// cost or that, and the Dragon cast for that goes on the stack above the resolving ability with nothing paid.
    #[test]
    fn a_cast_made_as_an_effect_resolves_may_be_for_an_alternative_cost() {
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::exile(ME), ME)]);
        let genesis = game.cards.id_of(&GENESIS);
        game.state.stack_push(stack_item(StackKind::Triggered, ids[0], AbilityRef { card: genesis, index: 0 }, ME, &[]));
        let serial = game.state.stack.last().expect("the resolving ability").serial;
        game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
        let frame = game.state.agenda.len() - 1;
        let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        assert!(cast_during_resolution(&mut cx, ids[1], CastTerms::default()).is_err(), "the cost is asked for");
        let Some(Request { kind: RequestKind::ChooseModes { modes, .. }, .. }) = cx.s().agenda[frame].pending.clone() else { panic!("a choice of costs") };
        assert_eq!(modes, vec![Label::ManaCost, Label::AlternativeCost(genesis, 0)]);
        cx.st().update_frame(frame, |f| {
            f.pending = None;
            f.answers = vec![(None, Answer::Modes(vec![1]))];
        });
        let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        assert_eq!(cast_during_resolution(&mut cx, ids[1], CastTerms::default()).ok(), Some(true));
        let s = cx.s();
        assert_eq!(s.stack.len(), 2);
        assert_eq!(s.obj(s.stack[1].object).card, cx.cards().id_of(&DRAGON));
        // Cast without paying its mana cost by the effect itself, it is asked nothing.
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&DRAGON, Zone::exile(ME), ME)]);
        game.state.stack_push(stack_item(StackKind::Triggered, ids[0], AbilityRef { card: genesis, index: 0 }, ME, &[]));
        let serial = game.state.stack.last().expect("the resolving ability").serial;
        game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
        let frame = game.state.agenda.len() - 1;
        let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        let free = CastTerms { instead: Some(ManaCost::ZERO), ..CastTerms::default() };
        assert_eq!(cast_during_resolution(&mut cx, ids[1], free).ok(), Some(true));
    }

    /// CR 118.6a: a card with no mana cost cast as an effect resolves, paying its costs, is cast only for an alternative cost:
    /// its mana cost is not among the costs it is asked to choose from, and with no alternative it is not cast.
    #[test]
    fn a_card_with_no_mana_cost_is_cast_as_an_effect_resolves_only_for_an_alternative_cost() {
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, ME), (&COSTLESS, Zone::exile(ME), ME)]);
        let genesis = game.cards.id_of(&GENESIS);
        game.state.stack_push(stack_item(StackKind::Triggered, ids[0], AbilityRef { card: genesis, index: 0 }, ME, &[]));
        let serial = game.state.stack.last().expect("the resolving ability").serial;
        game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
        let frame = game.state.agenda.len() - 1;
        let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        assert!(cast_during_resolution(&mut cx, ids[1], CastTerms::default()).is_err(), "the cost is asked for");
        let Some(Request { kind: RequestKind::ChooseModes { modes, .. }, .. }) = cx.s().agenda[frame].pending.clone() else { panic!("a choice of costs") };
        assert_eq!(modes, vec![Label::AlternativeCost(genesis, 0)]);
        // Under an opponent's Genesis, nothing lets it be cast.
        let (mut game, ids) = table(&[(&GENESIS, Zone::BATTLEFIELD, THEM), (&COSTLESS, Zone::exile(ME), ME)]);
        game.state.stack_push(stack_item(StackKind::Triggered, ids[0], AbilityRef { card: genesis, index: 0 }, ME, &[]));
        let serial = game.state.stack.last().expect("the resolving ability").serial;
        game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
        let frame = game.state.agenda.len() - 1;
        let mut cx = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        assert_eq!(cast_during_resolution(&mut cx, ids[1], CastTerms::default()).ok(), Some(false));
        assert!(cx.s().stack.len() == 1);
    }

    #[test]
    fn a_generic_reduction_takes_only_generic_mana_off() {
        // CR 118.7a: {2} off {1}{W}{W} leaves {W}{W}, never touching the white.
        assert_eq!(reduce(mana("{1}{W}{W}"), &mana("{2}")), mana("{W}{W}"));
    }

    #[test]
    fn a_colored_reduction_takes_its_own_color_off_and_the_rest_from_generic() {
        // CR 702.193b, 118.7: {3}{W}{W} off {5}{W}{W} is {W}{W} off the white and {3} off the generic.
        assert_eq!(reduce(mana("{5}{W}{W}"), &mana("{3}{W}{W}")), mana("{2}"));
        // CR 118.7b: blue the cost does not have comes off its generic mana.
        assert_eq!(reduce(mana("{3}{R}"), &mana("{U}")), mana("{2}{R}"));
        // CR 118.7c: more white than the cost has; the excess comes off its generic mana.
        assert_eq!(reduce(mana("{2}{W}"), &mana("{W}{W}")), mana("{1}"));
        // CR 118.7d: colorless likewise.
        assert_eq!(reduce(mana("{2}{C}"), &mana("{C}{C}")), mana("{1}"));
        // CR 118.7f: a Phyrexian symbol takes off mana of its color.
        assert_eq!(reduce(mana("{1}{G}"), &mana("{G/P}")), mana("{1}"));
    }

    #[test]
    fn mana_of_any_type_pays_a_symbol_before_the_generic_mana() {
        // CR 609.4b: mana of any type pays {1}{U}{C} as {3}, and a unit pays its own symbol, then another, the generic
        // last, which is left for delve or improvise (CR 702.66a, 702.126a).
        let mut owed = mana("{1}{U}{C}");
        assert_eq!(AnyMana::Type.view(&owed), mana("{3}"));
        AnyMana::Type.spend(&mut owed, Mana::C);
        assert_eq!(owed, mana("{1}{U}"));
        AnyMana::Type.spend(&mut owed, Mana::R);
        assert_eq!(owed, mana("{1}"));
        // As though of any color, mana pays no {C} (Mycosynth Lattice's ruling): red pays the {U} and then the generic.
        let mut owed = mana("{1}{U}{C}");
        assert_eq!(AnyMana::Color.view(&owed), mana("{2}{C}"));
        AnyMana::Color.spend(&mut owed, Mana::R);
        AnyMana::Color.spend(&mut owed, Mana::R);
        assert_eq!(owed, mana("{C}"));
    }

    /// CR 118.7e: a reduction by hybrid mana is split by the payer, one half of each symbol: {2}{G/W} off {3}{G}{W} is
    /// {2}{G} or {2}{W} off, by the half chosen; {2/R} is {R} or {2} of generic mana.
    #[test]
    fn a_reduction_by_hybrid_mana_is_split_by_the_payer() {
        let by = mana("{2}{G/W}");
        assert_eq!(reduce(mana("{3}{G}{W}"), &split_reduction(&by, &[false])), mana("{1}{W}"));
        assert_eq!(reduce(mana("{3}{G}{W}"), &split_reduction(&by, &[true])), mana("{1}{G}"));
        assert_eq!(reduce(mana("{3}{R}"), &split_reduction(&mana("{2/R}"), &[false])), mana("{3}"));
        assert_eq!(reduce(mana("{3}{R}"), &split_reduction(&mana("{2/R}"), &[true])), mana("{1}{R}"));
        // Unsplit, the first half: never a panic.
        assert_eq!(reduce(mana("{3}{G}{W}"), &by), mana("{1}{W}"));
    }

    #[test]
    fn a_reduction_takes_nothing_below_zero() {
        // CR 601.2f: the mana is reduced to nothing, {0}, and no further.
        assert_eq!(reduce(mana("{1}{U}"), &mana("{3}{U}{U}")), ManaCost::ZERO);
    }

    #[test]
    fn a_least_total_is_made_up_with_generic_mana_and_keeps_the_colored_symbols() {
        // CR 601.2f, Trinisphere: {B} comes to {2}{B}, {1}{B} to {2}{B}, {0} to {3}.
        assert_eq!(at_least(mana("{B}"), 3), mana("{2}{B}"));
        assert_eq!(at_least(mana("{1}{B}"), 3), mana("{2}{B}"));
        assert_eq!(at_least(ManaCost::ZERO, 3), mana("{3}"));
        // A total of three or more is left as it is, and no least (0) changes nothing.
        assert_eq!(at_least(mana("{W}{U}{B}"), 3), mana("{W}{U}{B}"));
        assert_eq!(at_least(mana("{5}{R}"), 3), mana("{5}{R}"));
        assert_eq!(at_least(mana("{U}"), 0), mana("{U}"));
    }

    /// What `fixed` mana and the choosing sources `complex` could pay, as the advice counts it.
    fn pool(fixed: [u32; 6], complex: Vec<(Vec<Alternative>, u32)>) -> Available {
        Available {
            fixed,
            flex: [0; 64],
            unknown: false,
            changers: Vec::new(),
            grantors: Vec::new(),
            restricted: Vec::new(),
            any_color: false,
            spenders: Vec::new(),
            linked: Vec::new(),
            complex,
            spent: Vec::new(),
        }
    }
    fn adds(fixed: [u32; 6]) -> Alternative {
        Alternative { fixed, k: 0, types: 0, restriction: None, cost: ManaCost::ZERO, source: ManaPayment::Other, extra: [0; 6], produced: 0, tapped: false }
    }
    fn one(mana: Mana, n: u32) -> [u32; 6] {
        let mut out = [0; 6];
        out[mana as usize] = n;
        out
    }
    const CREATURE_SPELL: ManaPayment = ManaPayment::Spell {
        types: TypeSet::of(&[CardType::Creature]),
        subtypes: SubtypeSet::NONE,
        legendary: false,
        mana_value: 2,
        x: false,
        from: ZoneKind::Hand,
    };

    #[test]
    fn three_mana_of_one_color_is_never_three_colors() {
        // Black Lotus: "Add three mana of any one color" - {B}{B}{B}, never {W}{U}{B} (CR 106.1b).
        let lotus = Alternative { k: 3, types: ANY_COLOR, ..adds([0; 6]) };
        let alone = pool([0; 6], vec![(vec![lotus], 1)]);
        assert!(alone.covers(&mana("{B}{B}{B}")));
        assert!(alone.covers(&mana("{1}{U}{U}")));
        assert!(!alone.covers(&mana("{W}{U}{B}")));
        assert!(!alone.covers(&mana("{4}")));
        // A Mox Pearl beside it is the {W}; a second Lotus, two colors of three each.
        assert!(!pool(one(Mana::W, 1), vec![(vec![lotus], 1)]).covers(&mana("{W}{U}{B}")));
        assert!(pool(one(Mana::W, 1), vec![(vec![lotus], 1)]).covers(&mana("{W}{B}{B}{B}")));
        let two = pool([0; 6], vec![(vec![lotus], 2)]);
        assert!(two.covers(&mana("{U}{U}{U}{B}{B}{B}")));
        assert!(!two.covers(&mana("{W}{U}{B}")));
    }

    #[test]
    fn a_choice_of_abilities_makes_one_of_them() {
        // A land with "{T}: Add {C}" and "{T}: Add {U}{U}" (a Shivan Reef under Mana Flare for {C}{C} or {U}{U}): one of
        // the two, never {C} and {U} from the one tap.
        let land = vec![adds(one(Mana::C, 1)), adds(one(Mana::U, 2))];
        let alone = pool([0; 6], vec![(land.clone(), 1)]);
        assert!(alone.covers(&mana("{U}{U}")));
        assert!(alone.covers(&mana("{C}")));
        assert!(!alone.covers(&mana("{C}{U}")));
        assert!(!alone.covers(&mana("{3}")));
        let two = pool([0; 6], vec![(land, 2)]);
        assert!(two.covers(&mana("{C}{U}{U}")));
        assert!(!two.covers(&mana("{C}{C}{U}")));
    }

    #[test]
    fn a_restricted_and_an_unrestricted_ability_of_one_land_make_one_mana() {
        // Cavern of Souls: "{T}: Add {C}" or one mana of any color only for creature spells of the chosen type - one or the
        // other, so {1}{U} for such a creature is not paid by the Cavern alone.
        let creatures = ManaRestriction::Spells(TypeSet::of(&[CardType::Creature]));
        let cavern = vec![adds(one(Mana::C, 1)), Alternative { k: 1, types: ANY_COLOR, restriction: Some(creatures), ..adds([0; 6]) }];
        let alone = pool([0; 6], vec![(cavern.clone(), 1)]);
        assert!(alone.covers_as(&mana("{U}"), &CREATURE_SPELL));
        assert!(!alone.covers_as(&mana("{1}{U}"), &CREATURE_SPELL));
        assert!(!alone.covers(&mana("{U}")));
        assert!(alone.covers(&mana("{1}")));
        let island = pool(one(Mana::U, 1), vec![(cavern.clone(), 1)]);
        assert!(island.covers_as(&mana("{1}{U}"), &CREATURE_SPELL));
        assert!(island.covers_as(&mana("{U}{U}"), &CREATURE_SPELL));
        assert!(!island.covers(&mana("{U}{U}")));
        // Under Mana Flare, each tap adds another mana of the type it produced, unrestricted (CR 106.12a): the creature's
        // {U}{U} from the colored tap, and the colorless tap's {C}{C} pays {2} for anything.
        let flared: Vec<Alternative> = cavern.iter().map(|a| Alternative { produced: 1, ..*a }).collect();
        let flared = pool([0; 6], vec![(flared, 1)]);
        assert!(flared.covers_as(&mana("{U}{U}"), &CREATURE_SPELL));
        assert!(flared.covers(&mana("{2}")));
        assert!(flared.covers(&mana("{U}")));
        assert!(!flared.covers(&mana("{U}{U}")));
        assert!(!flared.covers_as(&mana("{2}{U}"), &CREATURE_SPELL));
    }

    #[test]
    fn an_ability_that_costs_mana_is_paid_with_mana_there_before_it() {
        // "{1}, {T}: Add {B}{B}" beside "{T}: Add {C}" - and a Swamp: {B}{B} (the Swamp pays the {1}), or the Swamp's {B}
        // and the {C}; never three from the two (CR 605.3a).
        let filter = vec![adds(one(Mana::C, 1)), Alternative { cost: mana("{1}"), ..adds(one(Mana::B, 2)) }];
        let with_swamp = pool(one(Mana::B, 1), vec![(filter.clone(), 1)]);
        assert!(with_swamp.covers(&mana("{B}{B}")));
        assert!(with_swamp.covers(&mana("{C}{B}")));
        assert!(!with_swamp.covers(&mana("{B}{B}{B}")));
        assert_eq!(with_swamp.most_unrestricted(), 2);
        // Alone, it can't pay its own {1}: only its {C}.
        let alone = pool([0; 6], vec![(filter, 1)]);
        assert!(alone.covers(&mana("{1}")));
        assert!(!alone.covers(&mana("{B}")));
        // A colored cost is paid with that color (Arena of Glory's "{R}, {T}, Exert: Add {R}{R}" beside "{T}: Add {R}"): an
        // Island can't pay it, a Mountain can.
        let arena = vec![adds(one(Mana::R, 1)), Alternative { cost: mana("{R}"), ..adds(one(Mana::R, 2)) }];
        assert!(!pool(one(Mana::U, 1), vec![(arena.clone(), 1)]).covers(&mana("{R}{R}")));
        assert!(pool(one(Mana::U, 1), vec![(arena.clone(), 1)]).covers(&mana("{U}{R}")));
        assert!(pool(one(Mana::R, 1), vec![(arena.clone(), 1)]).covers(&mana("{R}{R}")));
        assert!(!pool(one(Mana::R, 1), vec![(arena, 1)]).covers(&mana("{R}{R}{R}")));
        // Two Signet-like sources ("{1}, {T}: Add {W}{U}") and one land: the land pays the first, the first's {W} the
        // second's {1}, leaving {U}{W}{U} - never {W}{W}{U}{U}.
        let signet = vec![Alternative { cost: mana("{1}"), ..adds([1, 1, 0, 0, 0, 0]) }];
        let two = pool(one(Mana::C, 1), vec![(signet, 2)]);
        assert!(two.covers(&mana("{W}{U}{U}")));
        assert!(!two.covers(&mana("{W}{W}{U}{U}")));
        assert!(!two.covers(&mana("{4}")));
    }

    #[test]
    fn hybrid_symbols_are_matched_to_the_mana_that_pays_them_all() {
        // Leyline of the Guildpact's {G/W}{G/U}{B/G}{R/G} off a Forest, a Plains, an Island and a Mountain: the Forest must
        // pay the {B/G}, not the {G/W} it could pay first (CR 107.4e).
        let lands = pool([1, 1, 0, 1, 1, 0], Vec::new());
        assert!(lands.covers(&mana("{G/W}{G/U}{B/G}{R/G}")));
        assert!(!lands.covers(&mana("{G/W}{G/U}{B/G}{B/G}")));
        // A Black Lotus for three of one color beside them: still one choice.
        let lotus = Alternative { k: 3, types: ANY_COLOR, ..adds([0; 6]) };
        assert!(pool([1, 1, 0, 1, 1, 0], vec![(vec![lotus], 1)]).covers(&mana("{G/W}{G/U}{B/G}{R/G}{B}{B}{B}")));
        assert!(!pool([1, 1, 0, 1, 1, 0], vec![(vec![lotus], 1)]).covers(&mana("{G/W}{G/U}{B/G}{R/G}{B}{B}{W}{W}")));
    }

    /// CR 118.7c, 601.2b: a {U} reduction the cost's {U} symbols don't absorb meets a hybrid symbol with a blue half, which
    /// may be announced as blue - and then is reduced to nothing - or as its other half, and the reduction takes generic mana
    /// instead. The totals the cast could come to are each announcement's: Expansion's {U/R}{U/R} under two is free; with
    /// {U/R} and {U/B} under one, either may be the one taken off.
    #[test]
    fn a_colored_reduction_meets_a_hybrid_symbol_announced_as_that_color() {
        let less = |n: u8| ManaCost { symbols: [0, n, 0, 0, 0, 0], ..ManaCost::ZERO };
        let totals = |cost: &str, n: u8| {
            let mut out: Vec<String> = announcements(&cantrip_api::mana(cost), &less(n)).into_iter().map(|c| reduce(c, &less(n)).to_string()).collect();
            out.sort();
            out.dedup();
            out
        };
        assert_eq!(totals("{U/R}{U/R}", 2), ["{0}", "{U/R}", "{U/R}{U/R}"]);
        assert_eq!(totals("{1}{U/R}", 2), ["{0}", "{U/R}"]);
        assert_eq!(totals("{2}{U}{U/R}", 2), ["{1}{U/R}", "{2}"]);
        assert_eq!(totals("{R}{W/B}", 1), ["{W/B}{R}"], "no symbol a blue half pays");
        assert_eq!(totals("{1}{U/R}{U/B}", 1), ["{1}{U/B}", "{1}{U/R}", "{U/R}{U/B}"], "either may be taken off, or neither");
        // A symbol still hybrid meets no reduction.
        assert_eq!(reduce(cantrip_api::mana("{U/R}{U/R}"), &less(2)), cantrip_api::mana("{U/R}{U/R}"));
    }

    /// Mind Spring with X = 254, {254}{U}{U}, under 300 flooded lands (Eluge: 255 {U} and the next 45 as generic): free.
    #[test]
    fn a_blue_reduction_past_255_takes_generic_mana() {
        let cost = ManaCost { generic: 254, symbols: [0, 2, 0, 0, 0, 0], ..ManaCost::ZERO };
        let less = ManaCost { generic: 45, symbols: [0, 255, 0, 0, 0, 0], ..ManaCost::ZERO };
        assert_eq!(reduce(cost, &less), ManaCost::ZERO);
        assert_eq!(reduce(cost, &ManaCost { symbols: [0, 255, 0, 0, 0, 0], ..ManaCost::ZERO }), ManaCost { generic: 1, ..ManaCost::ZERO });
    }

    fn creatures(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        q.chars(id).is(CardType::Creature)
    }

    /// Two creatures of the first player's: one with two -1/-1 counters and a stun counter, the other with one +1/+1
    /// counter.
    fn countered_creatures() -> (Game, ObjectId, ObjectId) {
        let (mut game, ids) = table(&[(&BEAR, Zone::BATTLEFIELD, ME), (&BEAR, Zone::BATTLEFIELD, ME)]);
        let mut c = cx(&mut game);
        counters::put(&mut c, ids[0], CounterKind::MinusOne, 2, ME, counters::How::Cost);
        counters::put(&mut c, ids[0], CounterKind::Stun, 1, ME, counters::How::Cost);
        counters::put(&mut c, ids[1], CounterKind::PlusOne, 1, ME, counters::How::Cost);
        (game, ids[0], ids[1])
    }

    /// CR 118.3, 122.1: "remove two +1/+1 counters from among creatures you control" (Hopeful Initiate) can't be paid with
    /// -1/-1 or stun counters, so one +1/+1 counter among them is too few, and the choice offers only +1/+1 counters.
    #[test]
    fn a_cost_of_one_kind_of_counters_from_among_creatures_takes_only_that_kind() {
        let (mut game, shrunk, grown) = countered_creatures();
        let cost = cantrip_api::remove_kind_counters_among(CounterKind::PlusOne, 2, creatures);
        let among = cost.remove_counters_among.expect("a removal");
        let c = cx(&mut game);
        assert!(!counters_among_payable(&c, grown, ME, &cost));
        let pools: Vec<_> = counter_pools(&c, grown, ME, among, None).iter().map(|p| (p.object, p.kind, p.count)).collect();
        assert_eq!(pools, [(grown, CounterKind::PlusOne, 1)]);
        let mut c = cx(&mut game);
        counters::put(&mut c, shrunk, CounterKind::PlusOne, 1, ME, counters::How::Cost);
        assert!(counters_among_payable(&c, grown, ME, &cost));
        let pools: Vec<_> = counter_pools(&c, grown, ME, among, None).iter().map(|p| (p.object, p.kind, p.count)).collect();
        assert_eq!(pools, [(shrunk, CounterKind::PlusOne, 1), (grown, CounterKind::PlusOne, 1)]);
    }

    /// CR 118.3, 122.1: "remove three counters from among creatures you control" (Tayam, Luminous Enigma) takes counters of
    /// any kinds, -1/-1 and stun counters too.
    #[test]
    fn a_cost_of_any_counters_from_among_creatures_takes_every_kind() {
        let (mut game, shrunk, grown) = countered_creatures();
        let cost = cantrip_api::remove_counters_among(4, creatures);
        let c = cx(&mut game);
        assert!(counters_among_payable(&c, grown, ME, &cost));
        assert!(!counters_among_payable(&c, grown, ME, &cantrip_api::remove_counters_among(5, creatures)));
        let pools: Vec<_> = counter_pools(&c, grown, ME, cost.remove_counters_among.expect("a removal"), None).iter().map(|p| (p.object, p.kind)).collect();
        assert_eq!(pools.len(), 3);
        assert!(pools.contains(&(shrunk, CounterKind::MinusOne)) && pools.contains(&(shrunk, CounterKind::Stun)) && pools.contains(&(grown, CounterKind::PlusOne)));
    }

}
