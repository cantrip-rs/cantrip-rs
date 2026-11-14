//! Replacement effects (CR 614, 616): a permanent's static ability that changes what an event does as it happens,
//! rather than reacting afterwards. The core proposes the event ([`Proposal`]), finds the effects that apply, has the
//! affected player order them when several do (CR 616.1), and carries out what is left.
//!
//! An effect is a pair of pure functions over the proposal: `applies` reads the board and the proposal and says
//! whether it replaces this event, `apply` edits the proposal. Neither writes to the game; the core does that once
//! every effect has had its say. Each effect applies at most once to a given event (CR 614.5).

use cantrip_state::*;

use crate::card::Asker;
use crate::game::{Game, Query};

/// An object about to change zones (CR 614.1a, 614.1c, 614.12), as the effects of other permanents see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZoneChange {
    pub object: ObjectId,
    pub from: Zone,
    /// Where it is going. Setting this is how "exile it instead" and "shuffle it into its library instead" work.
    pub to: Zone,
    pub reason: ZoneReason,
    /// For an object entering the battlefield, the player it enters under (CR 110.2a): the affected player, and the
    /// "you" of an effect about what enters under your control. The object's controller otherwise.
    pub controller: PlayerId,
    /// The player putting it onto the battlefield (CR 110.2a, 608.2c), independent of its entering controller. None
    /// outside a battlefield arrival or for actorless rules/setup. A replacement redirecting a move to the battlefield
    /// names its own instructed player, or the core supplies that replacement's controller (CR 609.1).
    pub put_by: Option<PlayerId>,
    /// It enters the battlefield tapped (CR 614.1c).
    pub tapped: bool,
    /// It is put onto the battlefield attacking (CR 614.1c, 508.4), as "instead it enters tapped and attacking" says. Such
    /// an effect names no defender, so its controller chooses what it attacks as it arrives
    /// (`ops::enter_attacking_by_choice`, CR 508.4), if it is a creature the rules let attack at all (CR 506.3b). Read only
    /// of an arrival on the battlefield, by the move that puts it there (`ops::move_together`) and by a token's creation: a
    /// move to any other zone ignores it.
    pub attacking: bool,
    /// The counters an effect gives it as it arrives, whatever the zone (CR 122.6, 614.1c): the void counter a card
    /// exiled in place of a graveyard carries. Not those a permanent enters with of its own (`Entering`).
    pub counters: [i32; CounterKind::COUNT],
    /// The player who puts `counters` on it (CR 122.6a): the player it arrives under, for a permanent entering with them
    /// unless the effect names another; the controller of the replacement effect that gives them to a card arriving in any
    /// other zone (CR 609.1), which the core sets as the effect applies.
    pub counters_by: PlayerId,
    /// It is revealed as it moves (CR 701.20a).
    pub reveal: bool,
    /// The library it arrives in is shuffled afterwards (CR 701.24a).
    pub shuffle: bool,
    /// It is a card, not a token or a copy of a spell (CR 108.2).
    pub is_card: bool,
    /// Its own madness (CR 702.35a) has exiled it in place of the graveyard it was discarded to: the core's replacement,
    /// ordered with the others (CR 616.1), whose linked "when this card is exiled this way" trigger follows the move.
    pub madness: bool,
    /// The card is discarded to pay a cost that specifies a characteristic of it ("unless you discard a creature card", "you
    /// may discard a land card instead", "Discard a creature card:"; CR 118.12): set by the discard, never by a replacement.
    /// A card put into a hidden zone unrevealed has its characteristics undefined and can't pay such a cost (CR 701.9c), so
    /// no replacement that would put it there is offered, and it goes where the others leave it.
    pub pays_characteristic: bool,
    /// The card is exiled by this ability (CR 607.2b, 406.6): a replacement effect of an object with an ability that refers
    /// to cards "exiled with" it has made the event exile the card (`cantrip_mechanics::replacements::exile_instead_linked_apply`),
    /// and the card in exile is linked to that object's text as it arrives (`Query::exiled_with`). Only the direct result of
    /// that replacement is linked: a later effect that sends the card elsewhere leaves it unlinked, which the core sees to.
    pub exiled_with: Option<Linked>,
    /// A spell leaving the stack that was cast with flashback, harmonize or aftermath (CR 702.34a, 702.180a, 702.127a): its
    /// own "exile this card instead of putting it anywhere else any time it would leave the stack", which the core orders
    /// with the other replacement effects that would change the move (CR 616.1). Set by the move, never by a replacement.
    pub flashback: bool,
    /// A spell leaving the stack whose cast carried "if that spell would be put into your graveyard, exile it instead" (CR
    /// 614.1a, `StackItem::exile_instead`): the core's replacement of its move to the graveyard, ordered with the others (CR
    /// 616.1). Set by the move, never by a replacement.
    pub exile_instead: ExileInstead,
    /// A spell cast as an Adventure, resolving (CR 715.3d): "instead of putting [it] into its owner's graveyard as it resolves,
    /// its controller exiles it", and may play the card from there. The core's replacement, ordered with the others (CR
    /// 616.1); `on_adventure` says it applied. Set by the move, never by a replacement.
    pub adventure: bool,
    /// Merged commander components individually replaced into the command zone under CR 903.9c. The permanent's
    /// `to` remains the destination of its top component; these identities split into the command zone separately.
    pub commander_components: [ObjectId; 8],
    /// An Omen spell resolving (CR 720.3d): its controller shuffles it into its owner's library instead of putting it into the
    /// graveyard. The core's replacement, ordered with the others (CR 616.1). Set by the move.
    pub omen: bool,
    /// A spell with rebound cast from its controller's hand, resolving (CR 702.88a): exiled instead of put into the graveyard,
    /// to be cast at its controller's next upkeep. The core's replacement, ordered with the others (CR 616.1); `rebounding`
    /// says it applied. Set by the move.
    pub rebound: bool,
    /// The Adventure's exile applied (`adventure`): the card exiled is on an Adventure for the spell's controller.
    pub on_adventure: bool,
    /// Rebound's exile applied (`rebound`): its delayed triggered ability is created with the move.
    pub rebounding: bool,
    /// The move is a destruction (CR 701.8a) that a regeneration shield may replace (CR 701.19a, 614.8): by an effect that
    /// destroys, or by lethal damage (CR 704.5g-h), and not one that says it can't be regenerated (CR 701.19c). Set by the
    /// move. Continuous regeneration prohibitions are checked by the core when
    /// a shield or a static replacement would regenerate it.
    pub destroy: bool,
    /// Regeneration replaced the destruction (CR 701.19a-b): the permanent does
    /// not move, and is regenerated instead, by a shield or a static ability.
    pub regenerated: bool,
    /// The regeneration came from a shield (CR 701.19a), which must be consumed.
    /// A static ability's regeneration (701.19b) consumes no unrelated shield.
    /// Set by the core's shield replacement; card replacements leave it false.
    pub regeneration_shield: bool,
    /// A shield counter replaced destruction due to an effect (CR 122.1c): "if this permanent would be destroyed as the
    /// result of an effect, instead remove a shield counter from it". Not a regeneration (the rulings), and not for state-based destruction.
    pub shielded: bool,
    /// A replacement effect made this event impossible, so the object does not move and no further replacement applies (CR 614.6).
    /// Its `then` instructions still resolve as part of the effect that replaced it.
    pub replaced: bool,
}

impl ZoneChange {
    /// The move is a destruction (CR 701.8a), whether or not a regeneration shield may replace it: by an effect that destroys,
    /// one that says it can't be regenerated (CR 701.19c), or lethal damage (CR 704.5g-h). Read from `destroy` and `reason`,
    /// never stored apart. A later replacement that changes `to` away from the owner's graveyard has changed the destruction
    /// event (CR 614.6); a reader checks `to` for that.
    pub fn destruction(&self) -> bool {
        self.destroy || self.reason == ZoneReason::DestroyNoRegen
    }
}

/// Counters that would be put on a permanent (CR 122.6, 614.1a): by an effect, or as it enters with them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CounterPlacement {
    pub object: ObjectId,
    /// Its controller, or for a permanent entering, the player it enters under: whose choice CR 616.1 is, and the
    /// "you control" an effect reads.
    pub controller: PlayerId,
    /// The counters are those a permanent enters the battlefield with (CR 122.6): it is not on the battlefield yet and
    /// is judged as it would exist there (CR 614.12).
    pub entering: bool,
    /// How many of each kind would be put.
    pub counts: [i32; CounterKind::COUNT],
    /// The player who puts them (CR 122.6a), always one: the controller of the spell or ability whose effect puts them
    /// (CR 608.2), the player paying a cost or doing the action (CR 118.3, 714.3c), the controller of a source whose damage
    /// puts them (CR 120.3d), or for a permanent entering, its controller unless the effect names another (CR 122.6a).
    /// "If you would put one or more counters on a permanent or player" is keyed to this.
    pub putter: PlayerId,
    /// An effect puts them (CR 609.1): a resolving spell's or ability's, or the replacement effect a permanent enters with
    /// them by (CR 614.1c, 122.6). Not a cost's (CR 118.3: a loyalty ability's, CR 606.4) nor a turn-based action's (a Saga's
    /// lore counter, CR 714.3c). "If an effect would put one or more counters" (Doubling Season) is keyed to this.
    pub effect: bool,
}

/// Counters that would be put on a player (CR 122.1, 122.6): poison, energy and experience counters, which a player has
/// rather than an object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerCounters {
    pub player: PlayerId,
    /// The player who puts them, always one, as for `CounterPlacement::putter`: the controller of a source whose damage
    /// gives poison counters (CR 120.3b, 702.90b), or of the spell or ability whose effect does.
    pub putter: PlayerId,
    pub poison: u16,
    pub energy: u16,
    pub experience: u16,
}

/// An instruction to draw cards, before any of the individual draws (CR 121.2a).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawInstruction {
    pub player: PlayerId,
    pub count: u32,
}

/// An instruction to mill cards (CR 701.17a), before any card moves: "if an opponent would mill one or more cards, they
/// mill twice that many cards instead" (Bruvac the Grandiloquent) changes `count` (CR 614.1a). It is the instruction's
/// number, which the library may not hold all of (CR 701.17b: as many as possible are milled once the effects have
/// applied). Each player's mill of a "mill together" is its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MillInstruction {
    pub player: PlayerId,
    pub count: u64,
}

/// A permanent about to connive (CR 701.50): "if a creature you control would connive, instead you draw a card, then that
/// creature connives" (Leader, Super-Genius), what the effect does besides being its `ReplacementThen`. The creature is
/// its last known information if it has left the battlefield (CR 701.50b); `count` is the N of connive N.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Connive {
    pub creature: ObjectId,
    pub count: u32,
}

/// One card about to be drawn (CR 121.1, 121.6): replaceable even when the library has no cards to draw. `replaced`
/// is set by an effect that replaces the draw ("instead you win the game"): nothing is drawn then (CR 614.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardDraw {
    pub player: PlayerId,
    pub replaced: bool,
}

/// Damage about to be dealt (CR 614.1a, 120.3): by a source to a player or a permanent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageEvent {
    pub source: ObjectId,
    pub target: Target,
    pub amount: u32,
    pub combat: bool,
    /// The damage already marked on the target is healed as this is dealt: Wolverine's "all other damage already dealt
    /// to him is healed". Damage dealt at the same time as this stays.
    pub heal_marked: bool,
    /// A shield counter of this permanent has already applied to this damage event (CR 122.1c, 120.4): all the damage dealt to it
    /// by the simultaneous event is prevented and a single shield counter is removed. Set by the effect as it applies, and carried
    /// by the damage event from one assignment to the next (`ops::DamageBatch`).
    pub shielded: Option<ObjectId>,
}

/// Tokens about to be created (CR 111.1, 614.1a): `count` of one definition under one player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenCreation {
    pub controller: PlayerId,
    pub token: CardId,
    pub count: u32,
    /// Copy values used by token-creation replacements (CR 707.2). An effect that replaces the token with a definition
    /// of its own sets them to that token under its source's words (`Query::token_copy`, CR 612.4).
    pub copy: Option<Copiable>,
    /// "Those tokens plus a [token] are created instead" (CR 614.1a): one more token of this definition, with these copy
    /// values, created under the same player in the same event. An effect sets it on the groups it applies to; the event
    /// gains a single such token however many of its groups the effect applied to, and the effects not yet applied meet
    /// that token too (CR 616.1f). Its copy values are the token under the effect's source's words (`Query::token_copy`):
    /// a text change on the source changes the token it adds (CR 612.4, 612.2a).
    pub plus: Option<(CardId, Option<Copiable>)>,
}

/// Life a player is about to gain or lose (CR 119.3, 614.1a): by an effect, as a cost (CR 119.4: paying life is losing
/// it), as the result of damage (CR 120.3a; the damage itself is a `DamageEvent`, already dealt), by lifelink
/// (CR 120.3f) or as a life total is set to a number (CR 119.5). A transient u64 magnitude and a sign preserve an
/// aggregate through replacements; the final event's magnitude is u32. "That player loses that much life instead" changes the sign. An effect that leaves it at zero leaves no
/// event (CR 119.10). A player who can't gain life meets no replacement of a gain (CR 119.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifeChange {
    pub player: PlayerId,
    pub magnitude: u64,
    pub negative: bool,
}

impl LifeChange {
    /// The signed amount for intermediate arithmetic; recorded events use a u32 magnitude and a sign.
    pub fn signed(self) -> i64 {
        debug_assert!(self.magnitude <= i64::MAX as u64, "life proposal exceeds its transient domain");
        if self.negative { -(self.magnitude as i64) } else { self.magnitude as i64 }
    }

    pub fn from_signed(player: PlayerId, amount: i64) -> Self {
        Self { player, magnitude: amount.unsigned_abs(), negative: amount < 0 }
    }
}

/// A permanent tapped for mana producing it (CR 106.12, 106.12b): all the mana the mana ability produces as it resolves,
/// before any of it is added to a pool. "If [a permanent] is tapped for mana, it produces [mana] instead" changes `mana`,
/// its type and amount (CR 614.1a); each effect has one opportunity at it (CR 614.5), and the player who tapped it orders
/// several (CR 616.1). What a triggered mana ability adds (CR 605.1b: "add an additional {G}") is no part of it: those
/// trigger on the mana as the effects have left it (CR 106.12a). Only a mana ability with {T} in its cost taps a permanent
/// for mana (CR 106.12), so no other ability's mana is proposed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManaProduction {
    /// The permanent tapped for mana, the source of the mana ability (CR 106.3). A cost may have sacrificed it ("{T},
    /// Sacrifice this artifact: Add one mana of any color"): `Query::chars` then reads it as it last existed on the
    /// battlefield (CR 608.2h, its last-known information).
    pub permanent: ObjectId,
    /// The player who tapped it, who activated the ability (CR 605.3b). "If you tap a permanent for mana" (Mana Reflection)
    /// is keyed to this.
    pub player: PlayerId,
    /// The player whose pool the mana goes to: the tapper's, or the one the ability names ("that player adds", Spectral
    /// Searchlight).
    pub to: PlayerId,
    /// The mana it produces, by type.
    pub mana: ManaPool,
    /// What restricts the mana or what it does as it is spent (CR 106.6): a rider of the ability, which stays on all the
    /// mana it produces, however much and of whatever type (Mana Reflection's ruling).
    pub restriction: Option<ManaRestriction>,
    /// How long the mana lasts (CR 106.4, 500.5a), as the ability says.
    pub lasts: ManaLasts,
}

impl ManaProduction {
    /// How much mana it produces, of every type.
    pub fn total(&self) -> u32 {
        self.mana.total()
    }
}

/// A proposed event a replacement effect may reshape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proposal {
    ZoneChange(ZoneChange),
    Counters(CounterPlacement),
    PlayerCounters(PlayerCounters),
    DrawInstruction(DrawInstruction),
    Draw(CardDraw),
    Connive(Connive),
    Mill(MillInstruction),
    Damage(DamageEvent),
    TokenCreation(TokenCreation),
    LifeChange(LifeChange),
    ManaProduction(ManaProduction),
}

/// The kinds of event a replacement effect watches for, so an effect is asked only about events it could replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplacementKind {
    ZoneChange,
    Counters,
    DrawInstruction,
    Draw,
    /// A permanent conniving (CR 701.50): `Proposal::Connive`.
    Connive,
    /// An instruction to mill cards (CR 701.17a): `Proposal::Mill`.
    Mill,
    Damage,
    TokenCreation,
    LifeChange,
    /// A permanent tapped for mana producing it (CR 106.12b): `Proposal::ManaProduction`.
    ManaProduction,
}

impl Proposal {
    pub fn kind(&self) -> ReplacementKind {
        match self {
            Proposal::ZoneChange(_) => ReplacementKind::ZoneChange,
            Proposal::Counters(_) | Proposal::PlayerCounters(_) => ReplacementKind::Counters,
            Proposal::DrawInstruction(_) => ReplacementKind::DrawInstruction,
            Proposal::Draw(_) => ReplacementKind::Draw,
            Proposal::Connive(_) => ReplacementKind::Connive,
            Proposal::Mill(_) => ReplacementKind::Mill,
            Proposal::Damage(_) => ReplacementKind::Damage,
            Proposal::TokenCreation(_) => ReplacementKind::TokenCreation,
            Proposal::LifeChange(_) => ReplacementKind::LifeChange,
            Proposal::ManaProduction(_) => ReplacementKind::ManaProduction,
        }
    }
}

/// Whether an effect replaces this event, judged on the board as it is. The `Asker` names the effect's source and
/// the player "you" means: its controller, or for an effect about its own entry, the player it enters under.
pub type ReplacementApplies = fn(&dyn Query, &Asker, &Proposal) -> bool;

/// What an effect does to the event.
pub type ReplacementApply = fn(&dyn Query, &Asker, &mut Proposal);

/// What an effect does besides reshaping the event: the work of "instead you create a Treasure token" (CR 614.1a), done
/// to the game once the effect has applied. It is handed the event as it was proposed and as the effect left it, so it
/// can act on the difference (Hullbreacher: a Treasure for the draw the effect replaced). What it does is the modified
/// event (CR 614.6): a card it has drawn meets the replacement effects that have not yet had their one opportunity at the
/// event (CR 614.5), so a second Thought Reflection doubles each of the two cards the first one draws, and the first doubles
/// neither. A question it asks that has no answer yet suspends the run, which is redone with the answer (`Game::ask`): it
/// returns there, doing nothing more.
pub type ReplacementThen = fn(&mut dyn Game, &Asker, &Proposal, &Proposal);

/// Additional damage components of the modified damage event (CR 120.4b-d, 614.6),
/// formed after `apply` and before damage has any results. Eye for an Eye adds
/// damage by a second source; Harm's Way and Captain's Maneuver split damage to
/// another recipient. These components inherit the effects already applied to
/// their parent (CR 614.5), and meet remaining replacement and prevention effects.
/// They are dealt simultaneously with every original component. An ordinary
/// `then` that calls `Game::damage` instead deals a separate event.
pub type ReplacementDamage = fn(&dyn Query, &Asker, &DamageEvent, &DamageEvent) -> Vec<DamageComponent>;

/// A component introduced while replacing damage (CR 614.6, 614.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageComponent {
    /// Damage by an additional source, such as Eye for an Eye. This is not damage
    /// dealt by the original assignment for `Game::damage`'s return value.
    Additional(DamageEvent),
    /// Part of the original damage redirected to another recipient. It retains
    /// the original source, combat status and return-value identity; the reshaped
    /// original proposal contains the amount remaining at its first recipient.
    Redirected { target: Target, amount: u32 },
}

/// The part of a mana replacement's condition independent of the amount and types produced.
/// Damping Sphere and Contamination ask whether the permanent is a land. Restricting this
/// predicate to the source makes the quantity contract executable authority, rather than
/// an assertion about an opaque callback that might inspect an undeclared amount.
pub type ManaReplacementSource = fn(&dyn Query, &Asker, ObjectId, PlayerId) -> bool;

/// Replace a qualifying tap's entire mana output by a fixed type and amount (CR 106.12b,
/// 614.1a). `minimum` is the printed total-quantity threshold. The actual replacement and
/// the CR 106.7 quantity-domain partition both use this same definition.
#[derive(Clone, Copy)]
pub struct FixedManaReplacement {
    pub minimum: u32,
    pub source: ManaReplacementSource,
    pub mana: Mana,
    pub amount: u32,
}

/// A replacement effect of a static ability (CR 614.1, 604).
#[derive(Clone, Copy)]
pub struct ReplacementDef {
    /// What it does, in a few words: shown beside its source when a player orders several (CR 616.1), and what a
    /// scenario's `Replace(...)` can name it by.
    pub hint: &'static str,
    pub kind: ReplacementKind,
    /// It functions in every zone, on the move of its own card only: "if this card would be put into a graveyard from
    /// anywhere" (CR 113.6b). Otherwise it functions while its source is on the battlefield (CR 604.1).
    pub own_everywhere: bool,
    pub applies: ReplacementApplies,
    pub apply: ReplacementApply,
    pub fixed_mana: Option<FixedManaReplacement>,
    /// Game operations the effect performs as it applies (see [`ReplacementThen`]).
    pub then: Option<ReplacementThen>,
    /// Additional components of a damage event; see [`ReplacementDamage`].
    pub damage: Option<ReplacementDamage>,
    /// "You may ... instead" (CR 614.1a): the effect's "you", its source's controller (CR 109.5), is asked this as the effect would apply, and declining it
    /// leaves the event as it was, for the other effects (CR 616.1). Dredge's and Abundance's.
    pub optional: Option<&'static Question>,
    /// It functions only while its card is in a graveyard (CR 113.6, 702.52a: dredge), "you" being the card's owner, and
    /// only on that player's draws: the one kind of event such an ability replaces. Never while the card is on the
    /// battlefield.
    pub in_graveyard: bool,
}

/// Whether a replacement effect a spell or ability made (`EffectKind::Replacement`) replaces this event, judged on the board
/// as it is. The `Asker` names the effect's source and its controller; the object is the one it is about.
pub type GrantedApplies = fn(&dyn Query, &Asker, ObjectId, &Proposal) -> bool;

/// What a replacement effect a spell or ability made does to the event.
pub type GrantedApply = fn(&dyn Query, &Asker, ObjectId, &mut Proposal);

/// What a replacement effect a spell or ability made does to the game as it applies, besides reshaping the event: as a
/// [`ReplacementThen`], also told the object the effect is about.
pub type GrantedThen = fn(&mut dyn Game, &Asker, ObjectId, &Proposal, &Proposal);

/// [`ReplacementDamage`] for an effect made about an object.
pub type GrantedDamage = fn(&dyn Query, &Asker, ObjectId, &DamageEvent, &DamageEvent) -> Vec<DamageComponent>;

/// A replacement effect (CR 614) that a resolving spell or ability makes for a duration (CR 611.2), as opposed to a static
/// ability's (`ReplacementDef`): it is not a card's standing ability, but a continuous effect that lasts, and it is
/// about the object it was made for.
#[derive(Clone, Copy)]
pub struct GrantedReplacementDef {
    /// What it does, in a few words: shown beside its source when a player orders several (CR 616.1).
    pub hint: &'static str,
    pub kind: ReplacementKind,
    pub applies: GrantedApplies,
    pub apply: GrantedApply,
    /// Game operations the effect performs as it applies (see [`GrantedThen`]).
    pub then: Option<GrantedThen>,
    /// Additional components of a damage event; see [`ReplacementDamage`].
    pub damage: Option<GrantedDamage>,
    /// "The next time [an event] would happen this turn, ... instead" (CR 614.1a): the effect is used up by the one event it
    /// replaces, and ends as it applies; a second such effect waits for the next event.
    pub once: bool,
    /// The effect's words, as its card's Oracle text prints them (the sentence that makes it, and what it does besides): what
    /// tells whether it refers to its own source while it waits to apply (CR 609.7a, `CardTable::names_its_source`), as Torch
    /// the Tower's "If a permanent dealt damage by Torch the Tower would die this turn, exile it instead" does. A shared
    /// definition leaves it empty and each card using it gives its own (`.text(..)`); `crates/cantrip-cards/tests/
    /// waiting_texts_match_oracle.rs` holds every card's to its Oracle text.
    pub text: &'static str,
}

/// Whether a prevention effect applies to this damage (CR 615.1), judged on the board as it is and on the damage as the
/// effects applied so far have left it (CR 616.1f). The `Asker` names the effect's source and its controller.
pub type PreventionApplies = fn(&dyn Query, &Asker, &DamageEvent) -> bool;

/// How much of the damage a prevention effect prevents (CR 615.1), judged on the damage as the effects applied so far have
/// left it (CR 616.1f): "prevent half that damage, rounded up". More than the damage is all of it.
pub type PreventionAmountFn = fn(&dyn Query, &Asker, &DamageEvent) -> u32;

/// What the printed prevention quantity means (CR 615.1, 615.10), not just its evaluated number.
#[derive(Clone, Copy)]
pub enum PreventionAmount {
    /// Prevent all applicable damage.
    All,
    /// A specific allowance per source/recipient damage event (615.10), computed when it first applies and then
    /// consumed without refill. It may depend on game state, but not on the changing damage quantity: Urza's Armor's 1,
    /// for example. A fresh native damage event computes a fresh capacity.
    Specific(PreventionAmountFn),
    /// A quantity relative to the event as modified so far (616.1f): Gisela's half, rounded up.
    /// Its opportunity follows the applicable damage lineage; it is not a fixed allowance.
    Relative(PreventionAmountFn),
}

/// A prevention effect of a static ability (CR 615, 604): "Prevent all damage that would be dealt to Black Panther". It
/// functions while its source is on the battlefield, and prevents all of the damage it applies to, or as much as `amount`
/// says. The core orders it with the replacement effects that would change the same damage (CR 616.1), skips it where the
/// damage can't be prevented (CR 615.12) and records what it prevents (`Event::DamagePrevented`).
#[derive(Clone, Copy)]
pub struct PreventionDef {
    /// What it does, in a few words: shown beside its source when a player orders several (CR 616.1).
    pub hint: &'static str,
    pub applies: PreventionApplies,
    /// The printed quantity contract: all damage, a specific source-event allowance, or an event-relative portion.
    /// Gisela's existing partial-amount callback is `Relative`, never reinterpreted as a specific allowance.
    pub amount: PreventionAmount,
}

/// Whether a prevention effect a spell or ability made applies to this damage (CR 615.1), judged as `PreventionApplies` is,
/// with the object the effect was made about (`ContinuousEffect::affected`): the resolving spell itself for one about no
/// object ("prevent all combat damage that would be dealt this turn"), else the object it names ("that creature").
pub type GrantedPreventionApplies = fn(&dyn Query, &Asker, ObjectId, &DamageEvent) -> bool;

/// A prevention effect a resolving spell or ability makes (CR 615, 611.2): "prevent the next 3 damage that would be dealt
/// to any target this turn", "prevent all combat damage that would be dealt this turn". The effect itself
/// (`EffectKind::Prevention`) holds whom the damage must be dealt to and how much is left to prevent (`Shield`); this says
/// what else the damage must be, if anything (combat damage, from a source, to or by the object it is about).
#[derive(Clone, Copy)]
pub struct GrantedPreventionDef {
    /// What it does, in a few words: shown beside its source when a player orders several (CR 616.1).
    pub hint: &'static str,
    /// The damage it prevents, as its text names it ("damage", "combat damage"): what is shielded is shown with "Prevent
    /// the next 3 damage that would be dealt to it this turn".
    pub damage: &'static str,
    /// For an effect about an object that prevents damage named by more than whom it is dealt to, how its text names that
    /// damage in relation to the object ("to and dealt by it": Maze of Ith's "that creature"): the effect is shown on the
    /// object so. None for one about whom the damage is dealt to, or about no object.
    pub of_it: Option<&'static str>,
    pub applies: GrantedPreventionApplies,
    /// What the effect does besides preventing the damage (CR 615.5; see [`PreventionThen`]): "If damage is prevented this
    /// way, Deflecting Palm deals that much damage to that source's controller." None for an effect that only prevents.
    pub then: Option<PreventionThen>,
    /// The effect's words, as its card's Oracle text prints them - the sentence that makes it and, for one with more to do,
    /// the rest (CR 615.5): what tells whether it refers to its own source while it waits to apply (CR 609.7a: an object a
    /// waiting prevention effect refers to; `CardTable::names_its_source`) - Deflecting Palm's "Deflecting Palm deals that
    /// much damage" does, Fog's "Prevent all combat damage that would be dealt this turn" does not. A shared definition leaves
    /// it empty and each card using it gives its own (`.text(..)`), held to its Oracle text by
    /// `crates/cantrip-cards/tests/waiting_texts_match_oracle.rs`.
    pub text: &'static str,
}

/// Damage a prevention effect prevented of one part of a damage event (CR 615.5): from `source` to `target`, `amount` of it
/// (more than 0), combat damage or not. "That much damage" is what was prevented, not what was proposed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prevented {
    pub source: ObjectId,
    pub target: Target,
    pub amount: u32,
    pub combat: bool,
}

/// The rest of a prevention effect a spell or ability made (CR 615.5): "the prevention takes place at the time the original
/// event would have happened; the rest of the effect takes place immediately afterward" - once the damage event it prevented
/// damage of has been dealt and announced, before state-based actions are checked and before the spell or ability dealing
/// that damage goes on resolving. Run once for that event, told what the effect prevented of each of its parts (every
/// source and recipient the effect prevented damage of, in the order it did) and the object the effect is about
/// (`ContinuousEffect::affected`: a chosen source). Never run where the effect prevented nothing, as of damage that can't be
/// prevented (CR 615.12): every printed such effect says "if damage is prevented this way" or counts "the damage prevented
/// this way". What it does, the effect's controller does (CR 609.1), as the effect's source - the spell, as it last existed
/// (CR 608.2h) - deals the damage it deals. A question it asks that has no answer yet suspends the run, which is redone
/// with the answer: it returns there, doing nothing more.
pub type PreventionThen = fn(&mut dyn Game, &Asker, ObjectId, &[Prevented]);

impl Prevented {
    /// How much the effect prevented in all, of these parts: "that much damage".
    pub fn total(parts: &[Prevented]) -> u32 {
        parts.iter().map(|p| p.amount).sum::<u32>()
    }
}

fn never(_: &dyn Query, _: &Asker, _: &Proposal) -> bool {
    false
}

fn nothing(_: &dyn Query, _: &Asker, _: &mut Proposal) {}

impl ReplacementDef {
    pub const BLANK: ReplacementDef =
        ReplacementDef {
        hint: "",
        kind: ReplacementKind::ZoneChange,
        own_everywhere: false,
        applies: never,
        apply: nothing,
        fixed_mana: None,
        then: None,
        damage: None,
        optional: None,
        in_graveyard: false,
    };

    pub fn applies_to(&self, q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
        if let Some(rule) = self.fixed_mana {
            matches!(proposal, Proposal::ManaProduction(m) if m.total() >= rule.minimum
                && (rule.source)(q, asker, m.permanent, m.player))
        } else {
            (self.applies)(q, asker, proposal)
        }
    }

    pub fn reshape(&self, q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
        if let Some(rule) = self.fixed_mana {
            if let Proposal::ManaProduction(m) = proposal {
                m.mana = ManaPool::default();
                m.mana.0[rule.mana as usize] = rule.amount;
            }
        } else {
            (self.apply)(q, asker, proposal);
        }
    }
}
