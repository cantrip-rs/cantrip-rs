//! Decisions are data (docs/engine-design.md, "Decisions are data"): what the engine is in the middle of is a
//! stack of frames in the state, and a question waiting for an answer is a frame holding the typed request.
//!
//! The requests mirror the C# engine's prompts kind for kind and option for option, in the same order, because the
//! protocol the clients speak and the scenarios' "the first one offered" both depend on it.

use crate::object::*;
use crate::vocab::*;

/// One unit of work in progress. The engine runs the top frame; a frame that asks a question stops there, holding the
/// request, and runs again from its start when the answer arrives, with its answers so far replayed (or, for a frame
/// written as explicit stages, from the stage it stopped at).
#[derive(Clone, Debug)]
pub struct Frame {
    pub kind: FrameKind,
    /// The answers this run of the frame has been given, in the order it asked, each with the player who gave it where the
    /// engine knows it (a test may give answers with none): a replayed answer is only ever the answer of the player it is
    /// asked of.
    pub answers: Vec<(Option<PlayerId>, Answer)>,
    /// Players who conceded while this run was waiting (CR 104.3a), each with the number of answers it had been given then:
    /// every replay of the run leaves the game for them at that point, after the work done before it (`Game::concede`).
    pub conceded: Vec<(u32, PlayerId)>,
    /// For a staged frame whose question was asked of a player who has left the game, while the controller of the object it
    /// is for chooses who makes it instead (CR 800.4g): the stage the question continues at, and the question.
    pub stand_in: Option<Box<(u16, RequestKind)>>,
    /// The question it is waiting on.
    pub pending: Option<Request>,
    /// Where the journal stood when this run of the frame began: what an answer rolls back to before the re-run.
    /// Bookkeeping of the run, not of the game: written without the journal, and no part of the position.
    pub mark: u32,
    /// Where the game log stood then, for the same reason.
    pub log_mark: u32,
    /// For a frame written as explicit stages: the stage to continue from, and values kept between stages. A staged
    /// frame is not rolled back when answered.
    pub stage: u16,
    /// The stage a staged frame's pending question continues at.
    pub next_stage: u16,
    pub staged: bool,
    pub scratch: [u32; 4],
    /// For a staged frame waiting on a question: a change with no event an earlier stage made that the state triggers have
    /// not judged yet (CR 603.8). The resolution goes on with it when answered, as a run never stopped would
    /// (`cantrip_rules::StateCheck`).
    pub unjudged: bool,
}

impl Frame {
    pub fn new(kind: FrameKind) -> Frame {
        Frame {
            kind,
            answers: Vec::new(),
            conceded: Vec::new(),
            stand_in: None,
            pending: None,
            mark: 0,
            log_mark: 0,
            stage: 0,
            next_stage: 0,
            staged: false,
            scratch: [0; 4],
            unjudged: false,
        }
    }

    #[allow(clippy::type_complexity)]
    fn position(
        &self,
    ) -> (&FrameKind, (&Vec<(Option<PlayerId>, Answer)>, &Vec<(u32, PlayerId)>, &Option<Box<(u16, RequestKind)>>), Option<(&PlayerId, &RequestKind)>, u16, u16, (bool, bool), &[u32; 4]) {
        (
            &self.kind,
            (&self.answers, &self.conceded, &self.stand_in),
            self.pending.as_ref().map(|r| (&r.player, &r.kind)),
            self.stage,
            self.next_stage,
            (self.staged, self.unjudged),
            &self.scratch,
        )
    }
}

/// Two frames are the same work in progress whatever bookkeeping their runs carry: the marks and request ids.
impl PartialEq for Frame {
    fn eq(&self, other: &Frame) -> bool {
        self.position() == other.position()
    }
}

impl Eq for Frame {}

impl std::hash::Hash for Frame {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.position().hash(state);
    }
}

/// What a frame is doing. Only the core's procedures are here: card behavior runs inside them, by function index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameKind {
    /// The game's start (CR 103): who goes first, opening hands, mulligans.
    Opening,
    /// The current step's turn-based actions (CR 500.2), then priority if the step has it.
    BeginStep,
    /// A player has priority (CR 117).
    Priority,
    /// State-based actions until none apply, then triggered abilities onto the stack (CR 117.5).
    Settle,
    /// Taking an action a player chose with priority: playing a land, casting a spell (CR 601.2), activating an
    /// ability (CR 602.2). The action is the frame's first answer.
    Act { player: PlayerId },
    /// Resolving the top of the stack (CR 608).
    Resolve,
}

/// A question for one player.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Request {
    /// Never reused within a game.
    pub id: u32,
    pub player: PlayerId,
    pub kind: RequestKind,
    /// Asked while a spell or ability is still being announced, so the player may give it up (CR 733.1).
    pub may_cancel: bool,
}

/// The kinds of action an option can be, as the C# engine numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionKind {
    PlayLand,
    CastSpell,
    ActivateAbility,
    CancelCasting,
    /// Pay an "unless" cost now that the pool covers it.
    PayCost,
    DeclineCost,
    /// Put a unit of pool mana of this kind towards the cost.
    AssignMana,
    ConfirmCasting,
    /// While announcing (CR 601.2b): one more for X, or one more time kicked.
    IncreaseX,
    DecreaseX,
    /// While paying (CR 601.2h): tap this permanent to pay for convoke (CR 702.51a) or improvise (CR 702.126a); the
    /// option's `index` is which of `PAY_*` it pays for.
    TapToPay,
    /// While paying (CR 601.2h): exile this card from the graveyard to pay for delve (CR 702.66a).
    ExileToPay,
}

/// Which way of paying with something other than mana a `TapToPay` or `ExileToPay` option is (its `index`).
/// The `index` of an `ActivateAbility` option that is a mana ability of the player's own (CR 605.1a, 611.2a) and not of any
/// object: the nth effect that gave them one, added to this. Its `source` is the card that gave it, wherever it is now.
pub const PLAYER_MANA_ABILITY: u8 = 200;

pub const PAY_DELVE: u8 = 0;
pub const PAY_IMPROVISE: u8 = 1;
pub const PAY_CONVOKE: u8 = 2;
/// Waterbend (CR 701.67a): an untapped artifact or creature tapped pays {1}.
pub const PAY_WATERBEND: u8 = 3;

/// One thing a player could do: an option of a priority, payment or "unless" question, as a prototype the answer
/// fills in (targets, a color).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionOption {
    pub kind: ActionKind,
    pub source: Option<ObjectId>,
    /// Which of the source's own options it is: for a cast or a land play, its way number (the face it casts and the cost
    /// of the card's own it is cast for: 0 its mana cost, n its nth printed alternative way, `WAY_ADVENTURE` and the rest);
    /// for an activation, which ability of the source's computed ability list. Never a position in a list another object
    /// makes: what another object adds is `granted_by` and `paid_by`.
    pub index: u8,
    /// What made the action possible, where the rules every player has do not (CR 601.3, 305.1): the permission a cast or a
    /// land play is made by, from a zone other than the hand (Lurrus of the Dream-Den, Mystic Forge, Snapcaster Mage's
    /// grant, Yawgmoth's Will, retrace). Each permission that would allow a cast is an option of its own, so the player
    /// chooses which one is used (CR 601.2b), and the cast is made by that one, on its terms, and recorded with it.
    pub granted_by: Option<Grant>,
    /// The alternative cost another object gives that the cast pays rather than its mana cost (CR 118.9, 601.2b): Jodah,
    /// Archmage Eternal's, Omniscience's, Primal Prayers'. None for a cost of the card's own (its mana cost, evoke) or one
    /// the permission names.
    pub paid_by: Option<Grant>,
    /// Targets the action is announced with, and who may be chosen for them (CR 601.2c): the candidates of every
    /// target slot together.
    pub targets_required: u32,
    pub targets_optional: bool,
    pub target_candidates: Vec<Target>,
    /// A mana ability's colors to choose from, when it adds one mana of any one color (CR 106.5).
    pub color_choices: ColorSet,
    /// A mana ability whose sacrificed permanent sets the mana (CR 106.7) may sacrifice one that could produce none:
    /// then it is activated naming no type, and adds no mana.
    pub names_nothing: bool,
    /// For `AssignMana`: the kind of mana.
    pub mana: Option<Mana>,
    /// For `AssignMana`: the restriction on the mana (CR 106.6), if it has one. For a mana ability: the restriction on
    /// the mana it adds.
    pub restriction: Option<ManaRestriction>,
    /// For a mana ability offered while a cost is paid: the mana it adds is restricted so that none of it may pay this
    /// cost (CR 106.6). Activating it is still a choice the player has (CR 601.2g, 605.3a); it only fills the pool.
    pub pays_nothing: bool,
}

impl ActionOption {
    pub fn simple(kind: ActionKind) -> ActionOption {
        ActionOption {
            kind,
            source: None,
            index: 0,
            granted_by: None,
            paid_by: None,
            targets_required: 0,
            targets_optional: false,
            target_candidates: Vec::new(),
            color_choices: ColorSet::NONE,
            names_nothing: false,
            mana: None,
            restriction: None,
            pays_nothing: false,
        }
    }

    pub fn on(kind: ActionKind, source: ObjectId, index: u8) -> ActionOption {
        ActionOption { source: Some(source), index, ..ActionOption::simple(kind) }
    }
}

/// A taken action: an option, filled in.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Action {
    pub kind: ActionKind,
    pub source: Option<ObjectId>,
    pub index: u8,
    pub granted_by: Option<Grant>,
    pub paid_by: Option<Grant>,
    pub targets: Vec<Target>,
    pub color: Option<Color>,
    pub mana: Option<Mana>,
    pub restriction: Option<ManaRestriction>,
}

impl Action {
    pub fn of(option: &ActionOption) -> Action {
        Action {
            kind: option.kind,
            source: option.source,
            index: option.index,
            granted_by: option.granted_by,
            paid_by: option.paid_by,
            targets: Vec::new(),
            color: None,
            mana: option.mana,
            restriction: option.restriction,
        }
    }
}

/// What a chosen card name must be the name of (CR 201.4a): "a card name" (any card), "a nonland card name" (Cabal
/// Therapy, Tamiyo, Collector of Tales), "a land card name" (Petrified Hamlet). The name's own card is what is judged, a
/// back face by the back face's characteristics alone (CR 201.4d).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NameKind {
    #[default]
    Any,
    Nonland,
    Land,
}

impl NameKind {
    /// Whether the name of a card that is (`land`) or is not a land may be chosen.
    pub fn allows(self, land: bool) -> bool {
        match self {
            NameKind::Any => true,
            NameKind::Nonland => !land,
            NameKind::Land => land,
        }
    }

    /// The words a card uses for the name it asks for, before "card name": "", "nonland", "land".
    pub fn word(self) -> &'static str {
        match self {
            NameKind::Any => "",
            NameKind::Nonland => "nonland",
            NameKind::Land => "land",
        }
    }
}

/// Who makes a choice asked of a player who has left the game (CR 800.4f-h): `RequestKind::stand_in`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StandIn {
    /// A cost, or whether to pay one: it is not paid (CR 800.4f).
    Unpaid,
    /// One of the player's own game actions (priority, attacks, blocks, the discard to hand size, a mulligan, the order of
    /// their triggers, combat damage): it ends with them, and no one makes it (CR 800.4a, 800.4d, 800.4j). The procedure
    /// asking it reads the player as gone (`Cx::ask_rule`).
    NotAsked,
    /// This player makes the choice: the one another object's controller is left to choose among, or the next player in turn
    /// order for a choice a rule asks (CR 800.4g-h).
    By(PlayerId),
    /// The controller of the object the choice is for chooses which of these players makes it (CR 800.4g).
    Chosen { controller: PlayerId, source: ObjectId, candidates: Vec<PlayerId> },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RequestKind {
    /// Act or pass (CR 117). Passing is always legal.
    Priority { options: Vec<ActionOption> },
    /// Pay what is still owed on a spell or ability (CR 601.2g-h): activate a mana ability, put pool mana towards it,
    /// confirm once paid, or give up.
    PayCosts {
        source: ObjectId,
        total: ManaCost,
        remaining: ManaCost,
        options: Vec<ActionOption>,
        /// Announcing X or how many times it is kicked (CR 601.2b), with the value so far, rather than paying.
        announcing: bool,
        x: u32,
        /// While announcing: the largest value that may be announced (`Answer::Announce`). Zero while paying.
        most: u32,
        /// The X being announced is a "−X" loyalty cost's (CR 606.4, 107.3): it removes that many loyalty counters from
        /// `source`, up to the loyalty it has, and costs no mana.
        loyalty: bool,
        /// The creatures whose cost to attack this pays (CR 508.1h-j), when it is one: `source` is then the permanent
        /// whose static ability makes attacking cost something (the first, when several do). Empty for anything else.
        attackers: Vec<ObjectId>,
        /// The life the total cost asks besides its mana (CR 119.4, 601.2h: "an additional 3 life", "pay X life"), paid with it
        /// once the mana is. Zero while X is announced, and for anything but a spell.
        life: u32,
    },
    /// A cost paid as a spell or ability resolves (CR 118.12, 608.2d): "[a player] may pay [cost]. If they do, ...", and
    /// "... unless [a player] pays [cost]" (CR 118.12a). One payment of the whole cost, `total` mana and `life` life:
    /// activate mana abilities while there is mana to pay (CR 605.3a), pay once the pool covers the mana and the life
    /// total covers the life (CR 118.3, 119.4), or decline. `question` is the card's words for what paying does, with
    /// `{cost}` and `{subject}` where the cost and the subject's name go; its verb is what the bots read it by.
    /// `remaining` is the mana the pool does not cover yet, and `life_payable` whether the life total still covers the
    /// life (false once a mana ability has cost the life the payment needed). `x` is the X chosen so far when the cost is
    /// "{X}" and the payer chooses X as they pay it ("you may pay {X}", CR 107.3f): `total` is then the cost at that X (its
    /// {X} as that much generic mana), `IncreaseX` and
    /// `DecreaseX` among the options change it, and paying pays that X. `None` for a cost with no X.
    UnlessPays {
        source: ObjectId,
        subject: ObjectId,
        total: ManaCost,
        remaining: ManaCost,
        life: u32,
        life_payable: bool,
        question: &'static Question,
        options: Vec<ActionOption>,
        x: Option<u16>,
    },
    /// Choose cards from among `cards`, every one of which the player sees: which sets of them are an answer is `rule`'s to
    /// say (`Selection`; the core checks every answer with it), and what may be chosen next as the player picks is asked of
    /// it too. `rule.kind` is what the asking card calls what may be chosen when it says more than "card" ("artifact or
    /// creature", "nonland"): the dock words the choice as the card does.
    ChooseCards {
        source: ObjectId,
        purpose: Purpose,
        cards: Vec<ObjectId>,
        /// Boxed, so that a question of another kind is no bigger for it.
        rule: Box<Selection>,
    },
    /// `least` to `count` targets are chosen (a range, "one or two target creatures", CR 601.2c); `optional` is `least == 0`.
    /// Targets for an ability as it goes on the stack, or any other "choose a [thing]" the C# engine asks as targets.
    /// `label` is what the slot is for as its card words it (`TargetSpec::label`, empty when it has nothing to add), and
    /// `slot_player` the opponent whose slot it is when the ability has one for each opponent. `total`, when the targets
    /// chosen together are limited to a total mana value ("any number of target creature cards with total mana value X or
    /// less", `TargetSpec::total_mana_value`), is that limit, its values index for index with `candidates`: the core refuses
    /// a choice whose mana values add up to more.
    ChooseTarget { source: ObjectId, purpose: Purpose, candidates: Vec<Target>, optional: bool, least: u32, count: u32, any_number: bool, label: &'static str, slot_player: Option<PlayerId>, total: Option<TotalAtMost> },
    /// One or more of these labelled options (a pile, a mode), by index.
    /// `count` to `at_most` of them when `at_most` is more than `count` (zero `count` is "up to"). `affordable` is
    /// the eligible modes whose own cost the chooser could pay as well, advice for a default; `additional_cost`
    /// marks choices that are costs (Phyrexian mana paid with life, CR 107.4f) rather than modes; `repeat` lets one
    /// mode be chosen more than once (CR 700.2d).
    ChooseModes { source: ObjectId, modes: Vec<Label>, eligible: Vec<u8>, count: u8, at_most: u8, affordable: Vec<u8>, additional_cost: bool, repeat: bool },
    /// `purpose` says what the colors are for, which a player or a bot choosing them reads (`ColorUse`).
    ChooseColors { source: ObjectId, choices: ColorSet, count: u8, purpose: ColorUse },
    /// A card name (CR 201.3, 614.12a): any card the game knows. `looked_at` are the cards the player has just been
    /// shown, and `suggestion` the name an unscripted player takes (the costliest nonland card among them).
    /// `kind` is what the name must be the name of (CR 201.4a): any card, a nonland card or a land card.
    ChooseName { source: ObjectId, looked_at: Vec<ObjectId>, suggestion: Option<CardId>, kind: NameKind },
    /// One subtype of `choices`, in order ("choose a creature type", CR 205.3m: the C# engine asks it as a modes
    /// question over every creature type, more than a mode index holds).
    ChooseSubtype { source: ObjectId, choices: Vec<Subtype>, purpose: WordUse },
    /// "You may".
    May { source: ObjectId, purpose: Purpose },
    /// A player chosen without targeting (CR 608.2d, 614.12a): True-Name Nemesis's "as this enters, choose a player",
    /// Baleful Mastery's "an opponent draws a card". Answered with `Answer::Player`. `verb` and `amount` say what the
    /// chosen player will do, as hint text.
    ChoosePlayer { source: ObjectId, candidates: Vec<PlayerId>, verb: &'static str, amount: u16 },
    /// Tap any number of untapped creatures with total power `minimum` or more (CR 702.122a crew, CR 702.171a saddle):
    /// any subset of `cards` whose `powers` (what each counts as towards the total, CR 702.122b) add up to it. A negative
    /// power is added as it is (CR 107.1b), taking from the total.
    TapForPower { source: ObjectId, purpose: Purpose, cards: Vec<ObjectId>, powers: Vec<i16>, minimum: u8 },
    /// Collect evidence N (CR 701.59a): exile any subset of `cards` (the player's graveyard) whose `values` (their mana
    /// values) add up to `minimum` or more.
    CollectEvidence { source: ObjectId, cards: Vec<ObjectId>, values: Vec<u8>, minimum: u8 },
    /// Craft's materials (CR 702.167a): exile `minimum` to `maximum` of `cards`, which are permanents the player controls
    /// and cards in their graveyard, each one of what `quality` names ("artifact", "creature"). An exact number of
    /// materials has `minimum == maximum`; "one or more" has `maximum` of every card offered.
    Craft { source: ObjectId, cards: Vec<ObjectId>, minimum: u32, maximum: u32, quality: &'static str },
    /// Remove `total` counters from among permanents (CR 118.3, 122.1): "remove three counters from among creatures you
    /// control" (Dawnhand Dissident; Tayam, Luminous Enigma). Each of `pools` is the counters of one kind on one permanent,
    /// with how many there are; any of them may be taken, of any kind and from any of the permanents, adding up to `total`.
    /// Answered with `Answer::Counters`.
    ///
    /// With `up_to`, an effect's "remove up to `total` counters from target creature" (Heartless Act, Hex Parasite) as it
    /// resolves (CR 608.2d, 122.1): any number of them from none to `total`, of whichever kinds the player chooses. Without
    /// it, exactly `total` (a cost, CR 118.3, or an effect that removes a fixed number).
    RemoveCounters { source: ObjectId, pools: Vec<CounterPool>, total: u8, up_to: bool },
    /// The targets of a slot that divides a total among them, and each one's share (CR 601.2d): "distribute two +1/+1
    /// counters among up to two target creatures". At most `max_targets` of `candidates` (any number when 0), each with a
    /// share of at least 1, adding up to `total`; none at all when the slot is `optional`.
    ChooseDividedTargets { source: ObjectId, candidates: Vec<Target>, total: u32, max_targets: u32, optional: bool },
    /// Declare attackers (CR 508.1): any of `eligible`, each at one of `players`; every one of `requirements` must
    /// attack (CR 508.1d).
    /// Who attacks what (CR 508.1): each attacker names a player, a planeswalker a defending player controls, or a battle a defending player protects (`battles`,
    /// CR 310.9b, 508.1b).
    /// `costs` is what each attack that costs something to declare costs (CR 508.1d, 508.1h: "creatures can't attack you
    /// unless their controller pays {2} for each ..."), one entry for each attacker and what it would attack, empty when
    /// nothing costs anything; the declaration's total is what its attacks' entries add up to, paid once it is declared.
    DeclareAttackers {
        eligible: Vec<ObjectId>,
        players: Vec<PlayerId>,
        planeswalkers: Vec<ObjectId>,
        battles: Vec<ObjectId>,
        requirements: Vec<ObjectId>,
        costs: Vec<AttackCost>,
        /// Advice while `costs` has any: the most mana the attacking player could pay now (their pool and what their mana
        /// abilities add, `u16::MAX` when that is not known), counting creatures that may yet be tapped by attacking.
        can_pay: u16,
    },
    /// How creatures divide their combat damage (CR 510.1c-d): one division per creature with a choice.
    AssignCombatDamage { divisions: Vec<DamageDivision> },
    /// Declare blockers (CR 509.1): `allowed` lists every (blocker, attacker) pair the rules allow, and `requirements` the
    /// blocks effects require if able (CR 509.1c).
    DeclareBlockers { eligible: Vec<ObjectId>, attackers: Vec<ObjectId>, allowed: Vec<(ObjectId, ObjectId)>, requirements: BlockRequirements },
    /// Discard down to maximum hand size (CR 514.1).
    DiscardToHandSize { count: u32, hand: Vec<ObjectId> },
    /// Keep the opening hand or take a mulligan (CR 103.5).
    Mulligan { mulligans: u8 },
    /// Who takes the first turn (CR 103.1).
    ChooseStartingPlayer { candidates: Vec<PlayerId> },
    /// The order a player's triggered abilities go on the stack (CR 603.3b): a permutation, first on the stack first.
    OrderTriggers { sources: Vec<ObjectId>, abilities: Vec<AbilityRef> },
    /// Cards looked at, each to be put in one of the destinations (Ponder, surveil, "put one into your hand and the
    /// rest into your graveyard"). `cards` are top of the library first.
    DistributeCards { source: ObjectId, cards: Vec<ObjectId>, destinations: Vec<Destination>, may_shuffle: bool },
    /// Search a library (CR 701.23): `library` is the whole library, which the searcher sees, and `rule` which sets of its
    /// cards the search may find (`Selection`: how many, of what quality, CR 701.23b). `destination` is where the searcher
    /// is told the cards found are going; whose library it is, is the owner of the cards in `library`.
    SearchLibrary { source: ObjectId, library: Vec<ObjectId>, rule: Box<Selection>, destination: SearchDestination },
    /// Which of several replacement effects applies to an event first (CR 616.1), asked of the affected player:
    /// `sources` are the objects whose effects apply and `hints` say what each does, in the order they are offered.
    /// `subject` is the object the event is about, if it is about one; `life`, for an event that changes the affected
    /// player's life total (CR 119.3), how much: a gain above zero, a loss below; `damage`, for damage about to be dealt,
    /// what it is as the effects applied so far have left it (whom it would now be dealt to, CR 614.6); `turn`, whether the
    /// event is the beginning of the asked player's turn, which effects would skip (CR 614.10); `counters`, for counters
    /// about to be put on `subject` (CR 122.6), how many of each kind as the effects so far have left them; `draw`, whether
    /// the event is a card the asked player would draw (CR 121.6).
    ChooseReplacement {
        subject: Option<ObjectId>,
        life: Option<i32>,
        sources: Vec<ObjectId>,
        hints: Vec<&'static str>,
        damage: Option<DamageAbout>,
        turn: bool,
        counters: Vec<(CounterKind, i16)>,
        draw: bool,
    },
}

impl RequestKind {
    /// A choice of `count` of `eligible` (or up to `count` with `up_to`) from among `cards`, called `kind`: the shape the
    /// core's costs and turn-based choices ask, as a `ChooseCards` with that rule (`Selection::listed`).
    pub fn choose_cards(source: ObjectId, purpose: Purpose, cards: Vec<ObjectId>, eligible: Vec<ObjectId>, count: u32, up_to: bool, kind: &'static str) -> RequestKind {
        let count = if up_to { Count::UpTo(count) } else { Count::Exactly(count) };
        let rule = Selection::listed(count, &cards, eligible).called(kind);
        RequestKind::ChooseCards { source, purpose, cards, rule: Box::new(rule) }
    }

    /// The object a question is for: the spell, ability or permanent whose text asks it. None for a choice the rules
    /// themselves ask: attacks, blocks, combat damage, the discard to hand size, mulligans, who goes first, the order of
    /// triggers, which replacement effect applies first.
    pub fn source(&self) -> Option<ObjectId> {
        use RequestKind as K;
        match self {
            K::PayCosts { source, .. }
            | K::UnlessPays { source, .. }
            | K::ChooseCards { source, .. }
            | K::ChooseTarget { source, .. }
            | K::ChooseModes { source, .. }
            | K::ChooseColors { source, .. }
            | K::ChooseName { source, .. }
            | K::ChooseSubtype { source, .. }
            | K::May { source, .. }
            | K::ChoosePlayer { source, .. }
            | K::TapForPower { source, .. }
            | K::CollectEvidence { source, .. }
            | K::Craft { source, .. }
            | K::RemoveCounters { source, .. }
            | K::ChooseDividedTargets { source, .. }
            | K::DistributeCards { source, .. }
            | K::SearchLibrary { source, .. } => Some(*source),
            K::Priority { .. }
            | K::DeclareAttackers { .. }
            | K::AssignCombatDamage { .. }
            | K::DeclareBlockers { .. }
            | K::DiscardToHandSize { .. }
            | K::Mulligan { .. }
            | K::ChooseStartingPlayer { .. }
            | K::OrderTriggers { .. }
            | K::ChooseReplacement { .. } => None,
        }
    }

    /// Whether the question is the paying of a cost, or whether to pay one (CR 118, 800.4f): the costs of a spell or ability
    /// being paid, a cost paid as one resolves ("unless [a player] pays", "[a player] may pay"), and a cost's tapping for
    /// power, collecting evidence and crafting.
    pub fn is_cost(&self) -> bool {
        matches!(
            self,
            RequestKind::PayCosts { .. }
                | RequestKind::UnlessPays { .. }
                | RequestKind::TapForPower { .. }
                | RequestKind::CollectEvidence { .. }
                | RequestKind::Craft { .. }
        )
    }

    /// Every object the request names, which its player must be able to see while it is outstanding: a library being
    /// searched, the cards looked at, an opponent's hand taken from (the host shows them; the rules record `Shown`).
    pub fn objects(&self, f: &mut impl FnMut(ObjectId)) {
        let mut add = |ids: &[ObjectId]| ids.iter().for_each(|&id| f(id));
        fn options(add: &mut dyn FnMut(&[ObjectId]), options: &[ActionOption]) {
            for o in options {
                if let Some(s) = o.source {
                    add(&[s]);
                }
                add(&o.target_candidates.iter().filter_map(|t| t.object()).collect::<Vec<_>>());
            }
        }
        match self {
            RequestKind::Priority { options: o } => options(&mut add, o),
            RequestKind::UnlessPays { options: o, source, subject, .. } => {
                options(&mut add, o);
                add(&[*source, *subject]);
            }
            RequestKind::PayCosts { source, options: o, attackers, .. } => {
                add(&[*source]);
                add(attackers);
                options(&mut add, o);
            }
            RequestKind::ChooseCards { source, cards, rule, .. } => {
                add(&[*source]);
                add(cards);
                add(rule.among.as_deref().unwrap_or(&[]));
            }
            RequestKind::ChooseTarget { source, candidates, .. } | RequestKind::ChooseDividedTargets { source, candidates, .. } => {
                add(&[*source]);
                add(&candidates.iter().filter_map(|t| t.object()).collect::<Vec<_>>());
            }
            RequestKind::ChooseModes { source, .. }
            | RequestKind::ChooseColors { source, .. }
            | RequestKind::ChooseSubtype { source, .. }
            | RequestKind::May { source, .. }
            | RequestKind::ChoosePlayer { source, .. } => add(&[*source]),
            RequestKind::ChooseName { source, looked_at, .. } => {
                add(&[*source]);
                add(looked_at);
            }
            RequestKind::TapForPower { source, cards, .. } | RequestKind::CollectEvidence { source, cards, .. } | RequestKind::Craft { source, cards, .. } => {
                add(&[*source]);
                add(cards);
            }
            RequestKind::RemoveCounters { source, pools, .. } => {
                add(&[*source]);
                add(&pools.iter().map(|p| p.object).collect::<Vec<_>>());
            }
            RequestKind::DeclareAttackers { eligible, requirements, battles, planeswalkers, .. } => {
                add(battles);
                add(planeswalkers);
                add(eligible);
                add(requirements);
            }
            RequestKind::AssignCombatDamage { divisions } => {
                for d in divisions {
                    add(&[d.source]);
                    add(&d.creatures);
                    if let Target::Object(id) = d.defender {
                        add(&[id]);
                    }
                }
            }
            RequestKind::DeclareBlockers { eligible, attackers, allowed, .. } => {
                add(eligible);
                add(attackers);
                add(&allowed.iter().flat_map(|(a, b)| [*a, *b]).collect::<Vec<_>>());
            }
            RequestKind::DiscardToHandSize { hand, .. } => add(hand),
            RequestKind::Mulligan { .. } | RequestKind::ChooseStartingPlayer { .. } => {}
            RequestKind::OrderTriggers { sources, .. } => add(sources),
            RequestKind::DistributeCards { source, cards, destinations, .. } => {
                add(&[*source]);
                add(cards);
                for d in destinations {
                    add(&d.eligible);
                }
            }
            RequestKind::SearchLibrary { source, library, rule, .. } => {
                add(&[*source]);
                add(library);
                add(rule.among.as_deref().unwrap_or(&[]));
            }
            RequestKind::ChooseReplacement { subject, sources, damage, .. } => {
                if let Some(s) = subject {
                    add(&[*s]);
                }
                add(sources);
                if let Some(d) = damage {
                    add(&[d.source]);
                    if let Target::Object(to) = d.to {
                        add(&[to]);
                    }
                }
            }
        }
    }
}

/// Who makes a choice the rules or an object ask of `player`, who has left the game (CR 800.4f-h): no one, for a cost
/// (800.4f); for another choice an object asks, another player its controller chooses - another opponent where an opponent
/// was to choose, if there is one (800.4g); for who goes first, or which replacement effect applies first to an event that
/// is still to happen, the next player in turn order (800.4h), as for an object's choice whose controller has left too; and
/// for the player's own game actions, no one (800.4a, 800.4j: `StandIn::NotAsked`).
pub fn stand_in(state: &crate::GameState, kind: &RequestKind, player: PlayerId) -> StandIn {
    if kind.is_cost() {
        return StandIn::Unpaid;
    }
    let next = || state.turn_order_from(player).next().unwrap_or(player);
    if matches!(kind, RequestKind::ChooseStartingPlayer { .. } | RequestKind::ChooseReplacement { .. }) {
        return StandIn::By(next());
    }
    let Some(source) = kind.source().filter(|&s| s != ObjectId::NONE && s.index() < state.objects.len()) else {
        return StandIn::NotAsked;
    };
    let controller = state.obj(source).controller;
    if state.player(controller).lost {
        return StandIn::By(next());
    }
    let in_game: Vec<PlayerId> = state.apnap().collect();
    let opponents: Vec<PlayerId> = in_game.iter().copied().filter(|&p| p != controller).collect();
    let candidates = if player != controller && !opponents.is_empty() { opponents } else { in_game };
    match candidates.as_slice() {
        [] => StandIn::Unpaid,
        [only] => StandIn::By(*only),
        _ => StandIn::Chosen { controller, source, candidates },
    }
}

/// The counters of one kind on one permanent that a `RequestKind::RemoveCounters` may take from, and how many it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CounterPool {
    pub object: ObjectId,
    pub kind: CounterKind,
    pub count: u16,
}

/// Damage a question about replacement effects is about (CR 616.1): its source, whom it would be dealt to and how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DamageAbout {
    pub source: ObjectId,
    pub to: Target,
    pub amount: u16,
}

/// Where the cards a library search finds are going (CR 701.23), as the searcher is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SearchDestination {
    Hand,
    TopOfLibrary,
    Graveyard,
    Battlefield,
    Exile,
    ExileFaceDown,
    /// Cast by the searcher as the effect resolves.
    Cast,
    /// Onto the battlefield tapped (Nissa, Winds of Abandon).
    BattlefieldTapped,
    /// Cast by the searcher without paying its mana cost (Sunforger).
    CastFree,
    /// Where it goes depends on the card found and on choices made after (Transmute Artifact): the search says only what
    /// it looks for.
    Found,
    /// A land card onto the battlefield tapped under the searcher's control, any other card into the searcher's hand
    /// (Archdruid's Charm): the type of the card found decides, as it was in the library.
    LandTappedElseHand,
}

/// What declaring one creature as an attacker of one player, planeswalker or battle costs (CR 508.1d, 508.1h): the sum of
/// every static ability that makes that attack cost something ("creatures can't attack you unless their controller pays
/// {1} for each of those creatures"). Attacks that cost nothing have no entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttackCost {
    pub attacker: ObjectId,
    pub target: Target,
    pub cost: ManaCost,
    /// The first permanent whose static ability asks it: what the payment is shown as being for.
    pub source: ObjectId,
}

impl AttackCost {
    /// What declaring `attacks` costs in all (CR 508.1h): each attack's own cost, added up.
    pub fn total(costs: &[AttackCost], attacks: &[(ObjectId, Target)]) -> ManaCost {
        let mut total = ManaCost::ZERO;
        for &(attacker, target) in attacks {
            if let Some(c) = costs.iter().find(|c| c.attacker == attacker && c.target == target) {
                total = total.plus(&c.cost);
            }
        }
        total
    }

    /// Whether `attacker` attacking `target` costs nothing to declare.
    pub fn free(costs: &[AttackCost], attacker: ObjectId, target: Target) -> bool {
        !costs.iter().any(|c| c.attacker == attacker && c.target == target)
    }
}

/// The requirements on a declaration of blockers (CR 509.1c): each block an effect requires if able ("all creatures able to
/// block enchanted creature do so"), among the blocks the restrictions allow; the most of them any declaration that obeys
/// every restriction obeys, which the declaration must obey too; and one such declaration, the engine's suggestion. Empty
/// where no effect requires a block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockRequirements {
    pub pairs: Vec<(ObjectId, ObjectId)>,
    pub most: u32,
    pub suggested: Vec<(ObjectId, ObjectId)>,
    /// The attacking bands (CR 702.22h), each attacker in one with its band's number; empty where no attacker is banded.
    pub bands: Vec<(ObjectId, u32)>,
}

impl BlockRequirements {
    /// Whether a creature blocking `declared` blocks `attacker` too: the same creature, or one in the same attacking band (a
    /// creature blocking one member of a band blocks every member, CR 702.22h).
    pub fn blocks_through(&self, declared: ObjectId, attacker: ObjectId) -> bool {
        let band = |a: ObjectId| self.bands.iter().find(|b| b.0 == a).map_or(0, |b| b.1);
        declared == attacker || (band(declared) != 0 && band(declared) == band(attacker))
    }

    /// How many of the requirements a declaration obeys (CR 509.1c): each one whose blocker blocks its attacker, as declared or
    /// through the attacker's band - so one block of a band may obey two.
    pub fn obeyed(&self, blocks: &[(ObjectId, ObjectId)]) -> u32 {
        crate::count_u32(self.pairs.iter().filter(|&&(b, a)| blocks.iter().any(|&(bb, x)| bb == b && self.blocks_through(x, a))).count())
    }
}

/// One creature's combat damage to divide among the creatures it is blocked by (and, with trample, the player or
/// planeswalker it attacks), with what is lethal to each and the engine's own suggestion.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DamageDivision {
    /// The creature whose combat damage this is: an attacker among its blockers, or a blocker among the attackers it blocks.
    pub source: ObjectId,
    /// The player asked is not its controller (CR 702.22j-k): banding gives the division of this creature's damage to the
    /// controller of the creatures with banding it is blocked by or is blocking, who divides it among them as they choose.
    pub banding: bool,
    pub damage: u16,
    pub creatures: Vec<ObjectId>,
    pub lethal: Vec<u16>,
    pub may_assign_to_defender: bool,
    pub defender: Target,
    pub suggested: Vec<u16>,
    pub suggested_to_defender: u16,
}

impl DamageDivision {
    /// The engine's own division of every creature's combat damage, as an answer's shares: lethal to each in turn,
    /// the rest to the defender.
    pub fn suggested_shares(divisions: &[DamageDivision]) -> Vec<(ObjectId, Option<ObjectId>, u16)> {
        let mut shares = Vec::new();
        for d in divisions {
            for (i, &c) in d.creatures.iter().enumerate() {
                if d.suggested[i] > 0 {
                    shares.push((d.source, Some(c), d.suggested[i]));
                }
            }
            if d.suggested_to_defender > 0 {
                shares.push((d.source, None, d.suggested_to_defender));
            }
        }
        shares
    }
}

/// "[Cards] with total mana value N or less" (CR 202.3): the mana values of the cards chosen together add up to at most
/// `most`. `values` is each eligible card's mana value, index for index with the question's `eligible`, read as it is asked
/// (where the card is: an {X} in a library or graveyard counts as 0, CR 202.3e), so whoever answers needs to know no card.
/// Michelangelo's Technique, Lively Dirge and Ao, the Dawn Sky print it of cards chosen; Fire Lord Sozin of targets
/// (`RequestKind::ChooseTarget::total`, its values index for index with the candidates).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TotalAtMost {
    pub values: Vec<u32>,
    pub most: u32,
}

impl TotalAtMost {
    /// The total mana value of `chosen`, each read at its index in `eligible` (a card not eligible adds nothing).
    /// Added up in a u64, so no number of u32 values overflows it.
    pub fn of<T: PartialEq>(&self, eligible: &[T], chosen: &[T]) -> u64 {
        chosen
            .iter()
            .filter_map(|c| eligible.iter().position(|e| e == c))
            .map(|i| u64::from(self.values.get(i).copied().unwrap_or(0)))
            .sum()
    }

    /// Whether `chosen` is within the limit.
    pub fn allows<T: PartialEq>(&self, eligible: &[T], chosen: &[T]) -> bool {
        self.of(eligible, chosen) <= u64::from(self.most)
    }

    /// The first of `eligible`, in order, that fit together within the limit, at most `count` of them: an answer always
    /// accepted, for a player who answers without reading the cards.
    pub fn first_fitting<T: Clone>(&self, eligible: &[T], count: usize) -> Vec<T> {
        let mut left = u32::from(self.most);
        let mut taken = Vec::new();
        for (i, card) in eligible.iter().enumerate() {
            let value = u32::from(self.values.get(i).copied().unwrap_or(0));
            if taken.len() < count && value <= left {
                left -= value;
                taken.push(card.clone());
            }
        }
        taken
    }
}

/// How many cards a choice of cards takes: the count of its `Selection`, which its caption words ("up to two").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Count {
    /// Exactly this many.
    Exactly(u32),
    /// From none to this many ("up to two").
    UpTo(u32),
    /// "Any number": none, some or all (CR 701.23, 608.2d).
    AnyNumber,
    /// This many, or fewer only when nothing more may be added: a search for "a card" or "two cards" with no quality it
    /// could fail to find takes what it can (CR 701.23b), and a choice of "a [quality] card" among cards some of which may
    /// not be chosen takes one if there is one.
    AsManyAsCan(u32),
}

impl Count {
    /// The most it takes, or `None` for any number.
    pub fn most(self) -> Option<u32> {
        match self {
            Count::Exactly(n) | Count::UpTo(n) | Count::AsManyAsCan(n) => Some(n),
            Count::AnyNumber => None,
        }
    }

    /// Whether a selection of `n` cards has room for one more.
    pub fn room(self, n: usize) -> bool {
        self.most().is_none_or(|most| (n as u64) < u64::from(most))
    }

    /// Whether `n` cards are as many as it takes, `more` saying whether any card could still be added to them.
    pub fn complete(self, n: usize, more: bool) -> bool {
        let n = n as u64;
        match self {
            Count::Exactly(most) => n == u64::from(most),
            Count::UpTo(most) => n <= u64::from(most),
            Count::AnyNumber => true,
            Count::AsManyAsCan(most) => n == u64::from(most) || (n < u64::from(most) && !more),
        }
    }
}

/// One part of a selection rule (`Selection`): a validator over the cards chosen so far, with the one number it is
/// parameterised by. The validator is a `cantrip_api::select::SelectFn` - a plain function, never a closure, so the rule is
/// data that the game-log tree moves through and replays like any other (`Game::goto`) - held here as `dyn Any`, because
/// this crate can't name the `Query` it reads the game through; `cantrip_api::select` makes parts and runs them. Two parts
/// are the same when they hold the same function with the same number and words.
#[derive(Clone, Copy)]
pub struct Part {
    /// The validator, private so that a part is made only by `cantrip_api::select::part` / `tallied`, the one place that
    /// knows its type; read it with `Part::judge`.
    judge: &'static (dyn std::any::Any + Send + Sync),
    /// The number the part is parameterised by: a count, a mana value, a card's id. Zero when it takes none.
    pub n: u32,
    /// What the part asks, in the words a caption puts after the cards' noun ("with different names", "with total mana
    /// value {n} or less", `{n}` standing for `n`); empty for a part the noun already says (`Selection::kind`).
    pub words: &'static str,
    /// What a client keeps count of for the player as the cards are chosen, beside the words: nothing, or a running total.
    pub tally: Tally,
}

/// What a client keeps count of for a part of a selection rule as the cards are chosen (`Part::tally`): display alone,
/// the rule's verdicts still judge the cards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tally {
    #[default]
    Nothing,
    /// The cards' total mana value, against the part's number as its most (CR 202.3).
    ManaValueAtMost,
}

impl Part {
    /// A part of `judge`, which must be a `cantrip_api::select::SelectFn`: called only by `cantrip_api::select::part` and
    /// `tallied`, which are the ways to make a part.
    #[doc(hidden)]
    pub const fn erased(judge: &'static (dyn std::any::Any + Send + Sync), n: u32, words: &'static str, tally: Tally) -> Part {
        Part { judge, n, words, tally }
    }

    /// The validator, as `dyn Any`: `cantrip_api::select` knows its type.
    #[doc(hidden)]
    pub fn judge(&self) -> &'static (dyn std::any::Any + Send + Sync) {
        self.judge
    }

    /// `words`, with `n` where it says `{n}`, and the name of the card whose id is `n` (`card`) where it says `{card}`.
    pub fn worded(&self, card: &dyn Fn(u32) -> String) -> String {
        let words = self.words.replace("{n}", &self.n.to_string());
        if words.contains("{card}") { words.replace("{card}", &card(self.n)) } else { words }
    }
}

impl std::fmt::Debug for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Part({:?}, {})", self.words, self.n)
    }
}

impl PartialEq for Part {
    fn eq(&self, other: &Part) -> bool {
        std::ptr::addr_eq(self.judge as *const _, other.judge as *const _) && self.n == other.n && self.words == other.words && self.tally == other.tally
    }
}

impl Eq for Part {}

/// Hashed by its number and words alone, which equal parts share: an address is no part of a position's fingerprint.
impl std::hash::Hash for Part {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.n.hash(state);
        self.words.hash(state);
    }
}

/// The parts of a selection rule, as many as it has: borrowed for ever when a rule is built in a constant
/// (`Selection::of`), shared when one is put together as the question is asked (`Selection::with`).
#[derive(Clone, Debug)]
pub enum Parts {
    Static(&'static [Part]),
    Shared(std::sync::Arc<[Part]>),
}

impl Parts {
    /// The parts, in order.
    pub fn as_slice(&self) -> &[Part] {
        match self {
            Parts::Static(parts) => parts,
            Parts::Shared(parts) => parts,
        }
    }
}

impl PartialEq for Parts {
    fn eq(&self, other: &Parts) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for Parts {}

impl std::hash::Hash for Parts {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

/// A selection rule (docs/engine-design.md, "Selection rules"): which sets of a question's cards answer a choice of cards
/// (`RequestKind::ChooseCards`) or a library search (`RequestKind::SearchLibrary`). Plain data: the `count`, the `parts`,
/// every one of which must allow a card for it to be added and call the cards chosen complete for them to be an answer,
/// and `among`, the cards a question asked by a list (`cantrip_mechanics::choose`'s older functions, `search_library`, the
/// core's costs) lets be chosen at all. `cantrip_api::select::judge` judges a selection by it; the core checks every
/// answer with it, and a client asks it which cards may be added as the player picks (the host's `Check`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Selection {
    pub count: Count,
    /// What the asking card calls the cards that may be chosen when it says more than "card" ("basic land", "nonland"):
    /// the caption's noun. Empty for "card".
    pub kind: &'static str,
    pub parts: Parts,
    /// The only cards that may be chosen, worked out as the question was asked; `None` for any of the question's cards.
    pub among: Option<Vec<ObjectId>>,
}

impl Selection {
    /// `count` cards, any of the question's.
    pub const fn new(count: Count) -> Selection {
        Selection { count, kind: "", parts: Parts::Static(&[]), among: None }
    }

    /// `count` of `eligible`, from among `cards`: a question asked with its eligible cards listed. An `eligible` that is
    /// all of `cards` leaves the rule no list to keep.
    pub fn listed(count: Count, cards: &[ObjectId], eligible: Vec<ObjectId>) -> Selection {
        let among = if eligible.as_slice() == cards { None } else { Some(eligible) };
        Selection { among, ..Selection::new(count) }
    }

    /// The same rule, with `part` as well: any number of parts may be added.
    pub fn with(mut self, part: Part) -> Selection {
        let mut parts = self.parts.as_slice().to_vec();
        parts.push(part);
        self.parts = Parts::Shared(parts.into());
        self
    }

    /// `count` cards with exactly `parts`: how a rule is built in a constant (`&[BASIC_LAND]` is kept for the whole
    /// program).
    pub const fn of(count: Count, parts: &'static [Part]) -> Selection {
        Selection { count, kind: "", parts: Parts::Static(parts), among: None }
    }

    /// The same rule, its cards called `kind` ("basic land").
    pub const fn called(mut self, kind: &'static str) -> Selection {
        self.kind = kind;
        self
    }

    /// The same rule, taking `count` cards.
    pub const fn taking(mut self, count: Count) -> Selection {
        self.count = count;
        self
    }

    /// The parts it has.
    pub fn parts(&self) -> impl Iterator<Item = &Part> {
        self.parts.as_slice().iter()
    }

    /// The words of every part that has some, `{n}` filled in: what a caption says after the noun. `card` names the card
    /// of an id, for a part that asks for a name ("named {card}").
    pub fn words(&self, card: &dyn Fn(u32) -> String) -> Vec<String> {
        self.parts().filter(|p| !p.words.is_empty()).map(|p| p.worded(card)).collect()
    }

    /// Whether `card` is one of the cards `among` lets be chosen.
    pub fn listed_card(&self, card: ObjectId) -> bool {
        self.among.as_ref().is_none_or(|among| among.contains(&card))
    }
}

/// One place the cards of a `DistributeCards` can go, and how many.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Destination {
    pub kind: DestinationKind,
    pub min: u8,
    /// `u8::MAX` for "the rest".
    pub max: u8,
    /// The cards it may take; every card looked at, when it takes any.
    pub eligible: Vec<ObjectId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DestinationKind {
    LibraryTop,
    LibraryBottom,
    Graveyard,
    Hand,
    Exile,
    /// Onto the battlefield face down (CR 701.40a, 708.3): manifest.
    BattlefieldFaceDown,
}

/// What a gift is (CR 702.174d-i): the effect the chosen opponent gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gift {
    /// "Gift a Food": the chosen player creates a Food token (CR 702.174d).
    Food,
    /// "Gift a card": the chosen player draws a card (CR 702.174e).
    Card,
    /// "Gift a tapped Fish": the chosen player creates a tapped 1/1 blue Fish creature token (CR 702.174f).
    TappedFish,
    /// "Gift an extra turn": the chosen player takes an extra turn after this one (CR 702.174g).
    ExtraTurn,
    /// "Gift a Treasure": the chosen player creates a Treasure token (CR 702.174h).
    Treasure,
    /// "Gift an Octopus": the chosen player creates an 8/8 blue Octopus creature token (CR 702.174i).
    Octopus,
}

/// A "you may" in the words of the card or rule that asks it (CR 608.2d), so the player is asked what the card says and
/// each answer is named for what it does: "Pay 2 life" and "Enters tapped", not Yes and No.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Question {
    /// What the bots read the choice by, a verb of the card vocabulary: `PayLife`, `Draw`, `SearchLibrary`.
    pub verb: &'static str,
    /// The question, after the source's name and a colon ("pay 2 life?"), or with `{source}` where the name goes.
    pub ask: &'static str,
    /// What the choice means beyond its buttons: "If you don't, it enters tapped". May be empty.
    pub detail: &'static str,
    /// The button that accepts, named for what it does.
    pub yes: &'static str,
    /// The button that declines, named for what it does.
    pub no: &'static str,
    /// How much, for a verb with an amount: the life `PayLife` pays.
    pub amount: u8,
    /// Whether accepting is the default a player passing through prompts takes. False for a choice that costs or risks
    /// something, so passing never pays it by accident.
    pub yes_is_default: bool,
}

impl Question {
    /// A question whose answer yes is the default, with no amount.
    pub const fn new(verb: &'static str, ask: &'static str, detail: &'static str, yes: &'static str, no: &'static str) -> Self {
        Question { verb, ask, detail, yes, no, amount: 0, yes_is_default: true }
    }

    /// The same question, with an amount for its verb.
    pub const fn amount(self, amount: u8) -> Self {
        Question { amount, ..self }
    }

    /// The same question, with neither answer the default.
    pub const fn no_default(self) -> Self {
        Question { yes_is_default: false, ..self }
    }
}

/// What a choice of a subtype is for, as far as the chooser needs to know to choose well.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WordUse {
    /// The type an effect names from now on ("as this enters, choose a creature type", CR 205.3m).
    Chosen,
    /// The word a text-changing effect replaces in its target's text (CR 612.1): "one creature type ...".
    Replaced,
    /// The word that replaces it ("... with another", CR 612.1).
    Replacing,
}

/// What a choice of colors is for (CR 105.1), as far as the chooser needs to know to choose well.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorUse {
    /// Nothing is said of it: the effect's own use (a color to protect from, a color everything becomes).
    Any,
    /// The color of mana to add (CR 106.1b): a color the chooser's hand asks for.
    AddMana,
    /// The color word a text-changing effect replaces in its target's text (CR 612.1).
    ReplacedWord,
    /// The color word that replaces it (CR 612.1).
    ReplacingWord,
    /// A color that helps the chooser: the one most of their own creatures and spells are, for "creatures you control of the
    /// chosen color get +1/+0" or "mana of the chosen color".
    Benefit,
}

/// Why a player is choosing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Purpose {
    /// The other attacking creatures that attack in a band with the request's source, a creature with banding (CR 702.22c,
    /// 508.1e): any number with banding and at most one without, all attacking what it attacks. None chosen is no band.
    Band,
    Discard,
    Sacrifice,
    /// A card chosen from a revealed hand (Duress).
    TakeFromHand,
    /// Which objects go in the first pile (CR 700.3).
    SeparatePiles,
    /// Which pile.
    ChoosePile,
    /// Enter as a copy (CR 707.2).
    CopyAsEnters,
    /// A creature to copy.
    CopyWhich,
    /// The targets of a triggered or activated ability.
    Targets,
    /// Which target of a spell or ability is changed to the request's source (CR 115.7b: "change a target of target spell or
    /// ability to this creature", Spellskite): a choice among its targets, not a target.
    ChangeTargetTo,
    Color,
    /// A cost's sacrifice (Fireblast's Mountains).
    SacrificeCost,
    /// Permanents returned to their owners' hands as a cost (Gush's Islands).
    ReturnToHand,
    /// Cards put back on top of a library (Brainstorm).
    PutOnLibrary,
    /// A card put onto the battlefield (Show and Tell).
    PutOntoBattlefield,
    /// The player, planeswalker or battle a creature put onto the battlefield attacking attacks, chosen by its controller
    /// as it enters when the effect names none (CR 508.4): not a target.
    EnterAttacking,
    /// A plain "you may" with nothing more to say about it. A prompt that asks only this shows the source's text and Yes
    /// and No; a card says what it asks with [`Purpose::Ask`] instead.
    Optional,
    /// A "you may" worded by the card or rule that asks it: the question, and what each answer does.
    Ask(&'static Question),
    /// "The player whose turn it is may end the turn" (CR 724.1): Obeka, Brute Chronologist.
    EndTurn,
    /// The rest of the C# engine's purposes (CardChoicePurpose), one for one.
    PutOnTop,
    KeepLegend,
    Exile,
    Untap,
    /// Permanents an effect taps as it resolves (CR 701.26a), not to pay a cost: Thousand Moons Smithy's "you may tap five
    /// untapped artifacts and/or creatures you control".
    Tap,
    ExileToPay,
    Keep,
    PutIntoHand,
    Cast,
    TapToPay,
    OrderSimultaneous,
    Destroy,
    Vote,
    PutIntoGraveyard,
    Play,
    Splice,
    PutOnBottom,
    Attach,
    /// A creature to exile a card with cipher encoded on (CR 702.99a).
    Encode,
    /// Whether to promise a gift (CR 702.174a), and which gift it is.
    PromiseGift(Gift),
    /// "You may reveal this card from your hand as you draw it" (CR 702.94a): miracle, with the card's one miracle ability.
    /// None for a card with more than one, which is then asked which it is revealed with (`Label::Miracle`).
    RevealForMiracle(Option<MiracleOf>),
    /// "You may cast it by paying its miracle cost" (CR 702.94a), as the linked ability resolves: the miracle ability it was
    /// revealed with.
    CastForMiracleCost(MiracleOf),
    /// Whether to pay another cost rather than an activated ability's (CR 118.9).
    AlternativeCost,
    /// What a cost beholds (CR 701.4a): a permanent chosen or a card revealed from the hand.
    Behold,
    /// The creature a cost blights (CR 701.68a).
    Blight,
    /// The permanent that gets this many counters of this kind, chosen without targeting (CR 122.1, 608.2d): which Jace
    /// planeswalker token empower Jace puts its loyalty counters on (Theorist's Sanctum, Fatehold Charm, Countersculpt),
    /// which Army amass puts its +1/+1 counters on (CR 701.47a; Orcish Bowmasters).
    PutCounters(CounterKind, u8),
    /// The creature harmonize taps, if any (CR 702.180a-b): up to one untapped creature, whose power comes off the generic
    /// mana of the total cost.
    TapToReduce,
    /// A card chosen to be revealed (CR 701.20a) from among cards looked at: what becomes of it is for the effect to say
    /// (Break Out reveals a creature card, then puts it onto the battlefield or into the hand).
    Reveal,
    /// A card an effect exiles that then becomes plotted (CR 702.170a, 702.170d): its owner may cast it as a sorcery on a later
    /// turn without paying its mana cost. Something gained, not given up, as with plot's own special action (Jace
    /// Reawakened's "you may exile a nonland card with mana value 3 or less from your hand. If you do, it becomes plotted").
    Plot,
    /// "You may cast it without paying its mana cost" as an effect resolves (CR 118.9, 608.2g), the card staying where it is
    /// if not: suspend, rebound, hideaway, Malcolm.
    CastFree,
    /// "You may cast a copy of it without paying its mana cost" (CR 707.12): paradigm, cipher.
    CastCopyFree,
    /// "You may cast it transformed without paying its mana cost" (CR 310.12b): a Siege defeated.
    CastTransformedFree,
    /// "You may cast it by paying its madness cost" (CR 702.35a).
    CastForMadness,
    /// "You may cast it without paying its mana cost. Put it into your hand if you don't" (Beseech the Mirror).
    CastFoundOrHand,
    /// "You may cast it without paying its mana cost. If you don't, it goes on the bottom of your library" (cascade).
    CastOrBottom,
    /// "You may cast any number of copies without paying their mana costs" within a budget (Uldaros Theorix): a copy chosen to be
    /// cast, with this much mana value left to spend in all.
    CastCopyFreeWithin(u8),
    /// Whose damage a prevention shield prevents first, where two or more sources would deal damage to the shielded
    /// permanent or player at the same time (CR 615.7): a source chosen from among them, by the player or the controller
    /// of the permanent, with this much left on the shield to prevent. The rest of the shield goes to the others.
    PreventDamageFrom(u16),
    /// Cards in the chooser's hand drawn this turn, for each of which they will pay this much life or put it on top of their
    /// library (Sylvan Library): which of them the choice is about, not which go back.
    PayLifeOrPutOnTop(u8),
}

/// The text of one option, as an index, so no string is built at runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Label {
    /// The nth label of the source card's definition.
    Card(CardId, u8),
    /// The nth label of a card's definition as one of the words its text offers to choose between (CR 607.2f): "choose
    /// Jeskai or Temur", "choose odd or even". Worded as `Card`; a question of these alone is captioned with all of them.
    Option(CardId, u8),
    /// A card's name: an option that is one of several cards, named as the C# engine offers them.
    Name(CardId),
    /// One of two piles (CR 700.3), and whether it has nothing in it.
    Pile { first: bool, empty: bool },
    /// The nth mode of a modal spell (its ability has index `u8::MAX`) or ability.
    Mode(AbilityRef, u8),
    /// The nth way of paying the source card's additional cost (`CardDef::additional_choice`).
    Extra(CardId, u8),
    /// "Pay 2 life for {W/P}": one Phyrexian symbol paid with life (CR 107.4f).
    PayLife(Color),
    /// "Pay {2} for {2/W}": one monocolored hybrid symbol paid with two generic mana rather than its color (CR 107.4e).
    PayGeneric(Color),
    /// "Pay {U/R} with {R}": the two-color hybrid symbol {a/b} announced as the half `as` (CR 601.2b), where a reduction
    /// meets its other half (`announce_hybrids`).
    PayHybridAs(Mana, Mana, Mana),
    /// "Reduce by {R} for {U/R}": a hybrid symbol {a/b} of a reduction taken as its half `as` (CR 118.7e).
    ReduceAs(Mana, Mana, Mana),
    /// "Reduce by {2} for {2/W}": a {2/C} symbol of a reduction taken as two generic mana (CR 118.7e).
    ReduceGeneric(Color),
    /// A kind of counter, to remove one of as a cost (CR 118.3).
    Counter(CounterKind),
    /// One of the faces a card may be cast as (CR 709.3, 715.3, 712.8a), named by the face's name.
    Face(CardId),
    /// A split card with fuse cast as both halves at once (CR 702.102a), beside its halves.
    Fused(CardId),
    /// One of a card's miracle abilities, to reveal it with (CR 702.94a, 607.5).
    Miracle(MiracleOf),
    /// A type of mana to add (CR 106.1b: white, blue, black, red, green or colorless): "one mana of any type that land
    /// produced".
    ManaType(Mana),
    /// Paying a spell's mana cost, as it is cast (CR 601.2b), beside the alternative costs it could be cast for instead.
    ManaCost,
    /// The `u8`th alternative cost a permanent of this card gives the spells a player casts (CR 118.9:
    /// `CardDef::spell_alternatives`), to pay rather than the mana cost.
    AlternativeCost(CardId, u8),
    /// The `u8`th of a card's own alternative ways to be cast (`CardDef::ways`, CR 118.9): evoke, Force of Will's.
    Way(CardId, u8),
    /// One instance of the word "target" on a spell or ability (CR 115.3), named by its slot's text, with what it holds:
    /// which of the instances that hold the same object a "change a target" effect changes (CR 115.7b, Spellskite).
    TargetSlot(&'static str, Target),
}

/// One miracle ability of a card (CR 702.94a), named without the object, which a question asked as the card is drawn may not
/// yet have: the card's own (`granter` the card itself, `index` `MiracleOf::OWN`), or the `index`th a permanent of the
/// `granter` card gives it (`CardDef::granted_miracles`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MiracleOf {
    pub card: CardId,
    pub granter: CardId,
    pub index: u8,
}

impl MiracleOf {
    pub const OWN: u8 = u8::MAX;

    pub fn is_own(&self) -> bool {
        self.index == Self::OWN
    }
}

/// An answer to a request.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Answer {
    Pass,
    Act(Action),
    Cards(Vec<ObjectId>),
    Targets(Vec<Target>),
    Modes(Vec<u8>),
    Colors(Vec<Color>),
    /// "You may", and a mulligan (yes is take one).
    Yes,
    No,
    Attack(Vec<(ObjectId, Target)>),
    Block(Vec<(ObjectId, ObjectId)>),
    Player(PlayerId),
    /// A card name chosen (CR 201.3).
    Name(Option<CardId>),
    /// A subtype chosen.
    Subtype(Subtype),
    /// Indices in order: the order to put triggered abilities on the stack.
    Order(Vec<u32>),
    /// Combat damage shares: (source, the creature it is dealt to, or `None` for the defender, amount).
    Shares(Vec<(ObjectId, Option<ObjectId>, u16)>),
    /// Where each looked-at card goes: one list per destination, in placing order (for a library top, the first is
    /// the top card); or, where allowed, shuffle instead.
    Placements { placements: Vec<Vec<ObjectId>>, shuffle: bool },
    /// A divided slot's targets, each with its share (CR 601.2d).
    Divided(Vec<(Target, u32)>),
    /// Give up the spell or ability being announced (CR 733.1).
    Cancel,
    /// The value of X, or how many times the spell is kicked, announced at once (CR 601.2b): an answer to a `PayCosts` that
    /// is `announcing`, up to its `most`, in place of raising and lowering it a step at a time.
    Announce(u32),
    /// The counters removed for a `RequestKind::RemoveCounters`: how many of each kind from each permanent.
    Counters(Vec<(ObjectId, CounterKind, u16)>),
    /// The index of the option chosen, for a question of one-of-several (which replacement effect applies first).
    Index(u8),
}

#[cfg(test)]
mod total_tests {
    use super::*;

    /// Mana values that add up past `u32::MAX` are not taken as fitting within a limit: the sum does not wrap.
    #[test]
    fn a_total_past_u32_does_not_wrap_under_the_limit() {
        let t = TotalAtMost { values: vec![u32::MAX, 2], most: 5 };
        assert_eq!(t.of(&[1, 2], &[1, 2]), u64::from(u32::MAX) + 2);
        assert!(!t.allows(&[1, 2], &[1, 2]));
        assert!(t.allows(&[1, 2], &[2]));
    }
}
