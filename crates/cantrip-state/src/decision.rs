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
    /// The answers this run of the frame has been given, in the order it asked.
    pub answers: Vec<Answer>,
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
}

impl Frame {
    pub fn new(kind: FrameKind) -> Frame {
        Frame {
            kind,
            answers: Vec::new(),
            pending: None,
            mark: 0,
            log_mark: 0,
            stage: 0,
            next_stage: 0,
            staged: false,
            scratch: [0; 4],
        }
    }

    fn position(&self) -> (&FrameKind, &Vec<Answer>, Option<(&PlayerId, &RequestKind)>, u16, u16, bool, &[u32; 4]) {
        (
            &self.kind,
            &self.answers,
            self.pending.as_ref().map(|r| (&r.player, &r.kind)),
            self.stage,
            self.next_stage,
            self.staged,
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
    /// For a cast, which way (0 its mana cost, n its nth alternative cost); for an activation, which ability of the
    /// source's computed ability list.
    pub index: u8,
    /// Targets the action is announced with, and who may be chosen for them (CR 601.2c): the candidates of every
    /// target slot together.
    pub targets_required: u8,
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
        x: u16,
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
        life: u8,
        life_payable: bool,
        question: &'static Question,
        options: Vec<ActionOption>,
        x: Option<u16>,
    },
    /// Choose `count` (or up to `count`) of `eligible`, from among `cards` (all of which the player sees).
    /// `kind` is what the asking card calls what may be chosen when it says more than "card" ("artifact or creature",
    /// "nonland"), empty otherwise: the dock words the choice as the card does. `total`, when the cards chosen together are
    /// limited to a total mana value ("up to two creature cards with total mana value 6 or less"), is that limit: the core
    /// refuses a choice whose mana values add up to more.
    ChooseCards { source: ObjectId, purpose: Purpose, cards: Vec<ObjectId>, eligible: Vec<ObjectId>, count: u8, up_to: bool, kind: &'static str, total: Option<TotalAtMost> },
    /// `least` to `count` targets are chosen (a range, "one or two target creatures", CR 601.2c); `optional` is `least == 0`.
    /// Targets for an ability as it goes on the stack, or any other "choose a [thing]" the C# engine asks as targets.
    /// `label` is what the slot is for as its card words it (`TargetSpec::label`, empty when it has nothing to add), and
    /// `slot_player` the opponent whose slot it is when the ability has one for each opponent. `total`, when the targets
    /// chosen together are limited to a total mana value ("any number of target creature cards with total mana value X or
    /// less", `TargetSpec::total_mana_value`), is that limit, its values index for index with `candidates`: the core refuses
    /// a choice whose mana values add up to more.
    ChooseTarget { source: ObjectId, purpose: Purpose, candidates: Vec<Target>, optional: bool, least: u8, count: u8, any_number: bool, label: &'static str, slot_player: Option<PlayerId>, total: Option<TotalAtMost> },
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
    /// any subset of `cards` whose `powers` (what each counts as towards the total, CR 702.122b) add up to it.
    TapForPower { source: ObjectId, purpose: Purpose, cards: Vec<ObjectId>, powers: Vec<u8>, minimum: u8 },
    /// Collect evidence N (CR 701.59a): exile any subset of `cards` (the player's graveyard) whose `values` (their mana
    /// values) add up to `minimum` or more.
    CollectEvidence { source: ObjectId, cards: Vec<ObjectId>, values: Vec<u8>, minimum: u8 },
    /// Craft's materials (CR 702.167a): exile `minimum` to `maximum` of `cards`, which are permanents the player controls
    /// and cards in their graveyard, each one of what `quality` names ("artifact", "creature"). An exact number of
    /// materials has `minimum == maximum`; "one or more" has `maximum` of every card offered.
    Craft { source: ObjectId, cards: Vec<ObjectId>, minimum: u8, maximum: u8, quality: &'static str },
    /// Remove `total` counters from among permanents (CR 118.3, 122.1): "remove three counters from among creatures you
    /// control" (Dawnhand Dissident; Tayam, Luminous Enigma). Each of `pools` is the counters of one kind on one permanent,
    /// with how many there are; any of them may be taken, of any kind and from any of the permanents, adding up to `total`.
    /// Answered with `Answer::Counters`.
    RemoveCounters { source: ObjectId, pools: Vec<CounterPool>, total: u8 },
    /// The targets of a slot that divides a total among them, and each one's share (CR 601.2d): "distribute two +1/+1
    /// counters among up to two target creatures". At most `max_targets` of `candidates` (any number when 0), each with a
    /// share of at least 1, adding up to `total`; none at all when the slot is `optional`.
    ChooseDividedTargets { source: ObjectId, candidates: Vec<Target>, total: u16, max_targets: u8, optional: bool },
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
    DiscardToHandSize { count: u8, hand: Vec<ObjectId> },
    /// Keep the opening hand or take a mulligan (CR 103.5).
    Mulligan { mulligans: u8 },
    /// Who takes the first turn (CR 103.1).
    ChooseStartingPlayer { candidates: Vec<PlayerId> },
    /// The order a player's triggered abilities go on the stack (CR 603.3b): a permutation, first on the stack first.
    OrderTriggers { sources: Vec<ObjectId>, abilities: Vec<AbilityRef> },
    /// Cards looked at, each to be put in one of the destinations (Ponder, surveil, "put one into your hand and the
    /// rest into your graveyard"). `cards` are top of the library first.
    DistributeCards { source: ObjectId, cards: Vec<ObjectId>, destinations: Vec<Destination>, may_shuffle: bool },
    /// Search a library (CR 701.23): `library` is the whole library, `eligible` what the search may find; with
    /// `optional`, finding nothing is allowed (CR 701.23b).
    /// `destination` is where the searcher is told the cards found are going; whose library it is, is the owner of the
    /// cards in `library`.
    SearchLibrary { source: ObjectId, library: Vec<ObjectId>, eligible: Vec<ObjectId>, count: u8, optional: bool, destination: SearchDestination },
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
            RequestKind::ChooseCards { source, cards, eligible, .. } => {
                add(&[*source]);
                add(cards);
                add(eligible);
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
            RequestKind::SearchLibrary { source, library, eligible, .. } => {
                add(&[*source]);
                add(library);
                add(eligible);
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
    /// Where it goes depends on the card found (Transmute Artifact): the search says only what it looks for.
    /// Onto the battlefield tapped (Nissa, Winds of Abandon).
    BattlefieldTapped,
    /// Cast by the searcher without paying its mana cost (Sunforger).
    CastFree,
    Found,
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
    pub most: u8,
    pub suggested: Vec<(ObjectId, ObjectId)>,
}

impl BlockRequirements {
    /// How many of the requirements a declaration obeys: each required block it makes.
    pub fn obeyed(&self, blocks: &[(ObjectId, ObjectId)]) -> u8 {
        blocks.iter().filter(|b| self.pairs.contains(b)).count() as u8
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
    pub values: Vec<u16>,
    pub most: u16,
}

impl TotalAtMost {
    /// The total mana value of `chosen`, each read at its index in `eligible` (a card not eligible adds nothing).
    pub fn of<T: PartialEq>(&self, eligible: &[T], chosen: &[T]) -> u32 {
        chosen
            .iter()
            .filter_map(|c| eligible.iter().position(|e| e == c))
            .map(|i| u32::from(self.values.get(i).copied().unwrap_or(0)))
            .sum()
    }

    /// Whether `chosen` is within the limit.
    pub fn allows<T: PartialEq>(&self, eligible: &[T], chosen: &[T]) -> bool {
        self.of(eligible, chosen) <= u32::from(self.most)
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
    /// A kind of counter, to remove one of as a cost (CR 118.3).
    Counter(CounterKind),
    /// One of the faces a card may be cast as (CR 709.3, 715.3, 712.8a), named by the face's name.
    Face(CardId),
    /// One of a card's miracle abilities, to reveal it with (CR 702.94a, 607.5).
    Miracle(MiracleOf),
    /// A type of mana to add (CR 106.1b: white, blue, black, red, green or colorless): "one mana of any type that land
    /// produced".
    ManaType(Mana),
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
    Order(Vec<u8>),
    /// Combat damage shares: (source, the creature it is dealt to, or `None` for the defender, amount).
    Shares(Vec<(ObjectId, Option<ObjectId>, u16)>),
    /// Where each looked-at card goes: one list per destination, in placing order (for a library top, the first is
    /// the top card); or, where allowed, shuffle instead.
    Placements { placements: Vec<Vec<ObjectId>>, shuffle: bool },
    /// A divided slot's targets, each with its share (CR 601.2d).
    Divided(Vec<(Target, u16)>),
    /// Give up the spell or ability being announced (CR 733.1).
    Cancel,
    /// The counters removed for a `RequestKind::RemoveCounters`: how many of each kind from each permanent.
    Counters(Vec<(ObjectId, CounterKind, u16)>),
    /// The index of the option chosen, for a question of one-of-several (which replacement effect applies first).
    Index(u8),
}
