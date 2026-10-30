//! Decisions are data (docs/engine-design.md, "Decisions are data"): what the engine is in the middle of is a
//! stack of frames in the state, and a question waiting for an answer is a frame holding the typed request.
//!
//! The requests mirror the C# engine's prompts kind for kind and option for option, in the same order, because the
//! protocol the clients speak and the scenarios' "the first one offered" both depend on it.

use crate::object::*;
use crate::vocab::*;
use crate::AttackRules;

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
    /// Resolving the stack item with this serial (CR 608): the top of the stack as it began. What it casts or copies as it
    /// resolves goes on the stack above it (CR 608.2g, 707.10), and it is still the one resolving.
    Resolve { serial: u32 },
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
    /// Pay {3} and put the companion its player revealed before the game into their hand from outside the game (CR
    /// 702.139a, 116.2g): a special action, at sorcery timing, once a game. `source` is the card outside the game.
    TakeCompanion,
}

/// Which way of paying with something other than mana a `TapToPay` or `ExileToPay` option is (its `index`).
/// The `index` of an `ActivateAbility` option that is a mana ability of the player's own (CR 605.1a, 611.2a) and not of any
/// object: the nth effect that gave them one, added to this. Its `source` is the card that gave it, wherever it is now.
pub const PLAYER_MANA_ABILITY: u8 = 200;

/// The steps of paying for a spell with assist (CR 702.132a) a `PayCosts` question can be: the player the caster chose
/// activates mana abilities (`ConfirmCasting` when done), then the caster does, then the chosen player announces how much of
/// the generic mana they pay (`most` at most) and pays it from their pool alone; then the caster pays the rest from theirs,
/// no mana ability offered (CR 601.2g: they had their chance).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AssistStep {
    #[default]
    None,
    HelperActivates,
    CasterActivates,
    HelperAnnounces,
    HelperPays,
    CasterPays,
}

impl AssistStep {
    /// A window for activating mana abilities, which `ConfirmCasting` closes, nothing being paid in it.
    pub fn activating(self) -> bool {
        matches!(self, AssistStep::HelperActivates | AssistStep::CasterActivates)
    }
}

pub const PAY_DELVE: u8 = 0;
pub const PAY_IMPROVISE: u8 = 1;
pub const PAY_CONVOKE: u8 = 2;
/// Waterbend (CR 701.67a): an untapped artifact or creature tapped pays {1}.
pub const PAY_WATERBEND: u8 = 3;

/// The `most` of a cost a player "may pay any number of times" as an effect resolves (`RequestKind::UnlessPays::times`,
/// `Game::may_pay_times`): no number but what the cost's shape holds (`ManaCost::times`).
pub const ANY_NUMBER_OF_TIMES: u16 = u16::MAX;

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

/// What a chosen card name must be the name of (CR 201.4a: "a name chosen with certain characteristics must be the name
/// of a card that has them"), as the card words it: every printed restriction is card types the named card has, card types
/// it has none of, and "other than a basic land card name". "A card name" (`ANY`), "a nonland card name" (`NONLAND`: Cabal
/// Therapy, Tamiyo, Collector of Tales), "a land card name" (`LAND`: Petrified Hamlet), "a creature card name" (Infinite
/// Obliteration), "an artifact card name" (Dispossess), "a noncreature, nonland card name" (Council of the Absolute), "a
/// card name other than a basic land card name" (`OTHER_THAN_BASIC_LAND`: Necromentia, Booby Trap, Desperate Research, Null
/// Chamber), and "a nonbasic land card name" (Alpine Moon), which is a land card name other than a basic land card name.
/// The name's own card is what is judged, a back face by the back face's characteristics alone (CR 201.4d).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NameKind {
    /// The card types the named card has, every one of them ("a land card name").
    pub types: TypeSet,
    /// The card types it has none of ("a noncreature, nonland card name").
    pub non_types: TypeSet,
    /// "Other than a basic land card name": it is not a basic land, a land with the supertype basic (CR 205.4c).
    pub other_than_basic_land: bool,
}

impl NameKind {
    /// "A card name": any card's.
    pub const ANY: NameKind = NameKind { types: TypeSet::NONE, non_types: TypeSet::NONE, other_than_basic_land: false };
    /// "A nonland card name".
    pub const NONLAND: NameKind = NameKind { non_types: TypeSet::of(&[CardType::Land]), ..NameKind::ANY };
    /// "A land card name".
    pub const LAND: NameKind = NameKind { types: TypeSet::of(&[CardType::Land]), ..NameKind::ANY };
    /// "A card name other than a basic land card name".
    pub const OTHER_THAN_BASIC_LAND: NameKind = NameKind { other_than_basic_land: true, ..NameKind::ANY };

    /// Whether the name of a card with these card types and supertypes may be chosen.
    pub fn allows(self, types: TypeSet, supertypes: Supertypes) -> bool {
        types.0 & self.types.0 == self.types.0
            && types.0 & self.non_types.0 == 0
            && !(self.other_than_basic_land && supertypes.basic && types.has(CardType::Land))
    }

    /// What the card asks for, in its words: "a card name", "a nonland card name", "an artifact card name", "a
    /// noncreature, nonland card name", "a card name other than a basic land card name", "a nonbasic land card name".
    pub fn words(self) -> String {
        let mut words: Vec<String> = self.non_types.iter().map(|t| format!("non{}", t.name().to_lowercase())).collect();
        let nonbasic_land = self.other_than_basic_land && self.types.has(CardType::Land);
        let types: Vec<String> = self.types.iter().map(|t| t.name().to_lowercase()).collect();
        if !types.is_empty() {
            let types = types.join(" ");
            words.push(if nonbasic_land { format!("nonbasic {types}") } else { types });
        }
        let mut phrase = words.join(", ");
        phrase.push_str(if phrase.is_empty() { "card name" } else { " card name" });
        if self.other_than_basic_land && !nonbasic_land {
            phrase.push_str(" other than a basic land card name");
        }
        let article = if phrase.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
        format!("{article} {phrase}")
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
        /// While announcing: the smallest value that may be announced, 1 for an X that can't be 0 (CR 107.3a, 601.2b), else
        /// 0. Zero while paying.
        least: u32,
        /// The X being announced is a "−X" loyalty cost's (CR 606.4, 107.3): it removes that many loyalty counters from
        /// `source`, up to the loyalty it has, and costs no mana.
        loyalty: bool,
        /// The creatures whose cost to attack this pays (CR 508.1h-j), when it is one: `source` is then the permanent
        /// whose static ability makes attacking cost something (the first, when several do). Empty for anything else.
        attackers: Vec<ObjectId>,
        /// The life the total cost asks besides its mana (CR 119.4, 601.2h: "an additional 3 life", "pay X life"), paid with it
        /// once the mana is. Zero while X is announced, and for anything but a spell.
        life: u32,
        /// While announcing an X that is not paid in mana: what X governs, as the cost words it ("the least power of the
        /// While announcing an X that is not paid in mana: what X governs, in words ("the least power of the creature you
        /// sacrifice", casualty X, CR 702.153a), for the player to read where "mana for X" would be false.
        /// Empty for an X that is mana's, life's or a count of loyalty counters, and while paying.
        governs: &'static str,
        /// Where this question stands in paying for a spell with assist (CR 702.132a), `AssistStep::None` for any other.
        assist: AssistStep,
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
    ///
    /// `times` is `Some(most)` for a cost the payer "may pay up to [most] times" or "any number of times"
    /// (`ANY_NUMBER_OF_TIMES`), as one payment whose number is chosen as it is paid (CR 603.12a): `x` is then the number of
    /// times so far, from 1 (declining is paying it no times), `total` the cost that many times over, and `IncreaseX` and
    /// `DecreaseX` change the number within 1 to `most`.
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
        times: Option<u16>,
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
    /// `slot_player` the opponent whose slot it is when the ability has one for each opponent. `together` is the rule of the
    /// slot's targets together (`TargetSpec::together`, a set rule: "with total mana value X or less", "with different mana
    /// values"), its number read (`TargetSpec::COUNTED` as the slot's count), none for a slot that asks
    /// nothing of them together: `cantrip_api::select::judge_targets` judges a choice by it, the core refuses one it does not
    /// call complete, and a client asks it which candidates may be added as the player picks (the host's `Check`).
    ChooseTarget { source: ObjectId, purpose: Purpose, candidates: Vec<Target>, optional: bool, least: u32, count: u32, any_number: bool, label: &'static str, slot_player: Option<PlayerId>, together: Option<Part> },
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
    /// `kind` is what the name must be the name of (CR 201.4a): any card, a nonland card or a land card. A name must be
    /// chosen (CR 201.3): `Answer::Name(None)` is refused, and with no suggestion an unscripted player takes the first name
    /// `kind` allows (`cantrip_rules::default_name`).
    ChooseName { source: ObjectId, looked_at: Vec<ObjectId>, suggestion: Option<CardId>, kind: NameKind },
    /// One subtype of `choices`, in order ("choose a creature type", CR 205.3m: the C# engine asks it as a modes
    /// question over every creature type, more than a mode index holds).
    ChooseSubtype { source: ObjectId, choices: Vec<Subtype>, purpose: WordUse },
    /// "You may".
    May { source: ObjectId, purpose: Purpose },
    /// A player chosen without targeting (CR 608.2d, 614.12a): True-Name Nemesis's "as this enters, choose a player",
    /// Baleful Mastery's "an opponent draws a card". Answered with `Answer::Player`. `verb` and `amount` say what the
    /// chosen player will do, as hint text; `subject` is the object the choice is for, when it is one of several the effect
    /// treats one at a time (Dack Fayden, Helping Hand's "for each of those permanents, choose a different opponent").
    ChoosePlayer { source: ObjectId, candidates: Vec<PlayerId>, verb: &'static str, amount: u16, subject: Option<ObjectId> },
    /// Tap any number of untapped creatures with total power `minimum` or more (CR 702.122a crew, CR 702.171a saddle):
    /// any subset of `cards` whose `powers` (what each counts as towards the total, CR 702.122b) add up to it. A negative
    /// power is added as it is (CR 107.1b), taking from the total.
    ///
    /// Harmonize's creature (`Purpose::TapToReduce`, CR 702.180a) is up to one, and `powers` is what tapping each takes off the
    /// generic mana of the total cost: exactly, or, with `up_to`, the most it could take, the total not yet settled as the
    /// creature is chosen (CR 702.180b, 601.2b: X, a kicker and the targets come after it).
    TapForPower { source: ObjectId, purpose: Purpose, cards: Vec<ObjectId>, powers: Vec<i16>, minimum: u8, up_to: bool },
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
    /// Declare attackers (CR 508.1): any of `eligible`, each at one of `players`; `requirements` are the requirements on
    /// the declaration (CR 508.1d), counted and judged by `AttackRules::judge`.
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
        requirements: AttackRules,
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
            RequestKind::ChooseModes { source, modes, .. } => {
                add(&[*source]);
                for mode in modes {
                    if let Label::Pile { cards, .. } | Label::FaceUpPile { cards } = mode {
                        add(cards);
                    }
                }
            }
            RequestKind::ChooseColors { source, .. }
            | RequestKind::ChooseSubtype { source, .. }
            | RequestKind::May { source, .. } => add(&[*source]),
            RequestKind::ChoosePlayer { source, subject, .. } => {
                add(&[*source]);
                add(subject.as_slice());
            }
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
            RequestKind::DeclareAttackers { eligible, battles, planeswalkers, .. } => {
                add(battles);
                add(planeswalkers);
                add(eligible);
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

/// An optional cost to attack (CR 508.1g): a cost a player may pay "as" a creature attacks, chosen once the attackers are
/// chosen and tapped (CR 508.1f), locked in with every other cost to attack (CR 508.1h) and paid with them (CR 508.1j). The
/// ability that makes it is a static ability of the attacker, and the triggered ability printed with it ("When you do,
/// ...") is linked to it (CR 607.2h) and triggers when it is paid: the engine keeps both as that one triggered ability
/// (`TriggeredDef::attack_option`), so each instance has its own cost and its own trigger (CR 702.154d).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackOption {
    /// Enlist (CR 702.154a-c): tap up to one untapped creature the attacking player controls that they didn't choose to attack
    /// with and that either has haste or has been under their control continuously since the turn began. A creature tapped
    /// for one enlist ability can't be tapped for another (the rulings): each is tapped for one at most.
    Enlist,
    /// "You may exert this creature as it attacks" (CR 701.43d): it won't untap during its controller's next untap step
    /// (CR 701.43a).
    Exert,
}

/// The rules of a declaration of blockers past which blocks are allowed (CR 509.1b-d), held in the question so the game-log
/// tree moves through them: each attacker's fewest and most blockers (menace, "can't be blocked by more than one creature");
/// each requirement an effect puts on the declaration (CR 509.1c), among the blocks the restrictions allow; what blocks cost
/// (CR 509.1d); the most requirements any declaration that obeys every restriction obeys without paying a cost, which the
/// declaration must obey too; and one such declaration, the engine's suggestion. `judge` (crate::blocks) says what they make
/// of any declaration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockRequirements {
    /// The requirements (CR 509.1c), one for each effect or static ability and each creature it is about: two Lures on a
    /// creature make two of each creature able to block it; "target creature blocks this turn if able" one of that creature,
    /// for any attacker.
    pub required: Vec<RequiredBlock>,
    pub most: u32,
    pub suggested: Vec<(ObjectId, ObjectId)>,
    /// The attacking bands (CR 702.22h), each attacker in one with its band's number; empty where no attacker is banded.
    pub bands: Vec<(ObjectId, u32)>,
    /// Each attacker offered with the fewest creatures that may block it, if any do (2 for menace, CR 702.111b), and the
    /// most (`u32::MAX` for any number; 1 for "can't be blocked by more than one creature", CR 509.1b).
    pub limits: Vec<(ObjectId, u32, u32)>,
    /// What each block that costs something costs (CR 509.1d); a block with no entry is free. A requirement is never obeyed
    /// by paying (CR 509.1c): `most` counts free blocks only. Empty until an effect makes blocking cost something.
    pub costs: Vec<BlockCost>,
}

/// One requirement on a declaration of blockers (CR 509.1c), obeyed by a block that matches it: `blocker` blocks `attacker`
/// if able (Lure); with no attacker, `blocker` blocks if able, whichever attacker it blocks ("target creature blocks this turn
/// if able"); with no blocker, `attacker` is blocked if able, by any creature ("must be blocked if able"). At least one of the
/// two is named. An attacker is blocked through its band too (CR 702.22h).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequiredBlock {
    pub blocker: Option<ObjectId>,
    pub attacker: Option<ObjectId>,
}

/// What declaring `blocker` as a blocker of `attacker` costs (CR 509.1d): the sum of what every effect that makes that block
/// cost something asks, with the first permanent asking it, which the payment is shown as being for. Paid as attack costs
/// are (CR 509.1d-f, 508.1h-j: mana abilities, then the total at once), through the same payment question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockCost {
    pub blocker: ObjectId,
    pub attacker: ObjectId,
    pub cost: ManaCost,
    pub source: ObjectId,
}

impl BlockRequirements {
    /// Whether a creature blocking `declared` blocks `attacker` too: the same creature, or one in the same attacking band (a
    /// creature blocking one member of a band blocks every member, CR 702.22h).
    pub fn blocks_through(&self, declared: ObjectId, attacker: ObjectId) -> bool {
        let band = |a: ObjectId| self.bands.iter().find(|b| b.0 == a).map_or(0, |b| b.1);
        declared == attacker || (band(declared) != 0 && band(declared) == band(attacker))
    }

    /// Whether `blocker` blocking `declared` obeys `r`.
    pub fn obeys(&self, r: &RequiredBlock, blocker: ObjectId, declared: ObjectId) -> bool {
        r.blocker.is_none_or(|b| b == blocker) && r.attacker.is_none_or(|a| self.blocks_through(declared, a))
    }

    /// How many of the requirements a declaration obeys (CR 509.1c): each one some block matches, as declared or through the
    /// attacker's band - so one block of a band may obey two.
    pub fn obeyed(&self, blocks: &[(ObjectId, ObjectId)]) -> u32 {
        crate::count_u32(self.required.iter().filter(|r| blocks.iter().any(|&(b, x)| self.obeys(r, b, x))).count())
    }

    /// The fewest and most creatures that may block `attacker` (1 and any number for one not listed).
    pub fn limits_of(&self, attacker: ObjectId) -> (u32, u32) {
        self.limits.iter().find(|l| l.0 == attacker).map_or((1, u32::MAX), |l| (l.1, l.2))
    }

    /// Whether `blocker` blocking `attacker` costs nothing to declare (CR 509.1d).
    pub fn free(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        !self.costs.iter().any(|c| c.blocker == blocker && c.attacker == attacker)
    }

    /// What declaring `blocks` costs in all (CR 509.1d): each block's own cost, added up.
    pub fn total_cost(&self, blocks: &[(ObjectId, ObjectId)]) -> ManaCost {
        let mut total = ManaCost::ZERO;
        for &(b, a) in blocks {
            if let Some(c) = self.costs.iter().find(|c| c.blocker == b && c.attacker == a) {
                total = total.plus(&c.cost);
            }
        }
        total
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

/// One part of a selection rule (`Selection`), or the rule of a target slot's targets together (`TargetSpec::together`):
/// a validator over the cards chosen so far, with the one number it is parameterised by.
///
/// **Every part is downward-closed**: any of the cards of a set it allows are a set it allows too (a quality of each card,
/// "with different names", a total that only grows as cards are added). The core relies on it - adding only addable cards
/// reaches every allowed set, and a set's reachability is the size of the largest one (`largest`) - so a part that is not
/// (one that asks for a total of at least N, or for the cards to share a quality) is no part, and is written another way. The validator is a `cantrip_api::select::SelectFn` - a plain function, never a closure, so the rule is
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
    /// For a rule of the cards together (a set rule: "with different mana values", "with total mana value N or less"), a
    /// `cantrip_api::select::LargestFn` that finds one of the largest sets it allows among any cards, exactly and in
    /// polynomial time: what decides whether a target slot's fewest targets can be chosen together at all (CR 601.2c). A
    /// target slot's rule must have it (`TargetSpec::together`); a quality of each card alone has none (it is the slot's
    /// filter's to ask).
    largest: Option<&'static (dyn std::any::Any + Send + Sync)>,
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
        Part { judge, n, words, tally, largest: None }
    }

    /// A set rule's part: `erased`, with its `cantrip_api::select::LargestFn` (`cantrip_api::select::set_part`).
    #[doc(hidden)]
    pub const fn erased_set(
        judge: &'static (dyn std::any::Any + Send + Sync),
        largest: &'static (dyn std::any::Any + Send + Sync),
        n: u32,
        words: &'static str,
        tally: Tally,
    ) -> Part {
        Part { judge, n, words, tally, largest: Some(largest) }
    }

    /// Whether it is a rule of the cards together that can say how large a set it allows (`largest`).
    pub const fn is_set_rule(&self) -> bool {
        self.largest.is_some()
    }

    /// Its `cantrip_api::select::LargestFn`, as `dyn Any`, for a set rule.
    #[doc(hidden)]
    pub fn largest(&self) -> Option<&'static (dyn std::any::Any + Send + Sync)> {
        self.largest
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
    /// No parts: a rule that asks nothing.
    pub const NONE: Parts = Parts::Static(&[]);

    /// Whether it has no part.
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

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
    /// Into a graveyard, as one of the cards of the distribution: which cards, not their order, which the rules ask of their
    /// owner as they arrive when it can matter (`GraveyardOrder`).
    Graveyard,
    /// The order of cards put into their owner's graveyard together (CR 404.3), the first on top: the rules' own question
    /// (`ops::move_together`), its one destination taking every card.
    GraveyardOrder,
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
    /// The creature an attacking creature's enlist ability taps (CR 702.154a, 508.1g), the request's source being that attacker:
    /// up to one, none chosen being the cost not paid.
    Enlist,
    Discard,
    Sacrifice,
    /// A card chosen from a revealed hand (Duress).
    TakeFromHand,
    /// Which objects go in the first pile (CR 700.3).
    SeparatePiles,
    /// Which objects go in the face-down pile of two, a face-down pile and a face-up pile (CR 700.3): the rest are the face-up
    /// pile. Riddles in the Dark.
    SeparateFaceDown,
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
    /// Whether to choose another player to help pay for the spell with assist (CR 702.132a).
    Assist,
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
    /// A permanent chosen as the source enters, which its text then refers to as "the chosen [permanent]" (CR 614.12a,
    /// 607.2d): Dauntless Bodyguard's "choose another creature you control". Chosen, not targeted; nothing happens to it.
    ChooseAsEnters,
    /// Objects chosen as an effect resolves, without targeting, that nothing happens to yet: the effect's later words refer to
    /// them as "the chosen" ones (CR 608.2d, 608.2c). Spry and Mighty's "choose exactly two creatures you control" and
    /// Final Showdown's "choose a creature you control. It gains indestructible until end of turn" print it. (A choice of what
    /// is kept, the rest sacrificed - Tragic Arrogance, Cataclysm - is [`Purpose::Keep`].)
    Choose,
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

/// One option's meaning: usually an index into fixed text; a pile carries the objects the chooser sees.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Label {
    /// The nth label of the source card's definition.
    Card(CardId, u8),
    /// The nth label of a card's definition as one of the words its text offers to choose between (CR 607.2f): "choose
    /// Jeskai or Temur", "choose odd or even". Worded as `Card`; a question of these alone is captioned with all of them.
    Option(CardId, u8),
    /// A card's name: an option that is one of several cards, named as the C# engine offers them.
    Name(CardId),
    /// One of two piles (CR 700.3), whose members the chooser sees. Fact or Fiction and Liliana of the Veil ask it.
    Pile { first: bool, cards: Vec<ObjectId> },
    /// The face-up pile of a face-down pile and a face-up pile (CR 700.3), whose members everyone sees: Riddles in the Dark.
    FaceUpPile { cards: Vec<ObjectId> },
    /// The face-down pile of a face-down pile and a face-up pile (CR 700.3): only how many cards it holds is shown to a chooser
    /// who may not look at them, so nothing of what they are is here. Riddles in the Dark.
    FaceDownPile { count: u16 },
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
    /// The permission a spell is cast by at a time a sorcery couldn't have been cast (CR 601.3, 307.5), where its card's text
    /// asks which (Necromancy's cycle, `CardDef::untimely_cleanup_rider`): its own "you may cast this spell as though it had
    /// flash" (`own`), or another one - `by` the card that gives it, where a card does (Valley Floodcaller), none for a rule
    /// of the spell's own (an instant, flash).
    CastPermission { own: bool, by: Option<CardId> },
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

