//! What happened, as the rules record it: the events triggers listen for and the game log is made of.

use cantrip_state::*;

use crate::history::Cast;

/// What a coin flip came to (CR 705.2): a flip its player called is won or lost; one that cares only whether it comes up
/// heads (Ral Zarek, Guest Lecturer's "the number of coins that came up heads") is heads or tails, and no one wins or
/// loses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoinFlip {
    Won,
    Lost,
    Heads,
    Tails,
}

/// The damage's combat designation and its source's five colors as it dealt damage (CR 120.3, 609.7).
/// Kept in one byte so adding excess information does not enlarge every event in the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageKind(u8);

impl DamageKind {
    pub const fn new(combat: bool, colors: ColorSet) -> Self {
        Self(colors.0 | ((combat as u8) << 5))
    }

    pub const fn is_combat(self) -> bool {
        self.0 & (1 << 5) != 0
    }

    pub const fn colors(self) -> ColorSet {
        ColorSet(self.0 & 0b1_1111)
    }
}

/// A single source's damage after replacement and prevention (CR 120.3, 120.10).
/// `target` is the actual recipient, including redirection; `excess` is damage actually dealt above lethal damage,
/// or the recipient's loyalty or defense before the damage. Players have no excess damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageResult {
    pub target: Target,
    pub amount: u16,
    pub excess: u32,
}

/// One thing that happened. Each names what it is about by id; a zone change names both the old object and the new
/// one it became (CR 400.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A spell became cast (CR 601.2i), with how many spells its caster has cast this turn, this one included.
    SpellCast { spell: ObjectId, player: PlayerId, nth: u32 },
    /// An object or player became the target of a spell or ability (CR 115.10).
    BecameTarget { target: Target, by: ObjectId, controller: PlayerId },
    /// An object moved between zones. A token's creation comes from no zone (`Zone::GONE`).
    ZoneChange { old: ObjectId, new: ObjectId, from: Zone, to: Zone, reason: ZoneReason },
    /// Damage was dealt. `controller` and `kind.colors()` are the source's as it dealt it (CR 120.3, 609.7): "damage red sources
    /// you controlled dealt this turn" counts a source that has since changed controller or color, or left (Temple of
    /// Power's ruling), and a spell's or an ability's source by what it was on the stack or the battlefield then.
    Damage {
        source: ObjectId, target: Target, amount: u16, kind: DamageKind, controller: PlayerId,
        /// Total excess damage this recipient was dealt by ALL sources in this simultaneous damage event (CR 120.10).
        /// Repeated on each source's record for that recipient: never sum it across those records or attribute it to
        /// that source alone. A trigger about the recipient uses the batch's other records for its source restrictions.
        recipient_excess: u32,
        /// The last-known row that keeps the source as it was when it dealt the damage (`history::DamageDealt::chars`):
        /// "a creature that, at the time it dealt that damage, was an Assassin creature" (freerunning, CR 702.173a) asks
        /// what it was then, whatever it has become or wherever it has gone since.
        known: u32,
    },
    /// Damage that would have been dealt was prevented (CR 615.6, 615.13): `amount` of what `source` would have dealt to
    /// `target`, by the prevention effect of `by` - the permanent whose static ability it is, the source of a resolved
    /// prevention effect, the protected permanent or the source of a player's protection (CR 702.16e). One event each time
    /// a prevention effect is applied to a damage event and prevents some or all of it, before what is left is dealt;
    /// what "if damage is prevented this way" (CR 615.5) would ask. `combat` says whether it would have been combat damage.
    DamagePrevented { source: ObjectId, target: Target, amount: u16, combat: bool, by: ObjectId },
    LifeChange { player: PlayerId, delta: i32 },
    Drew { player: PlayerId, card: ObjectId },
    /// A card was discarded (CR 701.9a): `card` is the object it became where it went. `undefined` when a replacement put it
    /// into a hidden zone instead of its owner's graveyard without its being revealed (Library of Leng): every value of its
    /// characteristics is undefined (CR 701.9c), its abilities among them (CR 109.3), for its own triggers and for anything
    /// that asks what was discarded (`cantrip_mechanics::hand::discarded_card`) - though the card itself, in that zone, is
    /// what it is to a later search or draw.
    Discarded { player: PlayerId, card: ObjectId, undefined: bool },
    /// Cards were revealed to every player (CR 701.20a).
    Revealed { player: PlayerId, card: ObjectId },
    Countered { spell: ObjectId },
    /// An ability was activated (CR 602.2, 605.3): `ability` of `source`, by `player`; `by` names the other object whose effect
    /// gave the source that ability (CR 113.2c, 613.1f: Chromatic Lantern's mana ability on a land, an Equipment's ability on
    /// the creature), with the ability, or is `Grant::NONE` for one of the source's own (`Grant::some` reads it).
    AbilityActivated { source: ObjectId, ability: AbilityRef, player: PlayerId, by: Grant },
    /// A creature was declared as an attacker (CR 508.1). `known` is the last-known row (`GameState::last_known`) that keeps
    /// its characteristics as it was declared, which "you attacked with a Rogue" asks of it once it has changed, left or
    /// changed controllers (`history::Attacked::chars`). `defending` is its defending player as it attacked (CR 508.5,
    /// `Attacker::defending`), which stays its defending player though what it attacks change controller or leave combat.
    Attacks { attacker: ObjectId, defender: Target, known: u32, defending: PlayerId },
    /// An optional cost to attack (CR 508.1g) was paid as `attacker` was declared an attacker (CR 508.1j): the cost of the
    /// `instance`th of the attacker's abilities that are `ability` (counted from 0 in the order it has them; two of one card's
    /// enlist are two instances, CR 702.154d), which is `option`. `tapped` is the creature the cost tapped - for enlist the
    /// creature enlisted (CR 702.154c: `attacker` enlists it) - or `ObjectId::NONE`. Emitted with the declaration's `Attacks`;
    /// the linked "when you do" ability (CR 607.2h, `TriggeredDef::attack_option`) of that instance triggers on it, the core
    /// owning that trigger, and "whenever this creature enlists a creature" reads it.
    PaidToAttack { attacker: ObjectId, option: AttackOption, ability: AbilityRef, instance: u8, tapped: ObjectId },
    /// A Class became a level (CR 716.2a).
    ClassLevel { object: ObjectId, level: u8 },
    /// `player` cycled `card` (CR 702.29c): discarded it to pay the cost of a cycling or typecycling ability (CR 702.29f,
    /// `ActivatedDef::cycling`), and no other "Discard this card" cost (channel). In the discard's batch, after its
    /// `Discarded`, and naming the same object: the card where the discard put it (CR 400.7).
    Cycled { card: ObjectId, player: PlayerId },
    /// A player looked at a card (a look at one card of a hand, at a card exiled face down): kept for the history, and
    /// seen by that player alone.
    Looked { player: PlayerId, card: ObjectId },
    /// A library was searched (CR 701.23).
    /// `player` searched the library of `library` (CR 701.23, 701.23f): their own, or another player's.
    Searched { player: PlayerId, library: PlayerId },
    Tapped { permanent: ObjectId },
    /// A permanent was tapped for mana (CR 106.12, 605.1b): a mana ability with {T} in its cost was activated by `player` and
    /// resolved. `produced` is the mana that ability added (CR 106.12a), of any restriction - not what the triggered mana
    /// abilities it triggers add: "one mana of any type that land produced" reads it. Recorded for the history; the
    /// triggered mana abilities it triggers (`TriggeredDef::mana_ability`) resolve at once, where the tap happens, and only
    /// when it produced some mana (CR 106.12a).
    TappedForMana { permanent: ObjectId, player: PlayerId, produced: ManaTally },
    /// `player` spent `mana` that `source` produced (CR 106.3; `ObjectId::NONE` for a player's own mana ability, CR 605.1a)
    /// to pay a cost of `on` (CR 601.2h, 602.2b, 118.3a): to cast it, a spell on the stack, if `cast`; otherwise an ability
    /// of it, or a payment as something resolves. One record for each source a payment took mana from, recorded as the
    /// payment is made, so a spell's come before its `Cast`: "using mana produced by [this]", "mana from a Treasure was spent
    /// to cast it" (`History::mana_spent_to_cast`). Only recorded.
    ManaSpent { player: PlayerId, source: ObjectId, on: ObjectId, cast: bool, mana: ManaTally },
    /// `creature` convoked `spell` (CR 702.51c): it was tapped to pay for mana in the spell's total cost (CR 702.51a,
    /// 601.2h) as the spell was cast. Recorded as it is tapped, right after its `Tapped`, so a spell's come before its
    /// `Cast`; a tap for mana or for improvise is no convoke. It names the creature and copies nothing out of it: what it
    /// is now, or was as it last existed, is its row (CR 608.2h). Only recorded: "creatures that convoked it" asks
    /// `History::convoked`.
    Convoked { creature: ObjectId, spell: ObjectId },
    /// A player became the monarch (CR 725.1, 725.4): it was not theirs before.
    BecameMonarch { player: PlayerId },
    /// A player took the initiative (CR 726.1), even one who had it (CR 726.5): what makes them venture into Undercity
    /// (CR 726.2).
    TookInitiative { player: PlayerId },
    /// Creatures were declared as attackers (CR 508.1): "whenever you attack" (CR 508.1m). Not for creatures put onto
    /// the battlefield attacking (CR 508.4).
    AttackersDeclared { player: PlayerId, count: u32 },
    /// A creature blocks an attacker (CR 509.1, 509.1g): one event for each blocker and each attacker it blocks, so an
    /// attacker blocked by two sees both. `declared` is the attacker it was declared to block; the other members of that
    /// attacker's band, which it blocks too (CR 702.22h), come as `declared: false`. "Whenever this creature blocks" (CR 509.3a)
    /// reads the declared one alone, triggering once; "whenever it blocks a creature" (CR 509.3b) and "becomes blocked by a
    /// creature" (CR 509.3d) read each.
    Blocks { blocker: ObjectId, attacker: ObjectId, declared: bool },
    /// An attacking creature was not blocked (CR 509.1h): decided as the declare blockers step's blocks are made.
    Unblocked { attacker: ObjectId, defender: Target },
    /// A permanent became monstrous (CR 701.37a).
    Monstrous { permanent: ObjectId },
    /// A Case became solved (CR 719.3a-b): it was not solved before.
    Solved { permanent: ObjectId },
    /// A permanent became suspected (CR 701.60a): it was not suspected before (CR 701.60d).
    Suspected { permanent: ObjectId },
    /// A permanent entered the battlefield face down (CR 708.3: turned face down before it entered; manifest, CR 701.40a,
    /// and manifest dread, CR 701.62a), under `controller`'s control. Recorded with its `ZoneChange`, so "a permanent
    /// entered the battlefield face down under your control this turn" is asked of the history after it has been turned
    /// face up (which ends its face-down status, CR 708.8) or has left, or its control has changed.
    EnteredFaceDown { permanent: ObjectId, controller: PlayerId },
    /// A face-down permanent was turned face up (CR 708.7, 708.8) by `player`: the player who took the special action
    /// (CR 116.2b, 701.40b, 701.58b), or the controller of the spell or ability whose effect turned it up. The same
    /// object, now with its normal characteristics.
    TurnedFaceUp { permanent: ObjectId, player: PlayerId },
    /// A permanent phased out (CR 702.26b), directly or `indirect`ly with what it is attached to (CR 702.26g), under
    /// `controller`'s control. No zone change (CR 702.26d): the same object, which nothing sees until it phases in.
    PhasedOut { permanent: ObjectId, controller: PlayerId, indirect: bool },
    /// A phased-out permanent phased in (CR 702.26c): the same object, which no "enters" ability sees enter (CR 702.26d).
    PhasedIn { permanent: ObjectId },
    StepBegan { step: Step },
    /// `amount` counters of `kind` were put on a permanent (CR 122.6), as many as the replacement effects left (CR 614.1a),
    /// or it entered with them. `total` is how many of that kind it has once they are on it, so "when the fourth plan
    /// counter is put on this" (the counters numbered `total - amount + 1` to `total`) and a Saga's chapters (CR 714.2b)
    /// read the event, never the permanent as it is later. `by` is the player who put them (CR 122.6a), always one: the
    /// controller of the spell or ability whose effect did it, the player whose action or cost it was, the controller of
    /// the source whose damage did it (CR 120.3d), or the controller of a permanent that enters with them.
    CountersPut { object: ObjectId, kind: CounterKind, amount: i16, total: i16, by: PlayerId },
    /// A permanent transformed (CR 701.27a): its other face is up now.
    Transformed { object: ObjectId },
    /// `player` gave a gift (CR 702.174c) to `to`: an instant or sorcery spell they control whose gift cost was paid
    /// resolved, or the gift triggered ability of a permanent they control resolved.
    GaveGift { player: PlayerId, to: PlayerId, gift: Gift },
    /// An Aura or Equipment stopped being attached to `from` (CR 701.3d): moved, unattached, or either one left.
    Unattached { attachment: ObjectId, from: ObjectId },
    /// An Aura, Equipment or Fortification became attached to `to` (CR 701.3a, 301.5, 303.4): an object, or a player an
    /// Aura enchants. Raised wherever it really does - attached by an effect or an equip, reconfigure or bestow ability, moved
    /// onto it from something else (after that move's `Unattached`, CR 701.3d), or entering the battlefield attached to it
    /// (an Aura spell resolving, CR 303.4f and 608.3b; an Aura or Equipment put onto the battlefield or created attached,
    /// CR 303.4f, 301.5e; a bestowed Aura, CR 702.103b), then after its `ZoneChange` and with the new object as
    /// `attachment`. Never for an attach to what it is already attached to, which does nothing (CR 701.3b), nor for
    /// phasing in attached (CR 702.26j). The state is as the attachment left it: "whenever this Equipment becomes attached
    /// to a creature" reads `to` as it is now.
    Attached { attachment: ObjectId, to: Target },
    /// A spell's characteristics as it became cast (CR 601.2i), recorded just before its `SpellCast`: what "the second
    /// creature spell you cast this turn" reads.
    Cast(Cast),
    /// A land was played (CR 305.1, a special action, CR 116.2a): `land` is the permanent it became, `player` who played it,
    /// `way` the face it was played as (0 the card itself, `WAY_BACK_FACE` a modal double-faced card's back face, CR
    /// 712.12), and `granted_by` what let it be played from where it was, if anything beyond the rules did
    /// (`ActionOption::granted_by`: Serra Paragon's "once during each of your turns, you may play a land from your
    /// graveyard", Crucible of Worlds' rule; `Grant::NONE` for nothing, and for a land a resolving effect plays, CR 608.2g;
    /// `Grant::some` reads it). Recorded only, as it moved: `land` may be elsewhere than the battlefield where a
    /// replacement sent it (CR 614.1: Containment Priest's exile), still a land played; its zone change says where it
    /// came from.
    LandPlayed { land: ObjectId, player: PlayerId, way: u8, granted_by: Grant },
    /// A door of a Room was unlocked (CR 709.5f, 709.5h): the permanent was given the unlocked designation of that half
    /// (`DOOR_LEFT` or `DOOR_RIGHT`) by `player`, whether as it entered because the half was cast (CR 709.5d), by the
    /// special action (CR 709.5e) or by an instruction to unlock.
    DoorUnlocked { permanent: ObjectId, door: u8, player: PlayerId },
    /// A spell was copied (CR 707.10): `copy` is the new spell on the stack, controlled by `player`, who made it. It is
    /// not cast (CR 707.10), so it is no `SpellCast`: magecraft's "or copy".
    Copied { copy: ObjectId, original: ObjectId, player: PlayerId },
    /// A player flipped a coin (CR 705.1): won or lost the flip, or, for a flip that cares only whether it comes up heads,
    /// heads or tails, which no player wins or loses (CR 705.2). A flip is a two-sided die, but no die roll.
    CoinFlipped { player: PlayerId, result: CoinFlip },
    /// A player rolled a die with `sides` sides (CR 706.2): the `natural` result, and the `result` once whatever modifies
    /// the roll has been applied (CR 706.4). Both are what the rolled number is, each for the questions that ask for it.
    DieRolled { player: PlayerId, sides: u8, natural: u8, result: i16 },
    /// One of `count` things was chosen at random: the one at `index`. A scenario fixes it like a die of `count`
    /// sides (face `index + 1`).
    ChosenAtRandom { count: u32, index: u32 },
    /// An activated or triggered ability of this object finished resolving (CR 608.2m), or, with `AbilityRef::spell`, this
    /// spell did (CR 608.2, 608.3): a copy too, and a permanent spell once it is on the battlefield; a spell that was countered
    /// or had no legal target did not resolve (CR 608.2b). Paradigm's "the first time a spell you control with this spell's
    /// name has resolved this game" reads it.
    Resolved { source: ObjectId, ability: AbilityRef },
    /// A triggered ability of this object triggered and was put on the stack: what "triggers only N times each turn" counts.
    Triggered { source: ObjectId, ability: AbilityRef },
    /// The modes of a modal spell or ability of this object were chosen (CR 601.2b, 602.2b, 603.3c, 700.2), as a
    /// `ModalDef` mask: what "choose one that hasn't been chosen" asks of the history.
    ModesChosen { source: ObjectId, ability: AbilityRef, modes: u32 },
    /// The player did what an ability says they may do only once each turn (CR 603.2h: "do this only once each turn").
    Done { source: ObjectId, ability: AbilityRef, player: PlayerId },
    /// An effect ended the turn (CR 724.1) of `player`, the player whose turn it was: recorded as the stack is exiled.
    TurnEnded { player: PlayerId },
    /// `player`'s turn began (CR 500.1, 502): recorded only, as its untap step starts. A turn skipped (CR 614.10) never
    /// begins and has none.
    TurnBegan { player: PlayerId },
    /// `player` completed a dungeon (CR 309.7): their venture marker left its bottommost room, by the state-based action
    /// (CR 704.5t) or by venturing on from it (CR 701.49c). Recorded only.
    DungeonCompleted { player: PlayerId },
    /// `player` committed a crime (CR 700.13): as they cast a spell, activated an ability or put a triggered ability on
    /// the stack, it targeted at least one opponent; at least one permanent, spell or ability an opponent controls; and/or
    /// at least one card in an opponent's graveyard. One event for the spell or ability however many of its targets are
    /// crimes, so "whenever you commit a crime" triggers once for it. `by` is the spell, or the source of the ability. A
    /// copy is neither cast nor activated (CR 707.10) and a new target chosen for a spell or ability already on the stack
    /// (CR 115.7) is not choosing it as it is cast, activated or put on the stack: neither commits one.
    CrimeCommitted { player: PlayerId, by: ObjectId },
    /// `player` scried `count` (CR 701.22a), recorded once the process is complete, even if some or all of it was
    /// impossible (CR 701.22d): `looked` is how many cards they looked at, fewer than `count` from a short library. Never
    /// for "scry 0", which is no scry (CR 701.22b).
    Scried { player: PlayerId, count: u32, looked: u32 },
    /// `player` surveilled `count` (CR 701.25a), recorded once the process is complete, even if some or all of it was
    /// impossible (CR 701.25d); `looked` as for `Scried`. Never for "surveil 0" (CR 701.25c).
    Surveilled { player: PlayerId, count: u32, looked: u32 },
    /// `player`'s speed became `speed` (CR 702.179): set to 1 by the state-based action (CR 704.5aa), or increased
    /// (CR 702.179c-d). Never for an increase that changed nothing.
    SpeedChanged { player: PlayerId, speed: u8 },
    /// `player` lost the game (CR 104.3), and why: recorded as they leave it (CR 800.4a).
    GameLost { player: PlayerId, reason: LossReason },
    /// The game ended in a draw (CR 104.4), and why.
    GameDrawn { reason: DrawReason },
    /// A hidden card was named by a question `player` was asked (the cards looked at by a scry or surveil, a library
    /// searched, an opponent's hand chosen from): from then on `player` knows what it is. Recorded only, as the answer
    /// is replayed, so the history says who has seen which hidden card.
    Shown { player: PlayerId, card: ObjectId },
    /// `player`'s library was shuffled (CR 701.24): whatever anyone knew of where its cards were is gone. Recorded only.
    Shuffled { player: PlayerId },
    /// `card` was put on the bottom of `player`'s library in a random order (CR 401.4): nobody knows where among the
    /// others it went. Recorded only.
    Randomized { player: PlayerId, card: ObjectId },
    /// The player who controls `player` changed (CR 723.1): `controller` makes their choices and sees what they see from
    /// now on (723.4, 723.5), or `player` themselves again when `controller` is `player` - the control ended as the next
    /// turn began (723.1) or its controller left the game (CR 800.4a). Recorded only, so the history says who saw which
    /// hidden card when (`GameState::player_controller` is who controls them now).
    PlayerControlled { player: PlayerId, controller: PlayerId },
}

/// Why a player loses the game (CR 104.3): what a "can't lose the game" or "don't lose the game for ..." effect is asked
/// about (Platinum Angel, Phyrexian Unlife), and what the history records of a loss.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LossReason {
    /// 0 or less life (CR 104.3b, 704.5a).
    Life,
    /// Attempted to draw from an empty library (CR 104.3c, 704.5b).
    EmptyLibrary,
    /// Ten or more poison counters (CR 104.3d, 704.5c).
    Poison,
    /// An effect states that the player loses the game (CR 104.3e), or an opponent wins it (CR 104.2b).
    Effect,
    /// The player conceded (CR 104.3a).
    Conceded,
}

/// Why a game is a draw (CR 104.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawReason {
    /// Every player still in the game lost at once (CR 104.4a).
    AllLost,
    /// A loop every player only passed priority in came round to the same position `LOOP_ROUNDS` times (CR 104.4b, 732.4-5;
    /// `docs/rules-exceptions.md`, RX-4).
    MandatoryLoop,
    /// The game reached the most events the engine records (`MAX_GAME_EVENTS`; `docs/rules-exceptions.md`, RX-1).
    EventLimit,
}

impl Event {
    /// The player an event is about, where it is about one.
    pub fn player(&self) -> Option<PlayerId> {
        match *self {
            Event::SpellCast { player, .. }
            | Event::LifeChange { player, .. }
            | Event::Drew { player, .. }
            | Event::Discarded { player, .. }
            | Event::Revealed { player, .. }
            | Event::Looked { player, .. }
            | Event::AbilityActivated { player, .. }
            | Event::Cycled { player, .. }
            | Event::Searched { player, .. }
            | Event::AttackersDeclared { player, .. }
            | Event::TappedForMana { player, .. }
            | Event::ManaSpent { player, .. }
            | Event::BecameMonarch { player }
            | Event::TookInitiative { player }
            | Event::GaveGift { player, .. }
            | Event::Copied { player, .. }
            | Event::DoorUnlocked { player, .. }
            | Event::TurnedFaceUp { player, .. }
            | Event::CoinFlipped { player, .. }
            | Event::DieRolled { player, .. }
            | Event::Done { player, .. }
            | Event::TurnEnded { player }
            | Event::TurnBegan { player }
            | Event::DungeonCompleted { player }
            | Event::Scried { player, .. }
            | Event::Surveilled { player, .. }
            | Event::SpeedChanged { player, .. }
            | Event::CrimeCommitted { player, .. }
            | Event::GameLost { player, .. }
            | Event::Shown { player, .. }
            | Event::Shuffled { player }
            | Event::Randomized { player, .. }
            | Event::PlayerControlled { player, .. } => Some(player),
            Event::Cast(c) => Some(c.player),
            Event::LandPlayed { player, .. } => Some(player),
            Event::BecameTarget { controller, .. } | Event::EnteredFaceDown { controller, .. } => Some(controller),
            Event::Damage { target: Target::Player(p), .. } | Event::DamagePrevented { target: Target::Player(p), .. } => Some(p),
            _ => None,
        }
    }

    /// Whether the event is about `object`: its subject, a zone change's old or new object, a damage event's source or
    /// target.
    pub fn is_about(&self, object: ObjectId) -> bool {
        match *self {
            Event::SpellCast { spell, .. } | Event::Countered { spell } => spell == object,
            Event::Cast(c) => c.spell == object,
            Event::LandPlayed { land, .. } => land == object,
            Event::Copied { copy, original, .. } => copy == object || original == object,
            Event::BecameTarget { target, by, .. } => by == object || target == Target::Object(object),
            Event::ZoneChange { old, new, .. } => old == object || new == object,
            Event::Damage { source, target, .. } => source == object || target == Target::Object(object),
            Event::DamagePrevented { source, target, by, .. } => source == object || by == object || target == Target::Object(object),
            Event::Drew { card, .. } | Event::Discarded { card, .. } | Event::Revealed { card, .. } | Event::Looked { card, .. } | Event::Cycled { card, .. } | Event::Shown { card, .. } | Event::Randomized { card, .. } => {
                card == object
            }
            Event::AbilityActivated { source, .. }
            | Event::Resolved { source, .. }
            | Event::Triggered { source, .. }
            | Event::ModesChosen { source, .. }
            | Event::Done { source, .. } => source == object,
            Event::CrimeCommitted { by, .. } => by == object,
            Event::Attacks { attacker, .. } | Event::Unblocked { attacker, .. } => attacker == object,
            Event::PaidToAttack { attacker, tapped, .. } => attacker == object || tapped == object,
            Event::Blocks { blocker, attacker, .. } => blocker == object || attacker == object,
            Event::ClassLevel { object: o, .. }
            | Event::CountersPut { object: o, .. }
            | Event::Transformed { object: o } => o == object,
            Event::Tapped { permanent }
            | Event::TappedForMana { permanent, .. }
            | Event::Monstrous { permanent }
            | Event::Solved { permanent }
            | Event::Suspected { permanent }
            | Event::EnteredFaceDown { permanent, .. }
            | Event::TurnedFaceUp { permanent, .. }
            | Event::PhasedOut { permanent, .. }
            | Event::PhasedIn { permanent }
            | Event::DoorUnlocked { permanent, .. } => permanent == object,
            Event::Unattached { attachment, from } => attachment == object || from == object,
            Event::Attached { attachment, to } => attachment == object || to == Target::Object(object),
            Event::ManaSpent { source, on, .. } => source == object || on == object,
            Event::Convoked { creature, spell } => creature == object || spell == object,
            Event::LifeChange { .. } | Event::Searched { .. } | Event::AttackersDeclared { .. } | Event::StepBegan { .. } | Event::BecameMonarch { .. } | Event::TookInitiative { .. } | Event::GaveGift { .. } | Event::ChosenAtRandom { .. } | Event::CoinFlipped { .. } | Event::DieRolled { .. } | Event::TurnEnded { .. } | Event::TurnBegan { .. } | Event::DungeonCompleted { .. } | Event::Scried { .. } | Event::Surveilled { .. } | Event::SpeedChanged { .. } | Event::GameLost { .. } | Event::GameDrawn { .. } | Event::Shuffled { .. } | Event::PlayerControlled { .. } => false,
        }
    }
}

/// The kinds of event, as a bitset, so a triggered ability names what it listens for and an event is checked only
/// against the abilities that could match it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EventMask(pub u64);

impl EventMask {
    pub const SPELL_CAST: EventMask = EventMask(1 << 0);
    pub const BECAME_TARGET: EventMask = EventMask(1 << 1);
    pub const ZONE_CHANGE: EventMask = EventMask(1 << 2);
    pub const DAMAGE: EventMask = EventMask(1 << 3);
    pub const STEP_BEGAN: EventMask = EventMask(1 << 4);
    pub const DISCARDED: EventMask = EventMask(1 << 5);
    pub const CYCLED: EventMask = EventMask(1 << 6);
    pub const ATTACKS: EventMask = EventMask(1 << 7);
    pub const CLASS_LEVEL: EventMask = EventMask(1 << 8);
    pub const SEARCHED: EventMask = EventMask(1 << 9);
    pub const DREW: EventMask = EventMask(1 << 10);
    pub const ATTACKERS_DECLARED: EventMask = EventMask(1 << 11);
    pub const UNBLOCKED: EventMask = EventMask(1 << 12);
    pub const MONSTROUS: EventMask = EventMask(1 << 13);
    pub const COUNTERS: EventMask = EventMask(1 << 14);
    pub const TRANSFORMED: EventMask = EventMask(1 << 15);
    pub const UNATTACHED: EventMask = EventMask(1 << 30);
    pub const TAPPED: EventMask = EventMask(1 << 29);
    pub const LIFE_CHANGE: EventMask = EventMask(1 << 16);
    pub const REVEALED: EventMask = EventMask(1 << 17);
    pub const COUNTERED: EventMask = EventMask(1 << 18);
    pub const ABILITY_ACTIVATED: EventMask = EventMask(1 << 19);
    pub const CAST: EventMask = EventMask(1 << 20);
    pub const RESOLVED: EventMask = EventMask(1 << 21);
    pub const TRIGGERED: EventMask = EventMask(1 << 22);
    pub const DONE: EventMask = EventMask(1 << 23);
    pub const BLOCKS: EventMask = EventMask(1 << 24);
    pub const COPIED: EventMask = EventMask(1 << 25);
    pub const BECAME_MONARCH: EventMask = EventMask(1 << 26);
    pub const COIN_FLIPPED: EventMask = EventMask(1 << 27);
    pub const DIE_ROLLED: EventMask = EventMask(1 << 28);
    pub const CHOSEN_AT_RANDOM: EventMask = EventMask(1 << 31);
    pub const LOOKED: EventMask = EventMask(1 << 32);
    pub const MODES_CHOSEN: EventMask = EventMask(1 << 33);
    pub const DOOR_UNLOCKED: EventMask = EventMask(1 << 34);
    pub const GAVE_GIFT: EventMask = EventMask(1 << 35);
    pub const TURN_ENDED: EventMask = EventMask(1 << 36);
    pub const CRIME: EventMask = EventMask(1 << 37);
    pub const SCRIED: EventMask = EventMask(1 << 38);
    pub const SURVEILLED: EventMask = EventMask(1 << 39);
    pub const SOLVED: EventMask = EventMask(1 << 40);
    pub const SUSPECTED: EventMask = EventMask(1 << 41);
    pub const ENTERED_FACE_DOWN: EventMask = EventMask(1 << 42);
    pub const TURNED_FACE_UP: EventMask = EventMask(1 << 43);
    /// Damage was prevented (CR 615.13).
    pub const DAMAGE_PREVENTED: EventMask = EventMask(1 << 44);
    pub const SPEED_CHANGED: EventMask = EventMask(1 << 45);
    pub const MANA_SPENT: EventMask = EventMask(1 << 46);
    /// A player lost the game, or the game was drawn: recorded only, for the history and the host.
    pub const GAME_OVER: EventMask = EventMask(1 << 47);
    /// An Aura, Equipment or Fortification became attached to something (CR 701.3a).
    pub const ATTACHED: EventMask = EventMask(1 << 48);
    /// A land was played (CR 305.1): recorded only, for the history.
    pub const LAND_PLAYED: EventMask = EventMask(1 << 49);
    pub const TOOK_INITIATIVE: EventMask = EventMask(1 << 50);
    /// A turn began (CR 502): recorded only, for the history.
    pub const TURN_BEGAN: EventMask = EventMask(1 << 51);
    /// A dungeon was completed (CR 309.7): recorded only, for the history.
    pub const DUNGEON_COMPLETED: EventMask = EventMask(1 << 52);
    /// What a seat was shown, a library shuffled, a card put on the bottom at random: recorded only, for what each
    /// player knows of the hidden zones.
    pub const SHOWN: EventMask = EventMask(1 << 53);
    pub const SHUFFLED: EventMask = EventMask(1 << 54);
    pub const RANDOMIZED: EventMask = EventMask(1 << 55);
    /// A permanent phased out (CR 702.26): recorded in the history; no ability triggers on it yet.
    pub const PHASED_OUT: EventMask = EventMask(1 << 56);
    /// An optional cost to attack was paid (CR 508.1g, 508.1j).
    pub const PAID_TO_ATTACK: EventMask = EventMask(1 << 57);
    /// A creature convoked a spell (CR 702.51c): recorded only, for the history.
    pub const CONVOKED: EventMask = EventMask(1 << 58);
    /// The player controlling a player changed (CR 723.1): recorded only, for what each player saw.
    pub const PLAYER_CONTROLLED: EventMask = EventMask(1 << 59);

    /// An event's kind as the history files it: every event has one. `of` is what triggers listen for, which leaves
    /// out the records no ability triggers on.
    pub fn of_history(event: &Event) -> EventMask {
        match event {
            Event::LifeChange { .. } => EventMask::LIFE_CHANGE,
            Event::Revealed { .. } => EventMask::REVEALED,
            Event::Looked { .. } => EventMask::LOOKED,
            Event::Countered { .. } => EventMask::COUNTERED,
            Event::AbilityActivated { .. } => EventMask::ABILITY_ACTIVATED,
            Event::CountersPut { .. } => EventMask::COUNTERS,
            Event::Cast(_) => EventMask::CAST,
            Event::LandPlayed { .. } => EventMask::LAND_PLAYED,
            Event::Resolved { .. } => EventMask::RESOLVED,
            Event::Triggered { .. } => EventMask::TRIGGERED,
            Event::ModesChosen { .. } => EventMask::MODES_CHOSEN,
            Event::Done { .. } => EventMask::DONE,
            Event::TurnEnded { .. } => EventMask::TURN_ENDED,
            Event::TurnBegan { .. } => EventMask::TURN_BEGAN,
            Event::DungeonCompleted { .. } => EventMask::DUNGEON_COMPLETED,
            Event::GameLost { .. } | Event::GameDrawn { .. } => EventMask::GAME_OVER,
            Event::ManaSpent { .. } => EventMask::MANA_SPENT,
            Event::Convoked { .. } => EventMask::CONVOKED,
            Event::Shown { .. } => EventMask::SHOWN,
            Event::Shuffled { .. } => EventMask::SHUFFLED,
            Event::Randomized { .. } => EventMask::RANDOMIZED,
            Event::PhasedOut { .. } => EventMask::PHASED_OUT,
            Event::PlayerControlled { .. } => EventMask::PLAYER_CONTROLLED,
            other => EventMask::of(other),
        }
    }

    /// The events a triggered ability can look back on (CR 603.10, `CardTable::looks_back`): every kind `looks_back` says
    /// yes to, so an ability listening for one is among the permanents taken as they were before (`settle::Then`). Phasing
    /// out and losing the game are filed in the history only (`of_history`); no ability triggers on them yet.
    pub const LOOKS_BACK: EventMask = EventMask(
        EventMask::ZONE_CHANGE.0 | EventMask::UNATTACHED.0 | EventMask::COUNTERED.0 | EventMask::PHASED_OUT.0 | EventMask::GAME_OVER.0,
    );

    pub fn of(event: &Event) -> EventMask {
        match event {
            Event::SpellCast { .. } => EventMask::SPELL_CAST,
            Event::BecameTarget { .. } => EventMask::BECAME_TARGET,
            Event::ZoneChange { .. } => EventMask::ZONE_CHANGE,
            Event::Damage { .. } => EventMask::DAMAGE,
            Event::StepBegan { .. } => EventMask::STEP_BEGAN,
            Event::Discarded { .. } => EventMask::DISCARDED,
            Event::Cycled { .. } => EventMask::CYCLED,
            Event::Attacks { .. } => EventMask::ATTACKS,
            Event::PaidToAttack { .. } => EventMask::PAID_TO_ATTACK,
            Event::ClassLevel { .. } => EventMask::CLASS_LEVEL,
            Event::Searched { .. } => EventMask::SEARCHED,
            Event::Drew { .. } => EventMask::DREW,
            Event::AttackersDeclared { .. } => EventMask::ATTACKERS_DECLARED,
            Event::Unblocked { .. } => EventMask::UNBLOCKED,
            Event::Blocks { .. } => EventMask::BLOCKS,
            Event::Monstrous { .. } => EventMask::MONSTROUS,
            Event::Solved { .. } => EventMask::SOLVED,
            Event::Suspected { .. } => EventMask::SUSPECTED,
            Event::EnteredFaceDown { .. } => EventMask::ENTERED_FACE_DOWN,
            Event::TurnedFaceUp { .. } => EventMask::TURNED_FACE_UP,
            Event::CountersPut { .. } => EventMask::COUNTERS,
            Event::Transformed { .. } => EventMask::TRANSFORMED,
            Event::Unattached { .. } => EventMask::UNATTACHED,
            Event::Attached { .. } => EventMask::ATTACHED,
            Event::Tapped { .. } => EventMask::TAPPED,
            Event::BecameMonarch { .. } => EventMask::BECAME_MONARCH,
            Event::TookInitiative { .. } => EventMask::TOOK_INITIATIVE,
            Event::CoinFlipped { .. } => EventMask::COIN_FLIPPED,
            Event::DieRolled { .. } => EventMask::DIE_ROLLED,
            Event::ChosenAtRandom { .. } => EventMask::CHOSEN_AT_RANDOM,
            Event::LifeChange { .. } => EventMask::LIFE_CHANGE,
            Event::Countered { .. } => EventMask::COUNTERED,
            Event::Copied { .. } => EventMask::COPIED,
            // "Whenever you activate a loyalty ability", "whenever a player activates an ability that isn't a mana ability"
            // (CR 602.2, 603.2): an activation, once the ability is on the stack and its costs are paid, is an event abilities
            // trigger on, so they go on the stack above it and resolve first. Only activations of abilities that use the stack
            // are emitted. A mana ability's activation (CR 605.3) is only recorded: no card here triggers on one yet (those
            // that would, "whenever you activate an ability of a creature" - Elrond, Moon-Reader, Ceaseless Searblades - wait
            // for it to be emitted too). The marker kept for the history (an extra loyalty activation allowed) is only
            // recorded, and a special action (CR 116.2) or a copy of an ability (CR 707.10) is
            // no activation.
            Event::AbilityActivated { .. } => EventMask::ABILITY_ACTIVATED,
            Event::DoorUnlocked { .. } => EventMask::DOOR_UNLOCKED,
            Event::GaveGift { .. } => EventMask::GAVE_GIFT,
            Event::CrimeCommitted { .. } => EventMask::CRIME,
            Event::Scried { .. } => EventMask::SCRIED,
            Event::Surveilled { .. } => EventMask::SURVEILLED,
            Event::DamagePrevented { .. } => EventMask::DAMAGE_PREVENTED,
            Event::SpeedChanged { .. } => EventMask::SPEED_CHANGED,
            _ => EventMask(0),
        }
    }

    pub fn intersects(self, other: EventMask) -> bool {
        self.0 & other.0 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_kind_preserves_every_color_set_and_combat_designation() {
        for bits in 0..32 {
            for combat in [false, true] {
                let kind = DamageKind::new(combat, ColorSet(bits));
                assert_eq!(kind.is_combat(), combat);
                assert_eq!(kind.colors(), ColorSet(bits));
            }
        }
        assert_eq!(std::mem::size_of::<DamageKind>(), 1);
    }

    /// The history holds an `Event` for everything since the game began, so its size is every record's: the mana in one
    /// is a `ManaTally` (a `u8` a kind), never a pool's `u32` counts, and a cast's mana value is in its last-known row
    /// (`Cast::value`). 28 since a damage event names the last-known row of its source as it dealt it (`Event::Damage::known`,
    /// freerunning's "at the time it dealt that damage", CR 702.173a): the four bytes of a row's index, which no reshuffle of
    /// the damage event's other fields could find room for.
    #[test]
    fn an_event_stays_small() {
        assert!(std::mem::size_of::<Event>() <= 28, "Event is {} bytes", std::mem::size_of::<Event>());
    }
}
