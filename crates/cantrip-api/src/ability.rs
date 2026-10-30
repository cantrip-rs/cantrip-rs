//! How a card writes its abilities: the cost on the left, what the ability does on the right, its printed text last.
//!
//! ```ignore
//! mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")
//! ability(cost("{2}{G}"), pump_team, "{2}{G}: Creatures you control get +0/+1 until end of turn.")
//! ability(loyalty(-2), steal, "\u{2212}2: Gain control of target artifact.").targets(&[targets::ARTIFACT])
//! triggered(when(EventMask::ZONE_CHANGE, this_enters), draw_a_card, "When this creature enters, draw a card.")
//! ```
//!
//! What few abilities need (targets, "activate only once each turn", a condition) is a method after the call. The
//! rest of a card is written the same way: `spell(&[targets::ANY], effect)`, `target(filter)`,
//! `static_ability(layer, affects, apply, text)`, `mode(effect, text)`, `replacement(kind, applies, apply, hint)`,
//! `way(cost, from, text)`, `kicker(mana("{2}"))`.

use cantrip_state::*;

use crate::card::*;
use crate::event::EventMask;
use crate::game::Game;
use crate::replace::{GrantedApplies, GrantedApply, GrantedReplacementDef, GrantedThen, ReplacementApplies, ReplacementApply, ReplacementDef, ReplacementKind, ReplacementThen};

/// The tap symbol, {T} (CR 107.5).
pub const TAP: Cost = Cost { tap: true, ..Cost::NONE };

/// "Sacrifice this permanent" (CR 701.21).
pub const SACRIFICE_THIS: Cost = Cost { sacrifice_self: true, ..Cost::NONE };

/// "Discard this card" (CR 701.9).
pub const DISCARD_THIS: Cost = Cost { discard_self: true, ..Cost::NONE };

/// "Exile this card" or "Exile this permanent".
pub const EXILE_THIS: Cost = Cost { exile_self: true, ..Cost::NONE };

/// "Exert this permanent" (CR 701.43a).
pub const EXERT: Cost = Cost { exert: true, ..Cost::NONE };

/// "Unattach this Equipment" (CR 701.3d).
pub const UNATTACH_THIS: Cost = Cost { unattach_self: true, ..Cost::NONE };

/// "Return this permanent to its owner's hand".
pub const RETURN_THIS: Cost = Cost { return_self: true, ..Cost::NONE };

/// "Discard your hand" (CR 701.9).
pub const DISCARD_HAND: Cost = Cost { discard_hand: true, ..Cost::NONE };

/// "Remove a counter from this permanent", of any kind.
pub const REMOVE_A_COUNTER: Cost = Cost { remove_any_counter: true, ..Cost::NONE };

/// "Sacrifice N [permanents]" that pass the filter (CR 701.21).
pub const fn sacrifice(n: u32, filter: ObjectFilter) -> Cost {
    Cost::sacrifice(n, filter)
}

/// "Return N [permanents] you control to their owner's hand".
pub const fn return_to_hand(n: u32, filter: ObjectFilter) -> Cost {
    Cost::return_to_hand(n, filter)
}

/// "Return N tapped [permanents] you control to their owner's hand" (web-slinging, CR 702.188a): chosen once the mana
/// is paid, so one tapped for mana while paying may be returned (`PermanentCost::tapped`).
pub const fn return_tapped_to_hand(n: u32, filter: ObjectFilter) -> Cost {
    Cost::return_tapped_to_hand(n, filter)
}

/// "Exile N [permanents] you control" (CR 118.3): The Soul Stone's "Exile a creature you control", Food Chain's. Chosen as
/// the cost is chosen (CR 601.2h, 602.2b); the exiled permanents are the ability's `Query::cost_objects` (read as they
/// last were on the battlefield, CR 608.2h: "the exiled creature's mana value"), and are "exiled with" its source (CR
/// 607.2a).
pub const fn exile_permanents(n: u32, filter: ObjectFilter) -> Cost {
    Cost::exile_permanents(n, filter)
}

/// "Tap N untapped [permanents] you control" (CR 118.3): not the {T} symbol.
pub const fn tap_untapped(n: u32, filter: ObjectFilter) -> Cost {
    Cost::tap_untapped(n, filter)
}

/// "Discard N cards" (CR 701.9): any cards of the hand.
pub const fn discard_cards(n: u8) -> Cost {
    Cost { discard: Discards::one(n, None), ..Cost::NONE }
}

/// "Discard N [kind of] cards" (CR 701.9): only cards that pass the filter, a cost that specifies a characteristic of them
/// (CR 701.9c) - a filter that passes every card would still be read as naming one, so "discard N cards" is `discard_cards`.
pub const fn discard(n: u8, filter: ObjectFilter) -> Cost {
    Cost { discard: Discards::one(n, Some(filter)), ..Cost::NONE }
}

/// "Exile N cards that pass the filter from your hand", other than this one (CR 118.9).
pub const fn exile_from_hand(n: u8, filter: ObjectFilter) -> Cost {
    Cost { exile_from_hand: Some((n, filter)), ..Cost::NONE }
}

/// "Remove N [kind] counters from this permanent".
pub const fn remove_counters(kind: CounterKind, n: u8) -> Cost {
    Cost { remove_counters: Some((kind, n)), ..Cost::NONE }
}

/// "Remove N counters from among [permanents you control that pass the filter]", of any kinds (CR 118.3, 122.1).
pub const fn remove_counters_among(n: u8, filter: ObjectFilter) -> Cost {
    Cost { remove_counters_among: Some(CountersAmong { count: n, filter, kind: None }), ..Cost::NONE }
}

/// "Remove N [kind] counters from among [permanents you control that pass the filter]": only counters of that kind pay it
/// (CR 118.3, 122.1). "Remove two +1/+1 counters from among creatures you control" (Hopeful Initiate).
pub const fn remove_kind_counters_among(kind: CounterKind, n: u8, filter: ObjectFilter) -> Cost {
    Cost { remove_counters_among: Some(CountersAmong { count: n, filter, kind: Some(kind) }), ..Cost::NONE }
}

/// "Put N [kind] counters on this [permanent]" as a cost (CR 118.3, 602.2b): Mazemind Tome's page counter.
pub const fn put_counters(kind: CounterKind, n: u32) -> Cost {
    Cost { put_counters: Some((kind, n)), ..Cost::NONE }
}

/// "Exile N cards from your graveyard" other than this one.
pub const fn exile_from_graveyard(n: u8) -> Cost {
    Cost { exile_from_graveyard: n, ..Cost::NONE }
}

/// "Exile N [quality] cards from your graveyard" other than this one: "Exile three creature cards from your graveyard".
pub const fn exile_from_graveyard_of(n: u8, filter: ObjectFilter) -> Cost {
    Cost { exile_from_graveyard: n, exile_from_graveyard_of: Some(filter), ..Cost::NONE }
}

/// A cost written in mana symbols as printed: `cost("{2}{G}")`, `cost("{X}{R}")`, `cost("{W/U}")`, `cost("{G/P}")`,
/// `cost("{1}, {T}")`. Read as the card is compiled: a symbol it doesn't know fails the build.
pub const fn cost(symbols: &str) -> Cost {
    let (mana, tap) = parse_symbols(symbols);
    Cost { mana, tap, ..Cost::NONE }
}

/// A mana cost written in its symbols as printed: `mana("{2}{G}")`, `mana("{X}{R}")`, `mana("{W/U}")`, `mana("{G/P}")`.
/// Read as the card is compiled: a symbol it doesn't know fails the build.
pub const fn mana(symbols: &str) -> ManaCost {
    let (mana, tap) = parse_symbols(symbols);
    assert!(!tap, "the tap symbol is not mana: write it in cost()");
    mana
}

/// N generic mana, N worked out by the card: `generic(price)`.
pub const fn generic(n: u8) -> ManaCost {
    ManaCost { generic: n, ..ManaCost::ZERO }
}

const fn parse_symbols(symbols: &str) -> (ManaCost, bool) {
    let b = symbols.as_bytes();
    let mut c = ManaCost::ZERO;
    let mut tap = false;
    let mut hybrid = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'{' => {}
            b'}' | b',' | b' ' => {
                i += 1;
                continue;
            }
            _ => panic!("a mana symbol is written in braces"),
        }
        let mut j = i + 1;
        while j < b.len() && b[j] != b'}' {
            j += 1;
        }
        assert!(j < b.len(), "an unclosed mana symbol");
        let inner = (i + 1, j);
        match inner.1 - inner.0 {
            1 => match b[inner.0] {
                b'T' => tap = true,
                b'X' => c.x += 1,
                b'0'..=b'9' => c.generic += b[inner.0] - b'0',
                m => c.symbols[mana_of(m) as usize] += 1,
            },
            2 => {
                assert!(b[inner.0].is_ascii_digit() && b[inner.0 + 1].is_ascii_digit(), "an unknown mana symbol");
                c.generic += (b[inner.0] - b'0') * 10 + (b[inner.0 + 1] - b'0');
            }
            3 => {
                assert!(b[inner.0 + 1] == b'/', "an unknown mana symbol");
                if b[inner.0] == b'2' {
                    // A monocolored hybrid symbol, {2/W} (CR 107.4e).
                    let color = mana_of(b[inner.0 + 2]);
                    assert!(!matches!(color, Mana::C), "a monocolored hybrid symbol has a color");
                    c.twobrid[color as usize] += 1;
                    i = j + 1;
                    continue;
                }
                let first = mana_of(b[inner.0]);
                if b[inner.0 + 2] == b'P' {
                    assert!(!matches!(first, Mana::C), "Phyrexian colorless mana is not a symbol");
                    c.phyrexian[first as usize] += 1;
                } else {
                    assert!(hybrid < 6, "more hybrid symbols than a cost holds");
                    c.hybrid[hybrid] = Some((first, mana_of(b[inner.0 + 2])));
                    hybrid += 1;
                }
            }
            _ => panic!("an unknown mana symbol"),
        }
        i = j + 1;
    }
    (c, tap)
}

const fn mana_of(symbol: u8) -> Mana {
    match symbol {
        b'W' => Mana::W,
        b'U' => Mana::U,
        b'B' => Mana::B,
        b'R' => Mana::R,
        b'G' => Mana::G,
        b'C' => Mana::C,
        _ => panic!("an unknown mana symbol"),
    }
}

/// A loyalty cost (CR 606.4): `loyalty(1)` for +1, `loyalty(-2)` for -2. Its ability is activated at sorcery speed
/// (CR 606.3).
pub const fn loyalty(n: i8) -> Cost {
    Cost { loyalty: Some(n), ..Cost::NONE }
}

/// The loyalty cost "−X" (CR 606.4, 107.3): `loyalty_minus_x()`.
pub const LOYALTY_MINUS_X: i8 = i8::MIN;

/// A loyalty cost of "−X" (CR 606.4, 107.3): X is announced as the ability is activated (CR 601.2b by way of 602.2b), up
/// to the loyalty the permanent has, since a cost can't remove counters that aren't there (CR 118.3); that many loyalty
/// counters are removed as the cost is paid, and the ability reads X as `StackItem::x`. Chandra, Chill of Compliance,
/// Ugin, the Spirit Dragon and Tezzeret the Seeker print it.
pub const fn loyalty_minus_x() -> Cost {
    loyalty(LOYALTY_MINUS_X)
}

/// "Pay N life" (CR 119.4).
pub const fn life(n: u8) -> Cost {
    Cost { life: n as u32, ..Cost::NONE }
}

/// A cost of this mana alone, worked out as the effect runs: Override's {1} per artifact, Flash's creature's mana cost
/// less {2} (`Game::may_pay`).
pub const fn of_mana(mana: ManaCost) -> Cost {
    Cost { mana, ..Cost::NONE }
}

/// An activated ability (CR 602) that uses the stack: pay `cost`, then `effect` as it resolves. A loyalty ability is
/// activated at sorcery speed (CR 606.3).
pub const fn ability(cost: Cost, effect: Effect, text: &'static str) -> AbilityDef {
    AbilityDef::Activated(ActivatedDef {
        text,
        cost,
        effect,
        timing: if cost.loyalty.is_some() { Timing::Sorcery } else { Timing::Instant },
        ..ActivatedDef::BLANK
    })
}

/// A mana ability (CR 605.1a): pay `cost`, and what `adds` says is added at once, never on the stack (CR 605.3).
pub const fn mana_ability(cost: Cost, adds: Adds, text: &'static str) -> AbilityDef {
    AbilityDef::Activated(ActivatedDef {
        text,
        cost,
        mana: adds.effect,
        adds: adds.pool,
        adds_any: adds.any,
        any_color: adds.any_color,
        colors: adds.colors,
        restricted: adds.restricted,
        ..ActivatedDef::BLANK
    })
}

/// When a triggered ability triggers (CR 603.2): the events it listens to, and whether one of them is its trigger event.
#[derive(Clone, Copy)]
pub struct Trigger {
    pub listens: EventMask,
    pub matches: TriggerMatch,
    /// The trigger event asked with the ability's controller, in place of `matches` when set (`when_asked`).
    pub asked: Option<AskedMatch>,
    /// The zones the trigger condition functions in (CR 113.6, 113.6k; `TriggeredDef::functions`): the battlefield, unless
    /// the condition can't trigger from there.
    pub functions: ZoneKinds,
}

/// The trigger of a triggered ability: `when(EventMask::ZONE_CHANGE, this_enters)`. It functions on the battlefield (CR 113.6).
pub const fn when(listens: EventMask, matches: TriggerMatch) -> Trigger {
    Trigger { listens, matches, asked: None, functions: ON_THE_BATTLEFIELD }
}

/// "When you cast this spell" (CR 601.2i, 603.2): the trigger of the spell's own casting, which `matches` is asked of (the
/// `SpellCast` event whose `spell` is the source, with anything else the condition says). A spell is cast on the stack, so
/// the condition can't trigger from the battlefield and functions from the stack (CR 113.6k): every trigger of this form
/// is made here, so none can be left functioning where its spell never is. Storm, cascade, replicate, ripple and casualty
/// (CR 702.40a, 702.85a, 702.56a, 702.60a, 702.153a) and Emrakul, the Promised End's "When you cast this spell" use it.
pub const fn when_this_spell_is_cast(matches: TriggerMatch) -> Trigger {
    Trigger { listens: EventMask::SPELL_CAST, matches, asked: None, functions: ON_THE_STACK }
}

/// Where a triggered ability functions unless its trigger condition or its text says otherwise (CR 113.6).
pub const ON_THE_BATTLEFIELD: ZoneKinds = ZoneKinds::of(&[ZoneKind::Battlefield]);
/// Where "when you cast this spell" functions (CR 113.6k, `when_this_spell_is_cast`).
pub const ON_THE_STACK: ZoneKinds = ZoneKinds::of(&[ZoneKind::Stack]);
/// Where an ability that returns "this card from your graveyard" functions (CR 113.6m), and one that triggers on its own
/// card being put into a graveyard (CR 113.6k).
pub const IN_A_GRAVEYARD: ZoneKinds = ZoneKinds::of(&[ZoneKind::Graveyard]);
/// Where a trigger on its own card being discarded functions - "when you discard this card", "when you cycle this card",
/// "when a spell or ability an opponent controls causes you to discard this card": every zone a discarded card can wind up
/// in, a replacement effect's included (CR 113.6k, 702.29c, 614.1) - its graveyard, exile (madness, Leyline of the Void),
/// the battlefield (Obstinate Baloth), its hand, and its library (Library of Leng, Wheel of Sun and Moon). The discard is the
/// event, not a move to a graveyard, so this holds for one whose effect then returns "this card from your graveyard" (Sand
/// Golem, Mangara's Blessing) too. One put into a hidden zone unrevealed triggers on nothing (CR 701.9c,
/// `Event::Discarded::undefined`).
pub const WHERE_A_DISCARD_ENDS: ZoneKinds =
    ZoneKinds::of(&[ZoneKind::Graveyard, ZoneKind::Exile, ZoneKind::Library, ZoneKind::Hand, ZoneKind::Battlefield]);

impl Trigger {
    /// A trigger condition that can't trigger from the battlefield, which functions in the zones it can trigger from (CR
    /// 113.6k): a shared matcher's own, as "when this card is put into a graveyard from anywhere" functions from the
    /// graveyard it was put into (`cantrip_mechanics::triggers::PUT_INTO_A_GRAVEYARD_FROM_ANYWHERE`).
    pub const fn functions_in(self, zones: ZoneKinds) -> Trigger {
        Trigger { functions: zones, ..self }
    }
}

/// The trigger of a triggered ability whose trigger event says "you": `matches` is told the ability's controller
/// (`AskedMatch`), which a delayed triggered ability keeps from the spell or ability that created it (CR 603.7d-e).
pub const fn when_asked(listens: EventMask, matches: AskedMatch) -> Trigger {
    Trigger { listens, matches: crate::card::no_match, asked: Some(matches), functions: ON_THE_BATTLEFIELD }
}

/// A triggered ability (CR 603): when `trigger` triggers, `effect` as it resolves.
pub const fn triggered(trigger: Trigger, effect: Effect, text: &'static str) -> AbilityDef {
    AbilityDef::Triggered(TriggeredDef {
        text,
        listens: trigger.listens,
        matches: trigger.matches,
        asked: trigger.asked,
        functions: trigger.functions,
        effect,
        ..TriggeredDef::BLANK
    })
}

/// A state-triggered ability (CR 603.8): "when [a game state is true], [effect]". It triggers as soon as `state` holds of
/// its source, and not again until it has left the stack; then, if its source is still there and the state still holds, it
/// triggers again. Mazemind Tome ("when there are four or more page counters on this artifact"), Dandân ("when you control
/// no Islands, sacrifice Dandân") and Lurking Jackals ("when an opponent has 10 or less life, if this is an enchantment, it
/// becomes ...") print one.
pub const fn state_triggered(state: TriggerCondition, effect: Effect, text: &'static str) -> AbilityDef {
    AbilityDef::Triggered(TriggeredDef { text, state: Some(state), effect, ..TriggeredDef::BLANK })
}

/// A modal activated ability (CR 700.2): pay `cost`, choose among `modes` as it is activated (CR 602.2b).
pub const fn modal_ability(cost: Cost, modes: &'static ModalDef, text: &'static str) -> AbilityDef {
    assert!(modes.max != ModalDef::COUNTED, "an activated ability has no count for \"that many\" modes");
    AbilityDef::Activated(ActivatedDef {
        text,
        cost,
        modal: Some(modes),
        timing: if cost.loyalty.is_some() { Timing::Sorcery } else { Timing::Instant },
        ..ActivatedDef::BLANK
    })
}

/// A modal triggered ability (CR 700.2): when `trigger` triggers, its modes are chosen as it is put on the stack
/// (CR 603.3c).
pub const fn modal_triggered(trigger: Trigger, modes: &'static ModalDef, text: &'static str) -> AbilityDef {
    AbilityDef::Triggered(TriggeredDef { text, listens: trigger.listens, matches: trigger.matches, asked: trigger.asked, modal: Some(modes), ..TriggeredDef::BLANK })
}

/// An optional cost to attack and the triggered ability linked to it (CR 508.1g, 607.2h): "As this creature attacks, you
/// may [pay `option`]. When you do, [`effect`]." - enlist (CR 702.154a) and "you may exert this creature as it attacks.
/// When you do, ..." (CR 701.43d). The core offers the cost as attackers are declared and owns the trigger
/// (`TriggeredDef::attack_option`); `cantrip_mechanics::enlist` and `cantrip_mechanics::exert` build it for cards.
pub const fn paid_as_it_attacks(option: AttackOption, effect: Effect, text: &'static str) -> AbilityDef {
    AbilityDef::Triggered(TriggeredDef { text, listens: EventMask::PAID_TO_ATTACK, attack_option: Some(option), effect, ..TriggeredDef::BLANK })
}

/// A delayed triggered ability (CR 603.7), listed in a card's `delayed`: the effect that created it says when it
/// triggers (`Game::delayed_trigger`), and `effect` is what it does then.
pub const fn delayed_trigger(effect: Effect, text: &'static str) -> AbilityDef {
    AbilityDef::Triggered(TriggeredDef { text, effect, ..TriggeredDef::BLANK })
}

/// A modal delayed or reflexive triggered ability (CR 603.7, 603.12, 700.2): "when you do, choose one -" (Caesar, Legion's
/// Emperor; Gorbag of Minas Morgul), "choose up to that many -" (Tranquil Frillback, `ModalDef::up_to_that_many`). Its
/// modes and their targets are chosen as it is put on the stack (CR 603.3c).
pub const fn modal_delayed_trigger(modes: &'static ModalDef, text: &'static str) -> AbilityDef {
    AbilityDef::Triggered(TriggeredDef { text, modal: Some(modes), ..TriggeredDef::BLANK })
}

/// A Saga's chapter ability (CR 714.2): it triggers when lore counters on its Saga reach `n`, and the core owns that
/// trigger.
pub const fn chapter(n: u8, effect: Effect, text: &'static str) -> AbilityDef {
    AbilityDef::Triggered(TriggeredDef { text, chapter: n, effect, ..TriggeredDef::BLANK })
}

/// What a mana ability adds (CR 106.4): mana of kinds it lists, mana of one color chosen as it is activated, or what an
/// effect of the card's own adds.
#[derive(Clone, Copy)]
pub struct Adds {
    pub pool: ManaPool,
    /// How many mana of the one color chosen as the ability is activated (CR 106.5).
    pub any: u8,
    /// The activation names a color (CR 106.5).
    pub any_color: bool,
    /// The colors that one may be; none for all five.
    pub colors: ColorSet,
    pub restricted: Option<ManaRestriction>,
    /// The card's own effect, where the ability does more than add the mana listed: the listing is then only what
    /// anyone may know of it before it is activated.
    pub effect: Option<ManaEffect>,
}

impl Adds {
    const NOTHING: Adds = Adds { pool: ManaPool([0; 6]), any: 0, any_color: false, colors: ColorSet::NONE, restricted: None, effect: None };

    /// "Spend this mana only ..." (CR 106.6): the mana goes to the pool apart from unrestricted mana.
    /// A restriction of card types that is not one of `ManaRestriction::NAMED` fails the build.
    pub const fn only_for(self, restriction: ManaRestriction) -> Adds {
        assert!(restriction.is_known(), "a mana restriction the wire has no word for: add it to ManaRestriction::NAMED");
        Adds { restricted: Some(restriction), ..self }
    }

    /// The colors its mana of one chosen color may be (CR 106.5): "Add one mana of any color among ...".
    pub const fn colors(self, colors: ColorSet) -> Adds {
        Adds { colors, ..self }
    }

    /// The activation names a color (CR 106.5) for the card's own effect, which may add it or not: Gemstone Caverns'
    /// "instead add one mana of any color".
    pub const fn choose_color(self) -> Adds {
        Adds { any_color: true, ..self }
    }

    /// For whoever asks before it is activated, it may add `n` mana of any color, chosen by its own effect: Orcish
    /// Lumberjack's "three mana in any combination of {R} and/or {G}".
    pub const fn yields_any(self, n: u8) -> Adds {
        Adds { any: n, ..self }
    }

    /// What the card's own effect adds, listed as this: "{T}: Add {G} for each creature you control" is
    /// `add(Mana::G).by(green_for_each_creature)`.
    pub const fn by(self, effect: ManaEffect) -> Adds {
        Adds { effect: Some(effect), ..self }
    }
}

/// "Add {G}."
pub const fn add(mana: Mana) -> Adds {
    add_n(mana, 1)
}

/// "Add {C}{C}." is `add_n(Mana::C, 2)`.
pub const fn add_n(mana: Mana, n: u8) -> Adds {
    let mut pool = [0u32; 6];
    pool[mana as usize] = n as u32;
    Adds { pool: ManaPool(pool), ..Adds::NOTHING }
}

/// "Add {W}{U}." is `add_each(&[Mana::W, Mana::U])`.
pub const fn add_each(mana: &[Mana]) -> Adds {
    let mut pool = [0u32; 6];
    let mut i = 0;
    while i < mana.len() {
        pool[mana[i] as usize] += 1;
        i += 1;
    }
    Adds { pool: ManaPool(pool), ..Adds::NOTHING }
}

/// "Add N mana of any one color." (CR 106.5)
pub const fn add_any_color(n: u8) -> Adds {
    Adds { any: n, any_color: true, ..Adds::NOTHING }
}

/// "Add {R} or {G}." / "Add {B}, {R}, or {G}.": one mana of a color chosen among these as the ability is activated.
pub const fn add_one_of(colors: &[Color]) -> Adds {
    let mut set = 0u8;
    let mut i = 0;
    while i < colors.len() {
        set |= ColorSet::of(colors[i]).0;
        i += 1;
    }
    Adds { any: 1, any_color: true, colors: ColorSet(set), ..Adds::NOTHING }
}

/// A mana ability's effect that the card writes itself, with nothing listed: Gaea's Cradle adds a number no one knows
/// before it is activated.
pub const fn add_by(effect: ManaEffect) -> Adds {
    Adds { effect: Some(effect), ..Adds::NOTHING }
}

/// Adds what a mana ability with no effect of its own lists (CR 106.4): its fixed mana, and its mana of one color as
/// the color `chosen` (colorless where it may name colorless, CR 106.7). Restricted mana goes to the pool apart.
pub fn add_listed(g: &mut dyn Game, controller: PlayerId, def: &ActivatedDef, chosen: Option<Mana>) {
    let mut pool = def.adds;
    if let Some(m) = chosen {
        pool.0[m as usize] += def.adds_any as u32;
    }
    for m in Mana::ALL {
        let n = pool.get(m);
        if n == 0 {
            continue;
        }
        match def.restricted {
            Some(r) => g.add_restricted_mana(controller, m, n, r),
            None => g.add_mana(controller, m, n),
        }
    }
}

const fn activated(a: AbilityDef, method: &str) -> ActivatedDef {
    match a {
        AbilityDef::Activated(d) => d,
        AbilityDef::Triggered(_) | AbilityDef::Static(_) => panic!("{}", method),
    }
}

impl AbilityDef {
    /// Its targets (CR 115.1), one slot each.
    pub const fn targets(self, targets: &'static [TargetSpec]) -> AbilityDef {
        match self {
            AbilityDef::Activated(d) => AbilityDef::Activated(ActivatedDef { targets, ..d }),
            AbilityDef::Triggered(t) => AbilityDef::Triggered(TriggeredDef { targets, ..t }),
            AbilityDef::Static(_) => panic!("a static ability has no targets"),
        }
    }

    /// "Activate only as a sorcery." (CR 602.5d)
    pub const fn sorcery_speed(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { timing: Timing::Sorcery, ..activated(self, "only an activated ability has a timing") })
    }

    /// "Activate only as an instant" (CR 304.5, 602.5e): only with priority, including a mana ability.
    pub const fn only_as_an_instant(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { priority_only: true, ..activated(self, "only an activated ability requires priority") })
    }

    /// When it may be activated, beyond sorcery speed: `Timing::OwnTurn` and the like.
    pub const fn timing(self, timing: Timing) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { timing, ..activated(self, "only an activated ability has a timing") })
    }

    /// "Activate only once each turn." (CR 602.5b)
    pub const fn once_each_turn(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { once_each_turn: true, ..activated(self, "only an activated ability is activated once each turn") })
    }

    /// Exhaust (CR 702.177a): activated only once.
    pub const fn exhaust(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { exhaust: true, once: true, ..activated(self, "only an activated ability has exhaust") })
    }

    /// Its targets are chosen once X is announced (CR 601.2b-c by way of 602.2b), as a target that depends on X needs:
    /// Likeness Looter's "target creature card in your graveyard with mana value X".
    pub const fn targets_after_x(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { late_targets: true, ..activated(self, "only an activated ability is announced") })
    }

    /// "Activate only once." (CR 602.5b)
    pub const fn once(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { once: true, ..activated(self, "only an activated ability is activated only once") })
    }

    /// Power-up (CR 702.193a): "activate this ability only once", an additional time for each permanent its controller controls that
    /// says so (`CardDef::extra_power_up_activations`).
    pub const fn power_up(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { once: true, power_up: true, ..activated(self, "only an activated ability is a power-up") })
    }

    /// Its own cost change (CR 601.2f, 602.2b): "this ability costs ... less to activate".
    pub const fn cost_change(self, change: AbilityCostChange) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { cost_change: Some(change), ..activated(self, "only an activated ability has a cost") })
    }

    /// The zone it functions from, where not the battlefield (CR 113.6).
    pub const fn from(self, from: ZoneKind) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { from, ..activated(self, "only an activated ability is given a zone this way") })
    }

    /// "Activate only if ..." (CR 602.5b), or a triggered ability's intervening "if" (CR 603.4).
    pub const fn condition(self, condition: Condition) -> AbilityDef {
        match self {
            AbilityDef::Activated(d) => AbilityDef::Activated(ActivatedDef { condition: Some(condition), ..d }),
            AbilityDef::Triggered(t) => AbilityDef::Triggered(TriggeredDef { condition: Some(condition), ..t }),
            AbilityDef::Static(_) => panic!("a static ability has no condition"),
        }
    }

    /// "This ability triggers only if ..." (CR 702.169c): asked with the trigger event only, never as the ability resolves.
    pub const fn only_if(self, only_if: Condition) -> AbilityDef {
        match self {
            AbilityDef::Triggered(t) => AbilityDef::Triggered(TriggeredDef { only_if: Some(only_if), ..t }),
            AbilityDef::Activated(_) | AbilityDef::Static(_) => panic!("only a triggered ability triggers only if"),
        }
    }

    /// Its modes (CR 700.2): chosen on activation, or as the triggered ability is put on the stack.
    pub const fn modal(self, modal: &'static ModalDef) -> AbilityDef {
        match self {
            AbilityDef::Activated(_) if modal.max == ModalDef::COUNTED => panic!("an activated ability has no count for \"that many\" modes"),
            AbilityDef::Activated(d) => AbilityDef::Activated(ActivatedDef { modal: Some(modal), ..d }),
            AbilityDef::Triggered(t) => AbilityDef::Triggered(TriggeredDef { modal: Some(modal), ..t }),
            AbilityDef::Static(_) => panic!("a static ability has no modes"),
        }
    }

    /// The colors its mana of one chosen color may be, worked out when it is offered (CR 607.2a): "any of the exiled
    /// card's colors".
    pub const fn colors_from(self, colors_from: fn(&dyn crate::game::Query, &Asker) -> ColorSet) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { colors_from: Some(colors_from), ..activated(self, "only a mana ability names colors") })
    }

    /// "Add one mana of any type the sacrificed land could produce" (CR 106.7).
    pub const fn colors_from_sacrifice(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { colors_from_sacrifice: true, ..activated(self, "only a mana ability names colors") })
    }

    /// Its mana cost is a waterbend cost (CR 701.67a).
    pub const fn waterbend(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { waterbend: true, ..activated(self, "only an activated ability has a mana cost to waterbend") })
    }

    /// "For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay
    /// that mana" (CR 118.3).
    pub const fn tap_creatures_for_mana(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { tap_creatures_for_mana: true, ..activated(self, "only an activated ability has an activation cost") })
    }

    /// A special action (CR 116.2), not an activated ability.
    pub const fn special_action(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { special_action: true, ..activated(self, "a special action is taken as an activated ability is") })
    }

    /// Plot's special action (CR 702.170a, 116.2k).
    pub const fn plot(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { plot: true, special_action: true, ..activated(self, "plotting is taken as an activated ability is") })
    }

    /// An equip ability (CR 702.6a).
    pub const fn equip(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { equip: true, ..activated(self, "only an activated ability equips") })
    }

    /// A cycling ability (CR 702.29a, 702.29f): its cost's discard is cycling its card (`ActivatedDef::cycling`). For the
    /// keyword's shared mechanics alone.
    pub const fn cycling(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { cycling: true, ..activated(self, "only an activated ability cycles") })
    }

    /// "X can't be 0" (CR 107.3a, 602.2b): the X of its activation cost is announced as 1 or more.
    pub const fn x_cant_be_zero(self) -> AbilityDef {
        AbilityDef::Activated(ActivatedDef { x_cant_be_zero: true, ..activated(self, "only an activated ability announces an X") })
    }

}

const fn triggered_def(a: AbilityDef, method: &str) -> TriggeredDef {
    match a {
        AbilityDef::Triggered(t) => t,
        AbilityDef::Activated(_) | AbilityDef::Static(_) => panic!("{}", method),
    }
}

/// What only a triggered ability (CR 603) has.
impl AbilityDef {
    /// It functions only in `zones`, as its own text says (CR 113.6b), or as its effect says by moving its card out of a zone
    /// (CR 113.6m): "return this card from your graveyard to the battlefield" functions only in the graveyard. What the
    /// trigger condition decides is the trigger's (`when_this_spell_is_cast`, `Trigger::functions_in`), never the card's.
    /// The words are held to the declaration (`cantrip-cards/tests/functioning_zones_match_text.rs`).
    pub const fn functions_in(self, zones: ZoneKinds) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { functions: zones, ..triggered_def(self, "only a triggered ability functions in zones this way") })
    }

    /// A "one or more" ability (CR 603.2c): the events of one batch that it matches trigger it once.
    pub const fn once_per_batch(self) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { once_per_batch: true, ..triggered_def(self, "only a triggered ability triggers per batch") })
    }

    /// With `once_per_batch`: events whose `data[1]` differ are different batches (CR 603.2c).
    pub const fn batch_by_data(self) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { batch_by_data: true, ..triggered_def(self, "only a triggered ability triggers per batch") })
    }

    /// A triggered mana ability (CR 605.1b), with what it adds to the player who tapped the permanent (`TriggeredMana`).
    pub const fn mana_ability(self, adds: TriggeredMana) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef {
            mana_ability: true,
            mana_yield: adds,
            ..triggered_def(self, "an activated mana ability is written with mana_ability()")
        })
    }

    /// Events of one batch about the same subject trigger it once, their amounts added up (CR 120.2, 603.2c).
    pub const fn sums_per_subject(self) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { sums_per_subject: true, ..triggered_def(self, "only a triggered ability sums its events") })
    }

    /// "This ability triggers only N times each turn" (no numbered rule; CR 603.2h is "do this only once each turn").
    pub const fn triggers_each_turn(self, n: u8) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { triggers_each_turn: n, ..triggered_def(self, "only a triggered ability triggers") })
    }

    /// "Up to that many target ..." where that many is the amount of the event that triggered it, as the life gained or the damage
    /// dealt: a `TargetSpec::COUNTED` bound reads it as the ability is put on the stack (CR 603.3d, 601.2c).
    pub const fn counted_by_amount(self) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { counted_by_amount: true, ..triggered_def(self, "only a triggered ability is counted by an event's amount") })
    }

    /// "Up to X target ..." where X is read from the game as the ability is put on the stack (CR 603.3d, 601.2c): a
    /// `TargetSpec::COUNTED` bound reads `count`, asked of the ability's source and controller. It is fixed there; the ability's
    /// own text reads the game again as it resolves.
    pub const fn counted_by(self, count: fn(&dyn crate::Query, &Asker) -> u16) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { counted_by: Some(count), ..triggered_def(self, "only a triggered ability is counted by the game") })
    }

    /// Its first target slot divides a total among its targets (CR 601.2d).
    pub const fn divide(self, divide: Divide) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { divide: Some(divide), ..triggered_def(self, "only a triggered ability divides this way") })
    }

    /// "That player chooses target ..." (CR 603.3d).
    pub const fn subject_chooses_targets(self) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { subject_chooses_targets: true, ..triggered_def(self, "only a triggered ability's subject chooses") })
    }

    /// A ward ability (CR 702.21a).
    pub const fn ward(self) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { ward: true, ..triggered_def(self, "only a triggered ability is a ward ability") })
    }

    /// A flanking ability (CR 702.25a).
    pub const fn flanking(self) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { flanking: true, ..triggered_def(self, "only a triggered ability is a flanking ability") })
    }

    /// Its trigger event is a sacrifice (CR 701.21a), not the zone change it causes: "whenever you sacrifice a land".
    pub const fn on_sacrifice(self) -> AbilityDef {
        AbilityDef::Triggered(TriggeredDef { on_sacrifice: true, ..triggered_def(self, "only a triggered ability triggers on a sacrifice") })
    }
}

// --- Spells, targets, modes ---------------------------------------------------------------------------------------

/// An instant's or sorcery's own ability (CR 113.3a): its targets, and what it does as it resolves.
pub const fn spell(targets: &'static [TargetSpec], effect: Effect) -> Option<SpellDef> {
    Some(SpellDef { targets, effect })
}

/// One target that passes the filter (CR 115.1).
pub const fn target(filter: TargetFilter) -> TargetSpec {
    TargetSpec { min: 1, max: 1, filter, label: "", if_kicked: None, together: None }
}

/// "Up to one target" (CR 115.1d).
pub const fn up_to_one(filter: TargetFilter) -> TargetSpec {
    TargetSpec { min: 0, max: 1, filter, label: "", if_kicked: None, together: None }
}

/// "Up to N targets" (CR 115.1d).
pub const fn up_to(n: u32, filter: TargetFilter) -> TargetSpec {
    TargetSpec { min: 0, max: n, filter, label: "", if_kicked: None, together: None }
}

impl TargetSpec {
    /// The same slot, which may be left without a target ("up to").
    pub const fn optional(self) -> TargetSpec {
        TargetSpec { min: 0, ..self }
    }

    /// The same slot, with between `min` and `max` targets; either may be `TargetSpec::COUNTED`.
    pub const fn between(self, min: u32, max: u32) -> TargetSpec {
        TargetSpec { min, max, ..self }
    }

    /// "X targets": as many as the X announced (CR 601.2b-c).
    pub const fn counted(self) -> TargetSpec {
        TargetSpec { min: TargetSpec::COUNTED, max: TargetSpec::COUNTED, ..self }
    }

    /// "If this spell was kicked, instead choose between `min` and `max` of these targets" (CR 702.33a, 601.2c): The
    /// Eagles Are Coming!'s and Divine Resilience's "instead choose any number of target creatures" is `if_kicked(0, n)`.
    pub const fn if_kicked(self, min: u32, max: u32) -> TargetSpec {
        TargetSpec { if_kicked: Some((min, max)), ..self }
    }

    /// The same slot, its targets together judged by the set rule `rule` (`TargetSpec::together`, CR 601.2c, 115.1): "any
    /// number of target creature cards with total mana value X or less" is
    /// `.together(select::total_mana_value_at_most(TargetSpec::COUNTED))`. A quality of each target is the filter's, never
    /// this: a rule that is not a set rule (`select::set_part`) does not compile here.
    pub const fn together(self, rule: Part) -> TargetSpec {
        assert!(rule.is_set_rule(), "a target slot's rule of its targets together is a set rule (select::set_part); a quality of each target is the slot's filter");
        TargetSpec { together: Some(rule), ..self }
    }
}

/// One mode of a modal spell or ability (CR 700.2).
pub const fn mode(effect: Effect, text: &'static str) -> ModeDef {
    ModeDef { text, targets: &[], effect, cost: ManaCost::ZERO }
}

impl ModeDef {
    /// Its targets (CR 700.2c).
    pub const fn targets(self, targets: &'static [TargetSpec]) -> ModeDef {
        ModeDef { targets, ..self }
    }

    /// What choosing it adds to the total cost: spree (CR 702.172a), tiered (CR 702.183a).
    pub const fn cost(self, cost: ManaCost) -> ModeDef {
        ModeDef { cost, ..self }
    }
}

/// "Choose one —" (CR 700.2): the modes, in printed order.
pub const fn modes(modes: &'static [ModeDef]) -> ModalDef {
    ModalDef { modes, min: 1, max: 1, escalate: None, repeat: false, allowed: None, when_paid: None, shared: None }
}

impl ModalDef {
    /// "Choose one or both", "choose one or more": between `min` and `max` modes.
    pub const fn choose(self, min: u8, max: u8) -> ModalDef {
        ModalDef { min, max, ..self }
    }

    /// "Choose up to that many" (CR 603.12a, 700.2b): a triggered ability's modes, as many as its count at most - the number
    /// of times a cost was paid for a reflexive ability (`Game::may_pay_times`, `Game::reflexive_trigger_counted`). With
    /// none chosen it is removed from the stack (CR 603.3c).
    pub const fn up_to_that_many(self) -> ModalDef {
        ModalDef { min: 0, max: ModalDef::COUNTED, ..self }
    }

    /// Escalate (CR 702.120a): each mode beyond the first costs this more.
    pub const fn escalate(self, cost: Cost) -> ModalDef {
        ModalDef { escalate: Some(cost), ..self }
    }

    /// An instruction printed before the bulleted modes that is done to what all the chosen modes name together, once, as the
    /// last of the spell (CR 700.2, 608.2c): "Choose up to two. Return those cards from your graveyard to your hand." Each
    /// mode then only names its targets (`no_effect`), and `effect` reads all of the item's targets, in the printed order of
    /// the chosen modes.
    pub const fn shared(self, effect: Effect) -> ModalDef {
        ModalDef { shared: Some(effect), ..self }
    }

    /// "You may choose the same mode more than once" (CR 700.2d).
    pub const fn repeat(self) -> ModalDef {
        ModalDef { repeat: true, ..self }
    }

    /// Whether a mode may be chosen now, beyond its targets.
    pub const fn allowed(self, allowed: fn(&dyn crate::game::Query, &Asker, u8) -> bool) -> ModalDef {
        ModalDef { allowed: Some(allowed), ..self }
    }

    /// "If this spell's additional cost was paid, choose both instead", "if this spell was kicked, choose any number
    /// instead": between `min` and `max` modes when the spell's optional additional cost is paid or it is kicked
    /// (CR 601.2b, 700.2a). The cost is then announced before the modes.
    pub const fn when_paid(self, min: u8, max: u8) -> ModalDef {
        ModalDef { when_paid: Some((min, max)), ..self }
    }
}

// --- Static abilities and replacement effects -----------------------------------------------------------------------

/// A static ability (CR 604) applying `apply` in `layer` to every object `affects` accepts.
pub const fn static_ability(layer: Layer, affects: StaticAffects, apply: StaticApply, text: &'static str) -> StaticDef {
    StaticDef { text, layer, affects, apply, ..StaticDef::BLANK }
}

/// A static ability (CR 604) whose effect on each object `affects` accepts reads that object (`StaticDef::apply_each`):
/// "each other non-Aura enchantment ... has base power and base toughness each equal to its mana value" (Opalescence;
/// March of the Machines, Titania's Song).
pub const fn static_ability_for_each(layer: Layer, affects: StaticAffects, apply: crate::StaticApplyEach, text: &'static str) -> StaticDef {
    StaticDef { text, layer, affects, apply_each: Some(apply), ..StaticDef::BLANK }
}

/// A static ability by which every object `affects` accepts loses all abilities (CR 613.1f, layer 6): "enchanted
/// creature loses all abilities" (Eaten by Piranhas, Darksteel Mutation, Kenrith's Transformation, Frogify), "all
/// creatures lose all abilities" (Humility). Its effect takes away every ability the object has at its timestamp, its
/// printed ones and those granted before it (CR 613.7), and the object's own static abilities of layer 6 and later no
/// longer apply (CR 613.8a, dependency); those of earlier layers have applied by then (CR 613.6). Cost changes,
/// replacement effects, play rules and the other rules a card prints are gone with them.
pub const fn loses_all_abilities(affects: StaticAffects, text: &'static str) -> StaticDef {
    StaticDef { text, layer: Layer::Ability, affects, apply: lose_all_abilities, loses_all: true, ..StaticDef::BLANK }
}

fn lose_all_abilities(_: &dyn LayerQuery, _: ObjectId, chars: &mut crate::Chars) {
    chars.lose_all_abilities();
}

/// A static ability by which every object `affects` accepts loses all abilities except mana abilities (CR 613.1f, layer
/// 6): "all lands lose all abilities except mana abilities" (Blood Sun). It is `loses_all_abilities` in every other way -
/// the object's static abilities from layer 6 on, its replacement effects ("enters tapped": Blood Sun's ruling), cost
/// changes, play rules and triggered abilities other than mana abilities are gone, what its static abilities did in layers
/// 1 to 5 stays (CR 613.6: Urborg, Tomb of Yawgmoth's, the ruling), and what is granted it later is kept (the ruling) -
/// but it keeps each mana ability it has at its timestamp: an activated ability that adds mana (CR 605.1a, an ability
/// that costs mana is not one, the ruling) or a triggered mana ability (CR 605.1b), the basic land types' among them (CR
/// 305.6).
pub const fn loses_all_abilities_except_mana_abilities(affects: StaticAffects, text: &'static str) -> StaticDef {
    StaticDef { text, layer: Layer::Ability, affects, apply: lose_all_abilities_except_mana_abilities, loses_all: true, ..StaticDef::BLANK }
}

fn lose_all_abilities_except_mana_abilities(view: &dyn LayerQuery, _: ObjectId, chars: &mut crate::Chars) {
    let cards = view.cards();
    let kept = chars.abilities.map(|a| {
        a.filter(|&a| match cards.ability(a) {
            AbilityDef::Activated(d) => d.is_mana_ability(),
            AbilityDef::Triggered(t) => t.mana_ability,
            _ => false,
        })
    });
    chars.lose_all_abilities();
    for (slot, ability) in kept.into_iter().flatten().enumerate() {
        chars.abilities[slot] = Some(ability);
    }
}

/// A static ability that sets the land subtypes of every land `affects` accepts (CR 305.7, layer 4): "nonbasic lands are
/// Mountains" (Blood Moon, Magus of the Moon), "enchanted land is an Island" (Spreading Seas, Convincing Mirage), "this
/// land is the chosen type" (Multiversal Passage). `apply` sets them with `Chars::set_land_subtypes`, which takes away
/// the abilities of the land's rules text. Such a land's own static abilities from layer 4 on no longer apply (CR
/// 613.8a: each depends on this effect, which applies first whatever their timestamps), nor do its replacement effects,
/// cost changes and the other rules a card prints (`Query::lost_all_abilities`); a land entering is judged as it would
/// be on the battlefield (CR 614.12), so its own "enters tapped" and "as this enters" do not apply. This effect itself,
/// a layer-4 one, goes on applying when its source loses its abilities in layer 6 (CR 613.6, Magus of the Moon's ruling).
pub const fn sets_land_subtypes(affects: StaticAffects, apply: StaticApply, text: &'static str) -> StaticDef {
    StaticDef { text, layer: Layer::Type, affects, apply, sets_land_subtypes: true, ..StaticDef::BLANK }
}

/// "You control [what it affects]" (CR 613.1b, layer 2): a static ability by which its source's controller controls every
/// permanent `affects` accepts, judged on what layer 1 made it. Mind Control, Treachery, Control Magic and Persuasion print
/// "You control enchanted creature." It applies at its source's timestamp (CR 613.7; an Aura's is when it became attached,
/// 613.7e), among the resolved control-changing effects, in layer 2's order (CR 613.7, 613.8: a reach that depends on control
/// reads it through `LayerQuery::controller`). Losing its abilities in layer 6 does not stop it, as it has applied by then
/// (CR 613.1, 613.6).
pub const fn control_changing(affects: StaticAffects, text: &'static str) -> StaticDef {
    StaticDef { text, layer: Layer::Control, affects, ..StaticDef::BLANK }
}

/// A static ability an effect gives an object (CR 613.1f), to be granted with `EffectKind::AddAbility` from the card's
/// `delayed` list: the object itself is its source, so "you" and "your" are its controller's (CR 113.6, 109.5), and it
/// applies in `layer` (`Layer::Ability`, ordered with the other layer 6 effects by `appear`, or `Layer::PtDefine` and later; never layers 1-5) at the later of the object's timestamp and the
/// granting effect's (CR 613.7a). It stops when the object loses the ability (CR 613.1f) or the effect ends.
/// Ms. Marvel, Kamala Khan grants herself "base power is equal to the number of cards in your hand"; any card that
/// gives a creature a quoted static ability about its characteristics can.
pub const fn granted_static(layer: Layer, apply: StaticApply, text: &'static str) -> AbilityDef {
    AbilityDef::Static(StaticDef { text, layer, apply, own: true, ..StaticDef::BLANK })
}

/// A static ability an effect gives an object (CR 613.1f) that affects the objects `affects` accepts, the object that
/// has it as the source (CR 113.6): The Irencrag's "it gains 'Equipped creature gets +3/+3'". Granted with
/// `EffectKind::AddAbility` from the card's `delayed` list, it applies in `layer` (`Layer::Ability`, ordered with the other layer 6 effects by `appear`, or `Layer::PtDefine` and later;
/// never layers 1-5) at the later of the object's timestamp and the granting effect's (CR 613.7a), while the object is on the
/// battlefield and still has the ability.
pub const fn granted_static_ability(layer: Layer, affects: StaticAffects, apply: StaticApply, text: &'static str) -> AbilityDef {
    AbilityDef::Static(StaticDef { text, layer, affects, apply, ..StaticDef::BLANK })
}

/// A characteristic-defining ability (CR 604.3): it applies to its own object only, in every zone.
pub const fn characteristic_defining(layer: Layer, apply: StaticApply, text: &'static str) -> StaticDef {
    StaticDef { text, layer, apply, own: true, ..StaticDef::BLANK }
}

impl StaticDef {
    /// It affects objects anywhere, not only permanents (CR 611.3a).
    pub const fn everywhere(self) -> StaticDef {
        StaticDef { everywhere: true, ..self }
    }

    /// It applies to its own object only.
    pub const fn own(self) -> StaticDef {
        StaticDef { own: true, ..self }
    }

    /// What the stars of the power and toughness it defines are worth once it is lost (CR 208.2a, 208.5): `*/1+*` is 0/1.
    pub const fn lost_pt(self, power: i16, toughness: i16) -> StaticDef {
        StaticDef { lost_pt: Some((power, toughness)), ..self }
    }

    /// It is a later part of the effect of the static ability before it in the card's list (CR 613.6,
    /// `StaticDef::continues`): it applies, in its own later layer, to the same objects that part applied to.
    pub const fn continues(self) -> StaticDef {
        StaticDef { continues: true, ..self }
    }

    /// The zone its source functions from, where not the battlefield (CR 113.6b).
    pub const fn from(self, from: ZoneKind) -> StaticDef {
        StaticDef { from, ..self }
    }
}

/// A replacement effect of a static ability (CR 614.1): whether it `applies` to an event of `kind`, and how it
/// reshapes it. `hint` is what it does in a few words.
pub const fn replacement(kind: ReplacementKind, applies: ReplacementApplies, apply: ReplacementApply, hint: &'static str) -> ReplacementDef {
    ReplacementDef { hint, kind, applies, apply, ..ReplacementDef::BLANK }
}

impl ReplacementDef {
    /// "If this card would be put into a graveyard from anywhere" (CR 113.6b): it functions in every zone, on its own card.
    pub const fn own_everywhere(self) -> ReplacementDef {
        ReplacementDef { own_everywhere: true, ..self }
    }

    /// What the effect does to the game as it applies, besides reshaping the event.
    pub const fn then(self, then: ReplacementThen) -> ReplacementDef {
        ReplacementDef { then: Some(then), ..self }
    }

    /// "You may ... instead" (CR 614.1a): the effect's "you", its source's controller (CR 109.5), is asked `question` as the effect would apply.
    pub const fn optional(self, question: &'static Question) -> ReplacementDef {
        ReplacementDef { optional: Some(question), ..self }
    }

    /// It functions only while its card is in a graveyard, on its owner's draws (CR 702.52a: dredge).
    pub const fn in_graveyard(self) -> ReplacementDef {
        assert!(matches!(self.kind, ReplacementKind::Draw), "an ability of a card in a graveyard replaces only draws");
        ReplacementDef { in_graveyard: true, ..self }
    }
}

/// A replacement effect a resolving spell or ability makes for a while (CR 611.2), about the object it was made for.
pub const fn granted_replacement(kind: ReplacementKind, applies: GrantedApplies, apply: GrantedApply, hint: &'static str) -> GrantedReplacementDef {
    GrantedReplacementDef { hint, kind, applies, apply, then: None, once: false }
}

impl GrantedReplacementDef {
    /// What the effect does to the game as it applies, besides reshaping the event.
    pub const fn then(self, then: GrantedThen) -> GrantedReplacementDef {
        GrantedReplacementDef { then: Some(then), ..self }
    }

    /// "The next time ... would ...": the effect ends as it applies, used up (CR 614.1a).
    pub const fn once(self) -> GrantedReplacementDef {
        GrantedReplacementDef { once: true, ..self }
    }
}

// --- Other ways to cast, kickers -------------------------------------------------------------------------------------

/// An alternative way to cast the card (CR 118.9): for `cost`, from the zone `from`.
pub const fn way(cost: Cost, from: ZoneKind, text: &'static str) -> WayDef {
    WayDef { text, cost, from, only_from: false, exile_after: false, condition: None, instant_speed: false, enters_attacking: false, kind: WayKind::Other, turn_up: Cost::NONE }
}

/// What casting a card face down costs by morph, megamorph or disguise: {3} rather than its mana cost (CR 702.37a, 702.168a).
pub const FACE_DOWN_CAST_COST: ManaCost = generic(3);

/// A way that casts the card face down (CR 702.37c, 702.168b, 708.4): `kind` is morph, megamorph or disguise, cast for {3}
/// rather than its mana cost (CR 118.9) from any zone it could be cast from (CR 702.37c, 702.168b: the way names no zone), and
/// turned face up for `turn_up` (CR 702.37e, 702.168d). Made by the keyword's module (`cantrip_mechanics::morph`,
/// `megamorph`, `disguise`).
pub const fn face_down_way(kind: WayKind, turn_up: Cost, text: &'static str) -> WayDef {
    WayDef { kind, turn_up, ..way(Cost::mana(FACE_DOWN_CAST_COST), ZoneKind::Hand, text) }
}

impl WayDef {
    /// This way is `kind`'s keyword.
    pub const fn kind(self, kind: WayKind) -> WayDef {
        WayDef { kind, ..self }
    }

    /// Its text names the zone it is cast from ("cast this card from your hand", CR 702.185a): no permission to cast the
    /// card from another zone opens it.
    pub const fn only_from(self) -> WayDef {
        WayDef { only_from: true, ..self }
    }

    /// It is exiled instead of going anywhere else as it leaves the stack (flashback, CR 702.34a).
    pub const fn exile_after(self) -> WayDef {
        WayDef { exile_after: true, ..self }
    }

    /// It may be cast this way only when the condition holds.
    pub const fn condition(self, condition: Condition) -> WayDef {
        WayDef { condition: Some(condition), ..self }
    }

    /// It may be cast this way whenever an instant could be (sneak, CR 702.190a).
    pub const fn instant_speed(self) -> WayDef {
        WayDef { instant_speed: true, ..self }
    }

    /// A permanent spell cast this way enters tapped and attacking what the creature its cost returned to its owner's hand
    /// was attacking (sneak, CR 702.190b).
    pub const fn enters_attacking(self) -> WayDef {
        WayDef { enters_attacking: true, ..self }
    }
}

/// Kicker (CR 702.33a): an optional additional cost, paid once.
pub const fn kicker(cost: ManaCost) -> Option<Kicker> {
    Some(Kicker { cost, extra: None, multi: false, waterbend: false })
}

/// Multikicker (CR 702.33c): paid any number of times.
pub const fn multikicker(cost: ManaCost) -> Option<Kicker> {
    Some(Kicker { cost, extra: None, multi: true, waterbend: false })
}

/// A multikicker whose cost is more than mana (Squad--Exile four cards from your graveyard, CR 702.157a): paid any number of
/// times, its mana and the rest once for each. The rest must be what `Cost::times` pays that many times over faithfully -
/// exiled cards, life, energy, counters, permanents - so a cost made of a once-only part (a tap, a sacrifice or discard of this
/// object, a variable amount) is refused where the card is built.
pub const fn multikicker_cost(cost: Cost) -> Option<Kicker> {
    assert!(
        !cost.tap
            && !cost.sacrifice_self
            && !cost.sacrifice_granter
            && !cost.discard_self
            && !cost.exile_self
            && !cost.return_self
            && !cost.unattach_self
            && !cost.exert
            && !cost.discard_hand
            && cost.collect_evidence == 0
            && cost.variable.is_none()
            && cost.loyalty.is_none()
            && cost.mana.x == 0
    );
    Some(Kicker { cost: cost.mana, extra: Some(Cost { mana: ManaCost::ZERO, ..cost }), multi: true, waterbend: false })
}

/// "As an additional cost to cast this spell, you may waterbend {N}" (CR 701.67a-b).
pub const fn waterbend_kicker(cost: ManaCost) -> Option<Kicker> {
    Some(Kicker { cost, extra: None, multi: false, waterbend: true })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_reads_printed_symbols() {
        assert_eq!(cost("{2}{G}").mana, ManaCost { generic: 2, symbols: [0, 0, 0, 0, 1, 0], ..ManaCost::ZERO });
        assert_eq!(cost("{0}").mana, ManaCost::ZERO);
        assert_eq!(cost("{12}").mana.generic, 12);
        assert_eq!(cost("{X}{R}").mana, ManaCost { x: 1, symbols: [0, 0, 0, 1, 0, 0], ..ManaCost::ZERO });
        assert_eq!(cost("{1}{G/W}{G/W}").mana, ManaCost { hybrid: [Some((Mana::G, Mana::W)), Some((Mana::G, Mana::W)), None, None, None, None], generic: 1, ..ManaCost::ZERO });
        assert_eq!(cost("{9}{G/P}").mana, ManaCost { generic: 9, phyrexian: [0, 0, 0, 0, 1], ..ManaCost::ZERO });
        assert_eq!(cost("{2/W}{C/U}").mana, ManaCost { twobrid: [1, 0, 0, 0, 0], hybrid: [Some((Mana::C, Mana::U)), None, None, None, None, None], ..ManaCost::ZERO });
        assert_eq!(cost("{C}").mana.symbols, [0, 0, 0, 0, 0, 1]);
        assert_eq!(mana("{2}{G}"), cost("{2}{G}").mana);
        let c = cost("{1}, {T}");
        assert!(c.tap && c.mana == generic(1));
    }

    #[test]
    fn costs_join() {
        let c = TAP.and(life(1)).and(SACRIFICE_THIS);
        assert!(c.tap && c.sacrifice_self && c.life == 1);
        assert_eq!(cost("{1}").and(cost("{U}")).mana, ManaCost { generic: 1, symbols: [0, 1, 0, 0, 0, 0], ..ManaCost::ZERO });
    }

    #[test]
    fn energy_is_paid_in_additional_and_repeated_costs() {
        let energy = Cost { energy: 1, ..Cost::NONE };
        let total = TAP.and(energy).and(energy.times(3));
        assert_eq!(total.energy, 4);
        assert!(total.tap);
        assert!(!total.is_tap_only(), "a mana ability with an energy cost is not a plain land tap");
        assert_eq!(energy.times(0).energy, 0);
        assert!(TAP.and(energy.times(0)).is_tap_only());
    }

    #[test]
    fn mana_abilities_list_what_they_add() {
        let AbilityDef::Activated(d) = mana_ability(TAP, add(Mana::G), "{T}: Add {G}.") else { panic!() };
        assert!(d.is_mana_ability() && d.mana.is_none() && d.adds.get(Mana::G) == 1 && d.cost.tap);
        let AbilityDef::Activated(d) = mana_ability(TAP, add_one_of(&[Color::Red, Color::White]), "{T}: Add {R} or {W}.") else { panic!() };
        assert!(d.any_color && d.adds_any == 1 && d.colors.has(Color::Red) && d.colors.has(Color::White) && !d.colors.has(Color::Blue));
        let AbilityDef::Activated(d) = ability(loyalty(-2), |_, _| Ok(()), "-2: Nothing.") else { panic!() };
        assert!(d.timing == Timing::Sorcery && !d.is_mana_ability());
    }

    #[test]
    fn requiring_priority_does_not_turn_a_mana_ability_into_a_stack_ability() {
        let AbilityDef::Activated(d) = mana_ability(TAP, add(Mana::G), "{T}: Add {G}. Activate only as an instant.").only_as_an_instant() else { panic!() };
        assert!(d.priority_only && d.is_mana_ability() && d.timing == Timing::Instant);
        let AbilityDef::Activated(d) = ability(TAP, |_, _| Ok(()), "{T}: Nothing. Activate only as an instant.").only_as_an_instant() else { panic!() };
        assert!(d.priority_only && !d.is_mana_ability());
    }
}
