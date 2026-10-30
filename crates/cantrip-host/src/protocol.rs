//! Cantrip's wire vocabulary, mirrored: what `src/Cantrip.Protocol` defines in C#, type for type and
//! field for field, so the host emits exactly what the server already sends its clients and the C#
//! side translates nothing.
//!
//! Serialisation is JSON: PascalCase fields as the C# properties are named, enums as their member
//! names, `ViewHandle` as a bare integer, and every union tagged with `$type` naming the C# class.
//! `Cantrip.Server` (C#, `NativeRulesHost`) deserialises these into the real `Cantrip.Protocol` types.
//!
//! Keep this file in step with the C# by hand; a drift test on the C# side compares the two.

use serde::{Deserialize, Serialize};

// --- Handles ---

/// How one viewer refers to one object. Allocated per viewer by `handles::HandleSpace`, never the
/// engine's object id: an id is stable for an object's life and would let a client follow a card
/// through a shuffle. Zero is "none".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ViewHandle(pub u32);

impl ViewHandle {
    pub const NONE: ViewHandle = ViewHandle(0);

    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}

// --- Vocabulary (Vocabulary.cs) ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurnStepView {
    Untap,
    Upkeep,
    Draw,
    PrecombatMain,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndOfCombat,
    PostcombatMain,
    End,
    Cleanup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhaseView {
    Beginning,
    PrecombatMain,
    Combat,
    PostcombatMain,
    Ending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneKindView {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    Command,
    Sideboard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CardTypeView {
    Artifact,
    Battle,
    Creature,
    Enchantment,
    Instant,
    Kindred,
    Land,
    Planeswalker,
    Sorcery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SupertypeView {
    Basic,
    Legendary,
    Ongoing,
    Snow,
    World,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ColorView {
    White,
    Blue,
    Black,
    Red,
    Green,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ManaTypeView {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
}

impl ManaTypeView {
    /// Index into `PlayerSummaryView::mana`, in the order the C# enum numbers them.
    pub fn index(self) -> usize {
        self as usize
    }

    pub const ALL: [ManaTypeView; 6] = [
        ManaTypeView::White,
        ManaTypeView::Blue,
        ManaTypeView::Black,
        ManaTypeView::Red,
        ManaTypeView::Green,
        ManaTypeView::Colorless,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            ManaTypeView::White => "{W}",
            ManaTypeView::Blue => "{U}",
            ManaTypeView::Black => "{B}",
            ManaTypeView::Red => "{R}",
            ManaTypeView::Green => "{G}",
            ManaTypeView::Colorless => "{C}",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerStatusView {
    #[default]
    Active,
    Lost,
    Conceded,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DayNightView {
    #[default]
    Neither,
    Day,
    Night,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameOutcomeView {
    #[default]
    InProgress,
    TeamWon,
    Draw,
}

/// Why a game is a draw (CR 104.4): `None` for a game that was won, or a draw the engine gave no reason for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DrawReasonView {
    #[default]
    None,
    /// Every player left lost at once (CR 104.4a).
    AllLost,
    /// A loop every player only passed in (CR 104.4b; `docs/rules-exceptions.md`, RX-4).
    MandatoryLoop,
    /// The game reached the most events the engine records (`docs/rules-exceptions.md`, RX-1).
    EventLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LossReasonView {
    Life,
    EmptyLibrary,
    Poison,
    Conceded,
    Effect,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectKindView {
    #[default]
    Card,
    Token,
    SpellCopy,
    AbilityOnStack,
    Emblem,
    CardCopy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionKindView {
    PlayLand,
    CastSpell,
    ActivateAbility,
    UndoManaAbility,
    CancelCasting,
    PayCost,
    DeclineCost,
    AssignMana,
    IncreaseX,
    DecreaseX,
    ConfirmCasting,
    TapToPay,
    TurnFaceUp,
    ExileToPay,
    PayLifeForSymbol,
    TakeCompanion,
    Plot,
    Suspend,
    Foretell,
    /// Unlocks a locked door of the named Room by paying that door's mana cost (CR 709.5e). A special action, at sorcery
    /// timing. `ability_index` says which door: the option's `description` names it and its cost.
    UnlockDoor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardChoicePurposeView {
    #[default]
    Discard,
    PutOnTop,
    KeepLegend,
    Sacrifice,
    Exile,
    ReturnToHand,
    Untap,
    PutOntoBattlefield,
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
    Encode,
    /// What a cost beholds (CR 701.4a): a permanent chosen or a card revealed, nothing lost.
    Behold,
    /// The creature a cost puts -1/-1 counters on (CR 701.68a).
    Blight,
    CastFree,
    CastCopyFree,
    /// Up to one untapped creature tapped as a cost, its power coming off the generic mana of the total cost (harmonize, CR 702.180).
    TapToReduce,
    /// The permanent that gets counters, chosen without targeting: `Counter` and `CounterCount` say which and how many
    /// (empower Jace's loyalty counters, amass's +1/+1 counters).
    PutCounters,
    /// Craft's materials (CR 702.167a): permanents you control and cards in your graveyard, of the quality `Kind` names,
    /// exiled with the crafting permanent to pay the cost; `Minimum` to `Count` of them.
    Craft,
    /// A card chosen to be revealed from among cards looked at; what becomes of it is the effect's to say (Break Out).
    Reveal,
    /// Whose damage a prevention shield prevents first, of two or more sources that would deal damage at the same time to
    /// the shielded permanent or player (CR 615.7): one of the sources. `Source` is what made the shield and
    /// `PreventLeft` how much it has left to prevent.
    PreventDamageFrom,
    /// Permanents an effect taps as it resolves (CR 701.26a), not to pay a cost (Thousand Moons Smithy).
    Tap,
    /// The other attacking creatures that attack in a band with `Source`, a creature with banding (CR 702.22c, 508.1e): any
    /// number with banding and at most one of `WithoutBanding`. None chosen forms no band.
    Band,
    /// Cards in the chooser's hand drawn this turn, for each of which they then pay `Life` life or put it on top of their
    /// library (Sylvan Library): the cards the payments are about, not those that go back.
    PayLifeOrPutOnTop,
    /// A card an effect exiles that becomes plotted (CR 702.170a, 702.170d): its owner may cast it as a sorcery on a later turn
    /// without paying its mana cost (Jace Reawakened). Something gained, not given up.
    Plot,
    /// The creature `Source`'s enlist ability taps as `Source` attacks (CR 702.154a, 508.1g): up to one untapped creature its
    /// controller controls that isn't attacking. Choosing none pays nothing; choosing one, `Source` gets +X/+0 until end of turn,
    /// X that creature's power.
    Enlist,
    /// A permanent chosen as `Source` enters, which its text then refers to as "the chosen [`Kind`]" (CR 614.12a, 607.2d):
    /// Dauntless Bodyguard's "another creature you control". Chosen, not targeted; nothing happens to it now.
    ChooseAsEnters,
    /// Objects chosen as an effect resolves, not targeted, that nothing happens to yet (CR 608.2d): Spry and Mighty's two
    /// creatures.
    Choose,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchDestinationView {
    #[default]
    Hand,
    TopOfLibrary,
    Graveyard,
    Battlefield,
    Exile,
    ExileFaceDown,
    Cast,
    BattlefieldTapped,
    CastFree,
    Found,
    LandTappedElseHand,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardDestinationKindView {
    #[default]
    LibraryTop,
    LibraryBottom,
    Graveyard,
    Hand,
    Exile,
    BattlefieldFaceDown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManaSpendFilterView {
    #[default]
    None,
    CreatureSpells,
    InstantOrSorcerySpells,
    ActivatedAbilities,
    ArtifactSpells,
    LegendarySpells,
    NotNonartifactSpells,
    PlaneswalkerSpells,
    /// Abilities of land sources.
    LandAbilities,
    /// Abilities of artifact sources.
    ArtifactAbilities,
    /// Spells with one of the subtypes named beside it (`spend_subtypes`): "Dragon spells", "a Dragon spell or an Omen
    /// spell".
    SubtypeSpells,
    /// Creature spells with one of the subtypes named beside it: "a creature spell of the chosen type".
    SubtypeCreatureSpells,
    /// Spells that are not creature spells: "a noncreature spell".
    NoncreatureSpells,
    /// Spells with the mana value named beside it (`spend_mana_value`) or greater: "spells with mana value 4 or greater".
    ManaValueSpells,
    /// Spells with the mana value named beside it or greater, or with {X} in their mana costs: "spells with mana value 5
    /// or greater or spells with {X} in their mana costs".
    ManaValueOrXSpells,
    /// Spells cast from exile: "spend this mana only to cast spells from exile".
    SpellsFromExile,
    /// Anything but spells cast from the hand: "this mana can't be spent to cast spells from your hand".
    NotSpellsFromHand,
    /// Abilities of creature sources.
    CreatureAbilities,
    /// Spells with one of the subtypes named beside it, or abilities of sources with one: "Elemental spells or abilities of
    /// Elemental sources".
    SubtypeSpellsOrAbilities,
    /// Creature spells with one of the subtypes named beside it, or abilities of creature sources with one: "a creature
    /// spell of the chosen type or an ability of a creature source of the chosen type".
    SubtypeCreatureSpellsOrAbilities,
    /// Abilities of artifacts: artifact permanents only (CR 109.2), never an artifact card in a hand or graveyard.
    AbilitiesOfArtifacts,
    /// Creature spells, or abilities of creatures: creature permanents only (CR 109.2).
    CreatureSpellsOrAbilitiesOfCreatures,
    /// Creature spells, or abilities of creature sources in any zone (CR 109.2c).
    CreatureSpellsOrAbilities,
    /// Artifact spells, or abilities of artifacts: artifact permanents only (CR 109.2).
    ArtifactSpellsOrAbilitiesOfArtifacts,
    /// Spells with one of the subtypes named beside it, or abilities of permanents with one (CR 109.2): "Elemental spells
    /// or abilities of Elementals".
    SubtypeSpellsOrAbilitiesOfPermanents,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManaSpendEffectView {
    #[default]
    None,
    GainsHaste,
    CantBeCountered,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackTargetKindView {
    #[default]
    Player,
    Planeswalker,
    Battle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneChangeReasonView {
    Draw,
    Play,
    Cast,
    Resolve,
    Discard,
    Destroyed,
    Sacrificed,
    Exiled,
    Returned,
    Milled,
    StateBasedAction,
    Countered,
    Searched,
    PutOnLibrary,
    Created,
    Surveilled,
    ShuffledIn,
    Manifested,
    PutIntoHand,
}

// --- The view (PlayerView.cs) ---

/// Everything one player is entitled to see, and nothing else. Seats are numbered from one.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerView {
    pub viewer: i32,
    pub turn: i32,
    pub step: Option<TurnStepView>,
    pub active_player: i32,
    /// Who holds priority, or zero while nobody does.
    pub priority_player: i32,
    pub players: Vec<PlayerSummaryView>,
    /// Objects this viewer can observe, in handle order.
    pub objects: Vec<ObjectView>,
    pub zones: Vec<ZoneView>,
    pub combat: CombatView,
    pub outcome: GameOutcomeView,
    pub winning_team: i32,
    pub day_night: DayNightView,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerSummaryView {
    pub seat: i32,
    pub name: String,
    pub team: i32,
    pub life: i32,
    pub status: PlayerStatusView,
    pub poison_counters: i32,
    pub hand_count: i32,
    pub library_count: i32,
    pub lands_played_this_turn: i32,
    /// Unspent unrestricted mana, indexed by `ManaTypeView`.
    pub mana: [i32; 6],
    pub restricted_mana: Vec<RestrictedManaView>,
    pub is_monarch: bool,
    pub has_initiative: bool,
    pub dungeon: String,
    pub dungeon_room: String,
    pub dungeons_completed: i32,
    pub energy: i32,
    pub has_citys_blessing: bool,
    /// Counters on the player by name (CR 122.1), poison and energy apart.
    pub counters: Vec<CounterView>,
    /// Effects waiting on this player that no object carries, one line each: "The next spell you cast this turn can't be
    /// countered" (CR 611.2c). Public, like the permanent effects on the table.
    pub pending_effects: Vec<String>,
    /// An enduring story (CR 702.195b): public, as the city's blessing is.
    pub has_enduring_story: bool,
    /// The player's speed (CR 702.179), 1 to 4; none while they have no speed (CR 702.179b). Public.
    pub speed: Option<i32>,
    /// The seat making this player's choices and decisions now (CR 723.5), while another player controls them; 0 while
    /// they make their own. Public, as the effect is.
    pub controlled_by: i32,
    /// The companion this player revealed before the game (CR 103.2b), while it is still outside the game, revealed: an
    /// object of every seat's view, in its owner's sideboard. None once it has been put into the game, or with none revealed.
    pub companion: ViewHandle,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RestrictedManaView {
    pub r#type: Option<ManaTypeView>,
    pub amount: i32,
    pub filter: ManaSpendFilterView,
    pub effect: ManaSpendEffectView,
    /// For `SubtypeSpells` and `SubtypeCreatureSpells`: the subtypes, any one of which the spell must have; empty for a
    /// chosen type never chosen, which no spell has.
    pub spend_subtypes: Vec<String>,
    /// For `ManaValueSpells` and `ManaValueOrXSpells`: the least mana value the spell may have; 0 otherwise.
    pub spend_mana_value: i32,
    /// The permanent that produced it, for ordinary mana a rule tells apart by its source (CR 106.3: "mana produced by
    /// Barracks of the Thousand"), which `AssignMana` options name as their `Source`; none otherwise.
    pub source: ViewHandle,
}

/// One object the viewer can see. Characteristics are computed, never printed.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectView {
    pub handle: ViewHandle,
    pub zone: Option<ZoneKindView>,
    /// Seat owning the zone, or zero for a shared zone.
    pub zone_owner: i32,
    pub kind: ObjectKindView,
    pub owner: i32,
    pub controller: i32,
    /// False when the viewer can see that something is there but not what it is.
    pub known: bool,
    /// Content id, for art and oracle text lookup. Empty when not known to the viewer.
    pub card_id: String,
    pub name: String,
    /// `{1}{R}`; `{0}` for a cost of zero; empty for no mana cost at all (CR 202.1b).
    pub mana_cost: String,
    pub mana_value: i32,
    /// Rules text as it currently reads (CR 109.3).
    pub oracle_text: String,
    pub supertypes: Vec<SupertypeView>,
    pub card_types: Vec<CardTypeView>,
    pub subtypes: Vec<String>,
    pub colors: Vec<ColorView>,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub tapped: bool,
    pub face_down: bool,
    pub damage_marked: i32,
    pub counters: Vec<CounterView>,
    pub summoning_sick: bool,
    /// The other seats that know this card although it is in a zone they cannot look into.
    pub known_by: Vec<i32>,
    /// What this spell or ability on the stack is aimed at, in printed order.
    pub targets: Vec<SpellTargetView>,
    pub exiled_with: ViewHandle,
    pub produces_mana: bool,
    pub face_down_name: String,
    pub monstrous: bool,
    pub attached_to: ViewHandle,
    pub choices: Vec<ChoiceView>,
    pub exerted: bool,
    pub saddled: bool,
    pub goaded: bool,
    /// The attacking band it is in (CR 702.22c), numbered from 1; 0 for none.
    pub band: i32,
    pub other_face: Option<Box<ObjectView>>,
    pub harnessed: bool,
    pub attached_to_player: i32,
    pub granted_abilities: Vec<AbilityTextView>,
    pub spent_lines: Vec<i32>,
    pub fingerprint: i64,
    pub is_copy: bool,
    pub copying_card: Option<Box<ObjectView>>,
    pub prepare_spell: Option<Box<ObjectView>>,
    pub chosen_modes: Vec<String>,
    pub class_level: i32,
    pub protector: i32,
    pub halves: Vec<ObjectView>,
    /// For a Room on the battlefield (CR 709.5): whether its left and right doors are unlocked, in that order. Empty for
    /// anything else. A locked door is unlocked by an activation offered on the Room (CR 709.5e).
    pub unlocked_doors: Vec<bool>,
    pub starting_counter: Option<i32>,
    /// It is every creature type (Changeling, CR 702.73a): `subtypes` then holds all of them, for a client that asks "is it a
    /// Goblin?", and `type_line_subtypes` what its type line prints.
    pub every_creature_type: bool,
    pub type_line_subtypes: Vec<String>,
    /// Permissions to play this card from where it is (CR 601.3, 305.1): an impulse draw's "this turn", a flashback.
    pub play_permissions: Vec<PlayPermissionView>,
    /// A spell that can't be countered because of what was done as it was cast (CR 101.2, 701.6a).
    pub cant_be_countered: bool,
    /// The abilities it printed and has lost (CR 613.1f), as they read; `granted_abilities` holds those it was given.
    pub lost_abilities: Vec<String>,
    /// For an adventurer or omen card, the Adventure or Omen printed in its inset frame (CR 715.2, 720.2), as printed: the
    /// spell it may be cast as. None on the stack for the spell cast as that face, which is the face itself.
    pub inset_spell: Option<Box<ObjectView>>,
    /// For a prototype card, its inset frame's keyword line as printed, "Prototype {2}{R} — 3/2" (CR 718.1): part of the
    /// card in every zone, though it has those values only while prototyped (CR 718.4). Empty for any other card.
    #[serde(default)]
    pub prototype: String,
    /// A prototyped spell or permanent, or a copy of one (CR 718.3b-d): its mana cost, power, toughness and colors are the
    /// prototype's (`mana_cost`, `power`, `toughness`, `colors` say them), everything else the card's.
    #[serde(default)]
    pub prototyped: bool,
    /// A permanent with the solved designation (CR 719.3b): a Case whose "To solve" ability has made it solved.
    pub solved: bool,
    /// A suspected permanent (CR 701.60b): it has menace and "This creature can't block" while it is.
    pub suspected: bool,
    /// A phased-out permanent (CR 702.26b): still a permanent, known to the players, but treated as though it does not exist
    /// until it phases in.
    pub phased_out: bool,
    /// The power and toughness box a station card prints in its striation (CR 721.2b), "2/2": what it will be once it has
    /// enough charge counters, drawn there in any zone. Presentation, not a characteristic (CR 721.2c); empty for any other card.
    pub station_pt: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlayPermissionView {
    /// The seat that may play the card.
    pub seat: i32,
    /// What it may do and until when, as it reads: "Playable this turn".
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AbilityTextView {
    pub text: String,
    pub spent: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SpellTargetView {
    /// Set when the target is a player; zero otherwise.
    pub player: i32,
    pub object: ViewHandle,
    pub text: String,
    pub still_legal: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChoiceView {
    pub key: String,
    pub value: String,
    /// The permanent chosen, for a choice of one (`creature`, `permanent`): what the client points at. None otherwise.
    pub object: ViewHandle,
}

/// A kind of counter and how many of it. The name is the card's, lower case: `+1/+1`, `stun`, `lore`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CounterView {
    pub name: String,
    pub count: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ZoneView {
    pub kind: Option<ZoneKindView>,
    pub owner: i32,
    pub count: i32,
    /// Contents in zone order for zones the viewer can look into, empty otherwise.
    pub contents: Vec<ViewHandle>,
    pub contents_visible: bool,
    pub known: Vec<KnownCardView>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KnownCardView {
    /// Cards above it in a library, zero for the top; -1 in a zone with no order.
    pub from_top: i32,
    pub card: ObjectView,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CombatView {
    pub attackers: Vec<AttackerView>,
    pub blocks: Vec<BlockView>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AttackerView {
    pub attacker: ViewHandle,
    pub target_kind: AttackTargetKindView,
    pub target_player: i32,
    pub target_permanent: ViewHandle,
    pub was_blocked: bool,
    /// The attacking band it is in (CR 702.22c), numbered from 1; 0 for none. Its band is blocked as one (CR 702.22h).
    pub band: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlockView {
    pub attacker: ViewHandle,
    /// Blockers in damage assignment order (CR 509.2).
    pub blockers: Vec<ViewHandle>,
}

// --- Prompts (Messages.cs) ---

/// Something a spell may target: a player, or an object by handle.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TargetMessage {
    pub player: i32,
    pub object: ViewHandle,
}

impl TargetMessage {
    pub fn player(seat: i32) -> Self {
        TargetMessage { player: seat, object: ViewHandle::NONE }
    }

    pub fn object(handle: ViewHandle) -> Self {
        TargetMessage { player: 0, object: handle }
    }

    pub fn is_player(&self) -> bool {
        self.player != 0
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HybridSymbolView {
    pub colors: Vec<ColorView>,
    /// The life that pays this symbol instead of mana (CR 107.4f), or zero for plain hybrid.
    pub life_cost: i32,
}

/// One thing a player may do at a priority, pay-costs or unless-pays prompt: a prototype the client
/// fills in, never a flat enumeration of concrete actions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionOptionMessage {
    pub kind: ActionKindView,
    pub source: ViewHandle,
    /// For an activation, which of the source's abilities. For a cast, 0 for the mana cost and n for
    /// the card's nth alternative cost (CR 118.9).
    pub ability_index: i32,
    /// Which option of its question this is, from 1: the answer that takes it echoes it (`TakeActionAnswer::option_id`), so
    /// two options alike in kind, source and ability index - a card two permanents' permissions would each let be cast -
    /// are told apart. Opaque: it means nothing outside its question, and says nothing of what makes the option possible.
    #[serde(default)]
    pub option_id: i32,
    pub targets_required: i32,
    pub target_candidates: Vec<TargetMessage>,
    /// For a mana ability: the mana it would add.
    pub produces: Vec<ManaTypeView>,
    pub affordable: bool,
    pub produces_of_any_one_color: i32,
    pub has_x: bool,
    pub max_x: i32,
    /// For an activation: how the ability reads, cost and all. For showing; never parsed by a client.
    pub description: String,
    pub life_cost: i32,
    pub damage_to_controller: i32,
    pub color_choices: Vec<ColorView>,
    pub cost_text: String,
    pub targets_optional: bool,
    pub filter: ManaSpendFilterView,
    pub effect: ManaSpendEffectView,
    /// The subtypes `filter` names, as `RestrictedManaView::spend_subtypes`.
    pub spend_subtypes: Vec<String>,
    /// The mana value `filter` names, as `RestrictedManaView::spend_mana_value`.
    pub spend_mana_value: i32,
    pub spell_name: String,
    pub produces_of_any_color_each: i32,
    pub sacrifice_candidates: Vec<ViewHandle>,
    pub tap: Option<ViewHandle>,
    pub pays_up_to: i32,
    pub alternative: i32,
    pub life_gained: i32,
    pub granted_by: String,
    /// For a mana ability offered while a cost is paid: its mana is restricted so that none of it may pay this cost
    /// (CR 106.6).
    pub pays_nothing: bool,
    /// For a cast: a way or an alternative cost a permanent gives that casts the spell without paying its mana cost (CR 118.9,
    /// Omniscience, Dracogenesis).
    /// `cost_text` is what it comes to with the cost changes in play, before any target ({0}, or {1} under Thalia).
    pub without_paying_mana_cost: bool,
    /// For a cast or an activation: its cost returns tapped permanents, chosen once the mana is paid (CR 601.2g-h, 602.2b:
    /// web-slinging's "return a tapped creature you control", Urban Retreat's), so one tapped for mana while paying may be
    /// one of them, and a payment that leaves too few tapped is taken back.
    pub tapped_after_mana: bool,
    /// For a cast: it casts the card face down, by morph or disguise (CR 702.37c, 702.168b), as a 2/2 face-down creature
    /// spell for {3} (`cost_text`); `description` is the keyword line ("Morph {4}{W}"). Its controller alone sees what it is
    /// on the stack (CR 708.5).
    #[serde(default)]
    pub cast_face_down: bool,
    /// For a cast: it casts the card prototyped (CR 702.160a, 718.3), as its prototype's mana cost (`cost_text`), power and
    /// toughness; `description` is the keyword line ("Prototype {2}{R} — 3/2").
    #[serde(default)]
    pub cast_prototyped: bool,
}

impl ActionOptionMessage {
    pub fn new(kind: ActionKindView, source: ViewHandle) -> Self {
        ActionOptionMessage {
            kind,
            source,
            ability_index: 0,
            option_id: 0,
            targets_required: 0,
            target_candidates: Vec::new(),
            produces: Vec::new(),
            affordable: true,
            produces_of_any_one_color: 0,
            has_x: false,
            max_x: -1,
            description: String::new(),
            life_cost: 0,
            damage_to_controller: 0,
            color_choices: Vec::new(),
            cost_text: String::new(),
            targets_optional: false,
            filter: ManaSpendFilterView::None,
            effect: ManaSpendEffectView::None,
            spend_subtypes: Vec::new(),
            spend_mana_value: 0,
            spell_name: String::new(),
            produces_of_any_color_each: 0,
            sacrifice_candidates: Vec::new(),
            tap: None,
            pays_up_to: 0,
            alternative: 0,
            life_gained: 0,
            granted_by: String::new(),
            without_paying_mana_cost: false,
            tapped_after_mana: false,
            pays_nothing: false,
            cast_face_down: false,
            cast_prototyped: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CardDestinationMessage {
    pub kind: CardDestinationKindView,
    pub owner: i32,
    pub min: i32,
    /// The most that may be put here, or `i32::MAX` for "and the rest".
    pub max: i32,
    pub eligible: Vec<ViewHandle>,
    pub ordered: bool,
    pub randomized: bool,
    pub groups: Vec<Vec<ViewHandle>>,
    pub group_names: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TargetShareMessage {
    pub target: TargetMessage,
    pub amount: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AttackMessage {
    pub attacker: ViewHandle,
    /// The seat attacked, when `target_permanent` is none.
    pub target_player: i32,
    pub target_permanent: ViewHandle,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlockMessage {
    pub blocker: ViewHandle,
    pub attacker: ViewHandle,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AttackRequirementMessage {
    pub attacker: ViewHandle,
    /// The seat it must attack, or 0: with `target_permanent` none as well, it must attack, whatever it attacks.
    pub target_player: i32,
    /// The planeswalker or battle it must attack, or none.
    pub target_permanent: ViewHandle,
    /// A seat it must attack a player other than (a goad's, CR 701.15b), or 0.
    pub other_than: i32,
}

/// What the rules make of a declaration of attackers (CR 508.1a-d): whether it may be declared as it is, why not, what it
/// costs, and the attacks some legal declaration extending it has (docs/host-protocol.md, "Checking a declaration of
/// attackers").
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AttackVerdictView {
    pub complete: bool,
    pub addable: Vec<AttackMessage>,
    pub issue: Option<AttackIssueView>,
    /// What declaring it costs (CR 508.1h), in mana symbols; empty when nothing.
    #[serde(default)]
    pub cost: String,
    /// Nothing is required: any declaration of `addable` attacks, each creature attacking once, is legal, and need not be
    /// asked. False wherever a target caps how many may attack it, as two such attacks need not both be legal.
    #[serde(default)]
    pub free: bool,
}

/// Why a declaration of attackers may not be made as it is, to caption as `Creature`, `Words`, then the seat
/// (`OtherPlayer`) or permanent (`OtherPermanent`) where there is one ("Ursine Monstrosity" "must attack" seat 2), with the
/// rule's number ("508.1d").
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AttackIssueView {
    pub creature: ViewHandle,
    pub words: String,
    pub other_player: i32,
    pub other_permanent: ViewHandle,
    pub rule: String,
}

/// What one attack costs to declare (CR 508.1d, 508.1h): `attacker` attacking the seat `target_player`, or
/// `target_permanent` when that is not none. Attacks that cost nothing have none.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AttackCostMessage {
    pub attacker: ViewHandle,
    pub target_player: i32,
    pub target_permanent: ViewHandle,
    /// The cost in mana symbols, "{2}".
    pub cost: String,
    /// Its mana value, for adding costs up.
    pub mana_value: i32,
    /// Its generic part.
    pub generic: i32,
}

/// One creature's combat damage and where it may go (CR 510.1).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DamageDivisionMessage {
    pub source: ViewHandle,
    pub damage: i32,
    pub creatures: Vec<ViewHandle>,
    pub lethal: Vec<i32>,
    pub may_assign_to_defender: bool,
    pub defender: i32,
    pub suggested: Vec<i32>,
    pub suggested_to_defender: i32,
    pub defender_permanent: ViewHandle,
    /// The creature is not the asked player's (CR 702.22j-k): banding gives them the division of its damage among their own
    /// creatures it is blocked by or is blocking.
    #[serde(default)]
    pub banding: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DamageShareMessage {
    pub source: ViewHandle,
    /// The creature this share goes to, or none for the player the source is attacking.
    pub creature: ViewHandle,
    pub amount: i32,
}

/// What the server asks a player, one of 21 kinds, each the C# class of the same name.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum PromptPayload {
    PriorityPromptMessage(PriorityPrompt),
    DiscardPromptMessage(DiscardPrompt),
    DeclareAttackersPromptMessage(DeclareAttackersPrompt),
    DeclareBlockersPromptMessage(DeclareBlockersPrompt),
    AssignCombatDamagePromptMessage(AssignCombatDamagePrompt),
    SearchLibraryPromptMessage(SearchLibraryPrompt),
    PayCostsPromptMessage(PayCostsPrompt),
    ChooseCardsPromptMessage(ChooseCardsPrompt),
    UnlessPaysPromptMessage(UnlessPaysPrompt),
    DistributeCardsPromptMessage(DistributeCardsPrompt),
    ChooseTargetPromptMessage(ChooseTargetPrompt),
    MayPromptMessage(MayPrompt),
    ChooseModesPromptMessage(ChooseModesPrompt),
    ChooseReplacementPromptMessage(ChooseReplacementPrompt),
    OrderTriggersPromptMessage(OrderTriggersPrompt),
    NameCardPromptMessage(NameCardPrompt),
    ChoosePlayerPromptMessage(ChoosePlayerPrompt),
    ChooseColorsPromptMessage(ChooseColorsPrompt),
    ChooseDividedTargetsPromptMessage(ChooseDividedTargetsPrompt),
    RemoveCountersPromptMessage(RemoveCountersPrompt),
    ChooseStartingPlayerPromptMessage(ChooseStartingPlayerPrompt),
    MulliganPromptMessage(MulliganPrompt),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PriorityPrompt {
    pub player: i32,
    pub options: Vec<ActionOptionMessage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DiscardPrompt {
    pub player: i32,
    pub count: i32,
    pub maximum_hand_size: i32,
    pub hand: Vec<ViewHandle>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeclareAttackersPrompt {
    pub player: i32,
    pub eligible: Vec<ViewHandle>,
    pub attackable_players: Vec<i32>,
    pub attackable_planeswalkers: Vec<ViewHandle>,
    pub attackable_battles: Vec<ViewHandle>,
    pub requirements: Vec<AttackRequirementMessage>,
    pub forbidden: Vec<AttackMessage>,
    /// What each attack that costs something costs (CR 508.1h); the declaration's total is paid once it is declared.
    pub costs: Vec<AttackCostMessage>,
    /// While `costs` has any, the most mana the player could pay (advice; -1 when it is not known).
    pub can_pay: i32,
    /// The requirements' most a declaration must obey (CR 508.1d), for showing the player; the host judges (`verdict`).
    #[serde(default)]
    pub most_requirements: i32,
    /// A declaration that obeys that many, the engine's own; empty where nothing is required.
    #[serde(default)]
    pub suggested: Vec<AttackMessage>,
    /// The rules' verdict on declaring no attackers: whether that may be declared, why not, and the attacks that may be
    /// declared first. Every other declaration is asked of the host (`CheckAttacks`).
    #[serde(default)]
    pub verdict: AttackVerdictView,
}

impl DeclareAttackersPrompt {
    pub fn is_forbidden(&self, attack: &AttackMessage) -> bool {
        self.forbidden.iter().any(|refused| {
            refused.attacker == attack.attacker
                && refused.target_permanent == attack.target_permanent
                && (!refused.target_permanent.is_none() || refused.target_player == attack.target_player)
        })
    }

    /// Every attack `attacker` may be declared making, seats first.
    pub fn attacks_for(&self, attacker: ViewHandle) -> Vec<AttackMessage> {
        self.attackable_players
            .iter()
            .map(|&seat| AttackMessage { attacker, target_player: seat, target_permanent: ViewHandle::NONE })
            .chain(self.attackable_planeswalkers.iter().chain(self.attackable_battles.iter()).map(|&p| AttackMessage {
                attacker,
                target_player: 0,
                target_permanent: p,
            }))
            .filter(|attack| !self.is_forbidden(attack))
            .collect()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeclareBlockersPrompt {
    pub player: i32,
    pub eligible: Vec<ViewHandle>,
    pub attackers: Vec<ViewHandle>,
    pub allowed: Vec<BlockMessage>,
    /// The fewest creatures that may block each of `attackers`, index for index.
    pub minimum_blockers: Vec<i32>,
    /// The most creatures that may block each of `attackers`, index for index (CR 509.1b): 1 for "can't be blocked by more
    /// than one creature", 0 for any number.
    #[serde(default)]
    pub maximum_blockers: Vec<i32>,
    /// The requirements effects put on the declaration (CR 509.1c), one each, for showing the player: a creature that must
    /// block an attacker if able (Lure's "all creatures able to block enchanted creature do so"); with no attacker
    /// (`ViewHandle::NONE`) a creature that must block if able, whichever attacker it blocks ("target creature blocks this turn
    /// if able"); with no blocker an attacker that must be blocked if able, by any creature. The host judges them (`verdict`).
    #[serde(default)]
    pub requirements: Vec<BlockMessage>,
    /// How many of `requirements` the declaration must obey: the most any declaration that obeys every restriction does.
    #[serde(default)]
    pub most_requirements: i32,
    /// A declaration that obeys that many, the engine's own; empty where nothing is required.
    #[serde(default)]
    pub suggested: Vec<BlockMessage>,
    /// The attacking band each of `attackers` is in, index for index, 0 for none (CR 702.22h): a creature blocking one member
    /// of a band blocks every member, so it obeys a requirement to block any of them.
    #[serde(default)]
    pub bands: Vec<i32>,
    /// The rules' verdict on declaring no blockers: whether that may be declared, why not, and the blocks that may be declared
    /// first. Every other declaration is asked of the host (`CheckBlocks`).
    #[serde(default)]
    pub verdict: BlockVerdictView,
}

/// What the rules make of a declaration of blockers (CR 509.1a-c): whether it may be declared as it is, why not, and the
/// blocks some legal declaration extending it has (docs/host-protocol.md, "Checking a declaration of blockers").
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlockVerdictView {
    pub complete: bool,
    pub addable: Vec<BlockMessage>,
    pub issue: Option<BlockIssueView>,
    /// What declaring it costs (CR 509.1d), in mana symbols ("{2}"); empty for a declaration that costs nothing. It is paid
    /// once declared, through the payment prompt attack costs use (CR 509.1d-f, 508.1h-j).
    #[serde(default)]
    pub cost: String,
}

/// Why a declaration of blockers may not be made as it is, to caption as `Creature`, `Words`, then `Other` where there is
/// one ("Grizzly Bears" "must block" "Ox"), with the rule's number ("509.1c").
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlockIssueView {
    pub creature: ViewHandle,
    pub words: String,
    pub other: ViewHandle,
    pub rule: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AssignCombatDamagePrompt {
    pub player: i32,
    pub divisions: Vec<DamageDivisionMessage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SearchLibraryPrompt {
    pub player: i32,
    /// How many the search takes, for its caption, as `count_kind` counts it.
    pub count: i32,
    pub library: Vec<ViewHandle>,
    pub destination: SearchDestinationView,
    pub owner: i32,
    pub count_kind: CountKindView,
    /// The noun the search's rule calls the cards ("basic land"), empty for "card".
    pub kind: String,
    /// What the rule's parts ask, each in the words a caption puts after the noun ("with different names").
    pub words: Vec<String>,
    /// The rule's verdict on finding nothing yet: whether that is an answer, and which cards may be picked first. Every pick
    /// after is asked of the host (`Check`).
    pub verdict: SelectionVerdictView,
}

/// How a choice of cards counts them (`Count`): exactly `Count`, up to it, any number, or `Count` unless fewer can be found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CountKindView {
    #[default]
    Exactly,
    UpTo,
    AnyNumber,
    AsManyAsCan,
}

/// What a question's rule says of a selection of its cards: whether it is an answer, and which further cards may be added.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SelectionVerdictView {
    pub complete: bool,
    pub addable: Vec<ViewHandle>,
    /// In a prompt's first verdict, for a question that takes one card at most: the `addable` cards that are, picked alone, an
    /// answer. A client may send such a pick at once, without asking the host (`Check`); empty otherwise.
    pub complete_alone: Vec<ViewHandle>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayCostsPrompt {
    pub player: i32,
    pub spell: ViewHandle,
    pub total: String,
    pub remaining: String,
    pub remaining_generic: i32,
    pub remaining_colored: Vec<ColorView>,
    pub options: Vec<ActionOptionMessage>,
    pub has_x: bool,
    pub x: i32,
    pub committed: Vec<RestrictedManaView>,
    pub remaining_hybrid: Vec<HybridSymbolView>,
    pub kicker: String,
    pub multikicker: bool,
    pub buyback: bool,
    /// Whether that optional additional cost is squad (CR 702.157a) rather than kicker: `kicker` carries its whole cost in
    /// words (mana and the rest), and a client says "pay squad", not "kick", and does not speak of mana for a cost that has
    /// none.
    pub squad: bool,
    pub announces_x_or_kicks: bool,
    pub most_affordable: i32,
    pub x_paid_in_life: bool,
    pub remaining_colorless: i32,
    pub kicker_paid: bool,
    pub variable_kicker: bool,
    pub spends_no_mana: bool,
    /// The attacking creatures whose cost to attack this pays (CR 508.1h-j); `spell` is then the permanent that makes
    /// attacking cost something. Empty for any other payment.
    pub attackers: Vec<ViewHandle>,
    /// Announcing the X of a "−X" loyalty cost (CR 606.4, 107.3): X loyalty counters come off `spell`, the permanent, and
    /// no mana is paid; `most_affordable` is its loyalty.
    pub x_removes_loyalty: bool,
    /// The life the total cost asks besides its mana (CR 119.4, 601.2h: "an additional 3 life", "pay X life"), paid with it
    /// once the mana is. Zero while X is announced.
    pub life: i32,
    /// Announcing how many permanents the spell's additional cost sacrifices, each taking this much generic mana off its
    /// total ("you may sacrifice any number of nonland permanents. This spell costs {1} less to cast for each permanent
    /// sacrificed this way", CR 601.2b, 601.2f); `x` is the number so far and `most_affordable` how many could be. Zero
    /// for any other announcement.
    pub sacrifice_reduces: i32,
    /// While announcing: the largest value an `AnnounceAnswer` may announce (CR 601.2b) - the rules' limit, as many as the
    /// legal targets an "X target" spell has, once for a kicker paid at most once; how much could be paid for is
    /// `most_affordable`. Zero while paying.
    pub most_announced: i32,
    /// While announcing: the smallest value an `AnnounceAnswer` may announce: 1 for an X that can't be 0 (CR 107.3a, 601.2b),
    /// else 0. The announcement starts at `x`: this, or the X a reduction already announced pays for (the cards a March
    /// exiles from the hand, CR 601.2f). Zero while paying.
    pub least_announced: i32,
    /// While announcing an X that is not paid in mana: what X governs, in words ("the least power of the creature you
    /// sacrifice", casualty X, CR 702.153a), for a caption where "mana for X" would be false. Empty for any other.
    pub x_governs: String,
    /// Paying for a spell with assist (CR 702.132a), the step this question is: "HelperActivates" (the player the caster
    /// chose activates mana abilities; `ConfirmCasting` is "done"), "CasterActivates" (the same, for the caster), "HelperAnnounces"
    /// (the chosen player announces how much of the generic mana they pay, `most_announced` at most), "HelperPays" (they pay
    /// it from their pool), "CasterPays" (the caster pays the rest, no mana ability offered). Empty for any other payment.
    #[serde(default)]
    pub assist_step: String,
    /// The mana other players have paid towards this spell's cost as it is cast: the assisting player's (CR 702.132a).
    #[serde(default)]
    pub assisted: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChooseCardsPrompt {
    pub player: i32,
    pub owner: i32,
    pub source: ViewHandle,
    pub purpose: CardChoicePurposeView,
    pub cards: Vec<ViewHandle>,
    /// How many the choice takes, for its caption, as `count_kind` counts it.
    pub count: i32,
    pub count_kind: CountKindView,
    /// What the rule's parts ask, each in the words a caption puts after the noun ("with total mana value 6 or less").
    pub words: Vec<String>,
    /// The rule's verdict on choosing nothing yet: whether that is an answer, and which cards may be picked first. Every pick
    /// after is asked of the host (`Check`).
    pub verdict: SelectionVerdictView,
    pub kind: String,
    pub minimum_power: i32,
    pub powers: Vec<i32>,
    /// For `TapToReduce` (harmonize, CR 702.180a): each of `powers` is the most tapping that creature could take off the
    /// generic mana of the total cost, which X, a kicker or the targets announced after it may still change (CR 601.2b-f);
    /// false when it is exactly what it takes off.
    pub powers_up_to: bool,
    pub minimum_mana_value: i32,
    pub mana_values: Vec<i32>,
    pub maximum_mana_value: i32,
    /// For `PutCounters`, the kind of counter as Oracle text names it ("loyalty", "+1/+1"); empty otherwise.
    pub counter: String,
    /// For `PutCounters`, how many counters the chosen permanent gets.
    pub counter_count: i32,
    /// The fewest that may be chosen when `CountKind` is `UpTo`: craft's "one or more" (CR 702.167a), for the caption. Zero
    /// when any number up to `Count` will do.
    pub minimum: i32,
    /// "With total mana value N or less" (CR 202.3): the most the chosen cards' `mana_values` add up to, for the client's running
    /// total (`Tally::ManaValueAtMost`); none when the rule keeps no such count. The rule's verdicts judge the cards.
    pub maximum_total_mana_value: Option<i32>,
    /// For `PreventDamageFrom`, how much damage the shield has left to prevent (CR 615.7).
    pub prevent_left: i32,
    /// For `Band`, the eligible creatures without banding, of which a band may have at most one (CR 702.22c).
    pub without_banding: Vec<ViewHandle>,
    /// For `PayLifeOrPutOnTop`, the life each chosen card costs to keep.
    pub life: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UnlessPaysPrompt {
    pub player: i32,
    pub source: ViewHandle,
    pub subject: ViewHandle,
    pub total: String,
    pub remaining: String,
    pub remaining_generic: i32,
    pub remaining_colored: Vec<ColorView>,
    pub options: Vec<ActionOptionMessage>,
    pub remaining_hybrid: Vec<HybridSymbolView>,
    /// What paying does, in card vocabulary (`Question::verb`): "Countered" for "unless pays", "CopySpell", "ReturnToHand".
    pub verb: String,
    /// The life the cost takes besides its mana (CR 119.4): "{W}{B} and 2 life" is `Total` {W}{B} and 2.
    pub life: i32,
    /// The life total no longer covers the life (a mana ability made for the payment cost it): only declining is left.
    pub life_short: bool,
    /// An amount the verb carries (`Question::amount`): the greatest mana value a card returned may have.
    pub amount: i32,
    /// The card's words for the payment, after the source's name, `{cost}` and `{subject}` where the cost and the subject's
    /// name go; `Detail` says more, and `Yes` and `No` name the buttons.
    pub question: String,
    pub detail: String,
    pub yes: String,
    pub no: String,
    /// "You may pay {X}" with X left to the payer (CR 107.3f): X is chosen as it is paid, `X` so far, `Total` the cost at
    /// that X, and `IncreaseX` and `DecreaseX` among the options change it.
    pub choosing_x: bool,
    pub x: i32,
    /// "You may pay {G} up to three times", "any number of times" (CR 603.12a): the number of times is chosen as it is paid,
    /// `X` the number so far (from 1), `Total` the cost that many times over, and `IncreaseX` and `DecreaseX` among the
    /// options change it within 1 to `MostTimes` - 0 for any number of times.
    pub choosing_times: bool,
    pub most_times: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DistributeCardsPrompt {
    pub player: i32,
    pub source: ViewHandle,
    /// The cards looked at, top card of the library first.
    pub cards: Vec<ViewHandle>,
    pub destinations: Vec<CardDestinationMessage>,
    pub may_shuffle: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChooseTargetPrompt {
    pub player: i32,
    pub ability: ViewHandle,
    pub source: ViewHandle,
    pub text: String,
    pub candidates: Vec<TargetMessage>,
    pub optional: bool,
    pub count: i32,
    pub targeted: bool,
    pub minimum: i32,
    pub any_number: bool,
    pub for_player: i32,
    /// "Any number of target creature cards with total mana value X or less" (CR 601.2c, 202.3): the most the chosen
    /// targets' `mana_values` may add up to, a running total the rule keeps for the player (`Tally`, display alone); none
    /// when the slot asks nothing of them.
    pub maximum_total_mana_value: Option<i32>,
    /// Each candidate's mana value, index for index, when `maximum_total_mana_value` is set; empty otherwise.
    pub mana_values: Vec<i32>,
    /// What the rule of the slot's targets together asks, each in the words a caption puts after the noun ("with different
    /// mana values", "with mana value 3 or less"); empty when it asks nothing.
    pub words: Vec<String>,
    /// For a slot whose targets together are judged (`TargetSpec::together`), the rule's verdict on none chosen: whether that
    /// is an answer, and which object candidates may be picked first. Every pick after is asked of the host (`Check`), as a
    /// choice of cards is. None for a slot that asks nothing of its targets together.
    pub verdict: Option<SelectionVerdictView>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MayPrompt {
    pub player: i32,
    pub source: ViewHandle,
    pub verb: String,
    pub amount: i32,
    /// The card's own words for a worded "you may" (`Purpose::Ask`), each empty for one that is not.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub question: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub yes: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub no: String,
    /// Neither answer is the default a player passing through prompts takes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_default: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChooseModesPrompt {
    pub player: i32,
    pub source: ViewHandle,
    pub modes: Vec<String>,
    pub eligible: Vec<i32>,
    pub count: i32,
    pub may_repeat: bool,
    pub additional_cost: bool,
    pub heading: String,
    pub at_most: i32,
    pub affordable: Vec<i32>,
    /// For an optional additional cost that is a keyword action: "Behold" or "Blight"; empty otherwise.
    pub cost_kind: String,
    pub detail: String,
    pub extra_modes_cost: bool,
    pub texts: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChooseReplacementPrompt {
    pub player: i32,
    pub subject: ViewHandle,
    pub event: String,
    pub sources: Vec<ViewHandle>,
    pub hints: Vec<String>,
    /// Each source's name, in step with `sources`: a source that has left (Ral Zarek's skip, CR 704.5i) may have no handle
    /// in a seat's view, and is named by this. Empty for one in a zone the seat may not look into.
    pub source_names: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OrderTriggersPrompt {
    pub player: i32,
    pub sources: Vec<ViewHandle>,
    pub texts: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NameCardPrompt {
    pub player: i32,
    pub source: ViewHandle,
    pub owner: i32,
    pub hand: Vec<ViewHandle>,
    pub names: Vec<String>,
    pub suggestion: String,
    /// What the name must be the name of, in the card's words (CR 201.4a): "a card name", "a nonland card name", "a card
    /// name other than a basic land card name".
    pub kind: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChoosePlayerPrompt {
    pub player: i32,
    pub source: ViewHandle,
    pub candidates: Vec<i32>,
    pub verb: String,
    pub amount: i32,
    /// The object the choice is for, of several the effect treats one at a time (Dack Fayden, Helping Hand's permanents);
    /// no handle otherwise.
    pub subject: ViewHandle,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChooseColorsPrompt {
    pub player: i32,
    pub source: ViewHandle,
    pub choices: Vec<ColorView>,
    pub count: i32,
    pub may_repeat: bool,
    pub verb: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChooseDividedTargetsPrompt {
    pub player: i32,
    pub ability: ViewHandle,
    pub source: ViewHandle,
    pub text: String,
    pub candidates: Vec<TargetMessage>,
    pub total: i32,
    pub max_targets: i32,
    pub optional: bool,
}

/// Remove `Total` counters, of any kinds, from among permanents (CR 122.1): as part of a cost (CR 118.3), "remove three
/// counters from among creatures you control", or as an effect resolves. Each of `Pools` is the counters of one kind on one
/// permanent, with how many there are; the answer takes some from any of them, adding up to `Total` - or, with `UpTo`, an
/// effect's "remove up to three counters from target creature" (CR 608.2d), to any number from none to `Total`. `Source` is
/// the spell or ability.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RemoveCountersPrompt {
    pub player: i32,
    pub source: ViewHandle,
    pub total: i32,
    pub pools: Vec<CounterPoolMessage>,
    pub up_to: bool,
}

/// Counters of one kind on one permanent: `Counter` names the kind as `CounterView` does (`+1/+1`, `stun`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CounterPoolMessage {
    pub object: ViewHandle,
    pub counter: String,
    pub count: i32,
}

impl RemoveCountersPrompt {
    /// As many as there are from each pool in turn, the first offered first, until the total is reached.
    pub fn default_removals(&self) -> Vec<CounterPoolMessage> {
        let mut left = self.total.max(0);
        let mut out = Vec::new();
        for p in &self.pools {
            let n = p.count.min(left);
            if n > 0 {
                out.push(CounterPoolMessage { object: p.object, counter: p.counter.clone(), count: n });
                left -= n;
            }
        }
        out
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChooseStartingPlayerPrompt {
    pub player: i32,
    pub candidates: Vec<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MulliganPrompt {
    pub player: i32,
    pub mulligans: i32,
    pub keeps: i32,
}

// --- Answers ---

/// What a player answers, one of 19 kinds, each the C# class of the same name.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum AnswerPayload {
    PassAnswer(PassAnswer),
    CardsChosenAnswer(CardsChosenAnswer),
    TakeActionAnswer(TakeActionAnswer),
    AttackersDeclaredAnswer(AttackersDeclaredAnswer),
    BlockersDeclaredAnswer(BlockersDeclaredAnswer),
    CombatDamageAssignedAnswer(CombatDamageAssignedAnswer),
    CardsDistributedAnswer(CardsDistributedAnswer),
    TargetsChosenAnswer(TargetsChosenAnswer),
    MayChosenAnswer(MayChosenAnswer),
    ModesChosenAnswer(ModesChosenAnswer),
    ReplacementChosenAnswer(ReplacementChosenAnswer),
    TriggersOrderedAnswer(TriggersOrderedAnswer),
    CancelAnnouncementAnswer(CancelAnnouncementAnswer),
    CardNamedAnswer(CardNamedAnswer),
    PlayerChosenAnswer(PlayerChosenAnswer),
    ColorsChosenAnswer(ColorsChosenAnswer),
    TargetsDividedAnswer(TargetsDividedAnswer),
    CountersRemovedAnswer(CountersRemovedAnswer),
    AnnounceAnswer(AnnounceAnswer),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PassAnswer {}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CancelAnnouncementAnswer {}

/// The value of X, or how many times the spell is kicked, announced at once (CR 601.2b): an answer to a pay-costs prompt
/// that `announces_x_or_kicks`, from its `least_announced` to its `most_announced`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AnnounceAnswer {
    pub value: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CardsChosenAnswer {
    pub cards: Vec<ViewHandle>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TakeActionAnswer {
    pub kind: ActionKindView,
    pub source: ViewHandle,
    pub ability_index: i32,
    /// The option taken (`ActionOptionMessage::option_id`), or 0 to name it by kind, source and ability index alone, which
    /// is refused where two options of the question share them.
    #[serde(default)]
    pub option_id: i32,
    pub targets: Vec<TargetMessage>,
    pub color: Option<ColorView>,
    pub colors: Vec<ColorView>,
    pub sacrifice: Option<ViewHandle>,
    pub tap: Option<ViewHandle>,
    pub mana: Option<ManaTypeView>,
    pub filter: ManaSpendFilterView,
    pub effect: ManaSpendEffectView,
    /// The subtypes `filter` names, as `RestrictedManaView::spend_subtypes`.
    pub spend_subtypes: Vec<String>,
    /// The mana value `filter` names, as `RestrictedManaView::spend_mana_value`.
    pub spend_mana_value: i32,
}

impl TakeActionAnswer {
    pub fn new(kind: ActionKindView, source: ViewHandle) -> Self {
        TakeActionAnswer {
            kind,
            source,
            ability_index: 0,
            option_id: 0,
            targets: Vec::new(),
            color: None,
            colors: Vec::new(),
            sacrifice: None,
            tap: None,
            mana: None,
            filter: ManaSpendFilterView::None,
            effect: ManaSpendEffectView::None,
            spend_subtypes: Vec::new(),
            spend_mana_value: 0,
        }
    }

    /// The answer that takes `option` as offered, targets and the rest still to fill in.
    pub fn take(option: &ActionOptionMessage) -> Self {
        let mut answer = Self::new(option.kind, option.source);
        answer.ability_index = option.ability_index;
        answer.option_id = option.option_id;
        answer.tap = option.tap;
        answer.filter = option.filter;
        answer.effect = option.effect;
        answer.spend_subtypes = option.spend_subtypes.clone();
        answer.spend_mana_value = option.spend_mana_value;
        answer
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AttackersDeclaredAnswer {
    pub attacks: Vec<AttackMessage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlockersDeclaredAnswer {
    pub blocks: Vec<BlockMessage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CombatDamageAssignedAnswer {
    pub shares: Vec<DamageShareMessage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CardsDistributedAnswer {
    pub placements: Vec<Vec<ViewHandle>>,
    pub shuffle: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TargetsChosenAnswer {
    pub targets: Vec<TargetMessage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MayChosenAnswer {
    pub accept: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ModesChosenAnswer {
    pub modes: Vec<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReplacementChosenAnswer {
    pub index: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TriggersOrderedAnswer {
    pub order: Vec<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CardNamedAnswer {
    pub name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerChosenAnswer {
    pub player: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ColorsChosenAnswer {
    pub colors: Vec<ColorView>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TargetsDividedAnswer {
    pub shares: Vec<TargetShareMessage>,
}

/// The counters taken for a `RemoveCountersPromptMessage`: how many of each kind from each permanent.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CountersRemovedAnswer {
    pub removals: Vec<CounterPoolMessage>,
}

/// The first of `eligible`, in order, whose `mana_values` (index for index) fit together within `most`, at most `count` of
/// them: a default answer to a choice "with total mana value N or less" (CR 202.3) always accepted.
fn first_fitting<T: Clone>(eligible: &[T], mana_values: &[i32], count: i32, most: i32) -> Vec<T> {
    let mut left = most;
    let mut taken = Vec::new();
    for (i, item) in eligible.iter().enumerate() {
        let value = mana_values.get(i).copied().unwrap_or(0);
        if taken.len() < count.max(0) as usize && value <= left {
            left -= value;
            taken.push(item.clone());
        }
    }
    taken
}

impl PromptPayload {
    /// The seat being asked.
    pub fn player(&self) -> i32 {
        match self {
            PromptPayload::PriorityPromptMessage(p) => p.player,
            PromptPayload::DiscardPromptMessage(p) => p.player,
            PromptPayload::DeclareAttackersPromptMessage(p) => p.player,
            PromptPayload::DeclareBlockersPromptMessage(p) => p.player,
            PromptPayload::AssignCombatDamagePromptMessage(p) => p.player,
            PromptPayload::SearchLibraryPromptMessage(p) => p.player,
            PromptPayload::PayCostsPromptMessage(p) => p.player,
            PromptPayload::ChooseCardsPromptMessage(p) => p.player,
            PromptPayload::UnlessPaysPromptMessage(p) => p.player,
            PromptPayload::DistributeCardsPromptMessage(p) => p.player,
            PromptPayload::ChooseTargetPromptMessage(p) => p.player,
            PromptPayload::MayPromptMessage(p) => p.player,
            PromptPayload::ChooseModesPromptMessage(p) => p.player,
            PromptPayload::ChooseReplacementPromptMessage(p) => p.player,
            PromptPayload::OrderTriggersPromptMessage(p) => p.player,
            PromptPayload::NameCardPromptMessage(p) => p.player,
            PromptPayload::ChoosePlayerPromptMessage(p) => p.player,
            PromptPayload::ChooseColorsPromptMessage(p) => p.player,
            PromptPayload::ChooseDividedTargetsPromptMessage(p) => p.player,
            PromptPayload::RemoveCountersPromptMessage(p) => p.player,
            PromptPayload::ChooseStartingPlayerPromptMessage(p) => p.player,
            PromptPayload::MulliganPromptMessage(p) => p.player,
        }
    }

    /// The C# class name, which is also the `$type` tag.
    pub fn kind_name(&self) -> &'static str {
        match self {
            PromptPayload::PriorityPromptMessage(_) => "PriorityPromptMessage",
            PromptPayload::DiscardPromptMessage(_) => "DiscardPromptMessage",
            PromptPayload::DeclareAttackersPromptMessage(_) => "DeclareAttackersPromptMessage",
            PromptPayload::DeclareBlockersPromptMessage(_) => "DeclareBlockersPromptMessage",
            PromptPayload::AssignCombatDamagePromptMessage(_) => "AssignCombatDamagePromptMessage",
            PromptPayload::SearchLibraryPromptMessage(_) => "SearchLibraryPromptMessage",
            PromptPayload::PayCostsPromptMessage(_) => "PayCostsPromptMessage",
            PromptPayload::ChooseCardsPromptMessage(_) => "ChooseCardsPromptMessage",
            PromptPayload::UnlessPaysPromptMessage(_) => "UnlessPaysPromptMessage",
            PromptPayload::DistributeCardsPromptMessage(_) => "DistributeCardsPromptMessage",
            PromptPayload::ChooseTargetPromptMessage(_) => "ChooseTargetPromptMessage",
            PromptPayload::MayPromptMessage(_) => "MayPromptMessage",
            PromptPayload::ChooseModesPromptMessage(_) => "ChooseModesPromptMessage",
            PromptPayload::ChooseReplacementPromptMessage(_) => "ChooseReplacementPromptMessage",
            PromptPayload::OrderTriggersPromptMessage(_) => "OrderTriggersPromptMessage",
            PromptPayload::NameCardPromptMessage(_) => "NameCardPromptMessage",
            PromptPayload::ChoosePlayerPromptMessage(_) => "ChoosePlayerPromptMessage",
            PromptPayload::ChooseColorsPromptMessage(_) => "ChooseColorsPromptMessage",
            PromptPayload::ChooseDividedTargetsPromptMessage(_) => "ChooseDividedTargetsPromptMessage",
            PromptPayload::RemoveCountersPromptMessage(_) => "RemoveCountersPromptMessage",
            PromptPayload::ChooseStartingPlayerPromptMessage(_) => "ChooseStartingPlayerPromptMessage",
            PromptPayload::MulliganPromptMessage(_) => "MulliganPromptMessage",
        }
    }

    /// The simplest legal answer, composed only out of what the prompt itself carries: the mirror of
    /// each C# prompt's `Default()`. What every automatic answerer falls back to.
    pub fn default_answer(&self) -> AnswerPayload {
        match self {
            PromptPayload::PriorityPromptMessage(_) => AnswerPayload::PassAnswer(PassAnswer {}),
            PromptPayload::DiscardPromptMessage(p) => AnswerPayload::CardsChosenAnswer(CardsChosenAnswer {
                cards: p.hand.iter().take(p.count.max(0) as usize).copied().collect(),
            }),
            // No attack, or the engine's own declaration where an effect requires attacks (CR 508.1d).
            PromptPayload::DeclareAttackersPromptMessage(p) => {
                AnswerPayload::AttackersDeclaredAnswer(AttackersDeclaredAnswer { attacks: p.suggested.clone() })
            }
            // No blocks, or the engine's own declaration where an effect requires blocks (CR 509.1c).
            PromptPayload::DeclareBlockersPromptMessage(p) => {
                AnswerPayload::BlockersDeclaredAnswer(BlockersDeclaredAnswer { blocks: p.suggested.clone() })
            }
            PromptPayload::AssignCombatDamagePromptMessage(p) => {
                let mut shares = Vec::new();
                for division in &p.divisions {
                    for (i, creature) in division.creatures.iter().enumerate() {
                        let amount = division.suggested.get(i).copied().unwrap_or(0);
                        if amount > 0 {
                            shares.push(DamageShareMessage { source: division.source, creature: *creature, amount });
                        }
                    }
                    if division.suggested_to_defender > 0 {
                        shares.push(DamageShareMessage {
                            source: division.source,
                            creature: ViewHandle::NONE,
                            amount: division.suggested_to_defender,
                        });
                    }
                }
                AnswerPayload::CombatDamageAssignedAnswer(CombatDamageAssignedAnswer { shares })
            }
            // Nothing found where that is an answer (CR 701.23b); otherwise the first cards that may be found, as many as it takes.
            PromptPayload::SearchLibraryPromptMessage(p) => AnswerPayload::CardsChosenAnswer(CardsChosenAnswer {
                cards: if p.verdict.complete { Vec::new() } else { p.verdict.addable.iter().take(p.count.max(0) as usize).copied().collect() },
            }),
            // Giving up on the spell. A question that offers no giving up - the player assisting with another's spell
            // (CR 702.132a), whose spell it is not - closes their window, announces nothing more, or pays with the first
            // pool mana offered: what they announced they must pay.
            PromptPayload::PayCostsPromptMessage(p) => {
                let offered = |kind: ActionKindView| p.options.iter().find(|o| o.kind == kind);
                if offered(ActionKindView::CancelCasting).is_none() {
                    if let Some(o) = offered(ActionKindView::ConfirmCasting) {
                        return AnswerPayload::TakeActionAnswer(TakeActionAnswer::take(o));
                    }
                    if let Some(o) = offered(ActionKindView::AssignMana) {
                        let mut answer = TakeActionAnswer::take(o);
                        answer.mana = o.produces.first().copied();
                        return AnswerPayload::TakeActionAnswer(answer);
                    }
                }
                AnswerPayload::TakeActionAnswer(TakeActionAnswer::new(ActionKindView::CancelCasting, ViewHandle::NONE))
            }
            PromptPayload::ChooseCardsPromptMessage(p) => AnswerPayload::CardsChosenAnswer(CardsChosenAnswer {
                cards: if p.minimum_power > 0 {
                    // "Total power N or more" (crew, saddle): every creature with positive power, enough whenever anything is;
                    // a negative power would take from the total (CR 107.1b).
                    p.cards.iter().enumerate().filter(|&(i, _)| p.powers.get(i).is_none_or(|&power| power > 0)).map(|(_, &c)| c).collect()
                } else if let Some(most) = p.maximum_total_mana_value {
                    // "With total mana value N or less": the first cards that may be picked and fit together, in order (each
                    // one's mana value is at its place among `cards`).
                    let values: Vec<i32> = p.verdict.addable.iter().map(|a| p.cards.iter().position(|c| c == a).and_then(|i| p.mana_values.get(i).copied()).unwrap_or(0)).collect();
                    first_fitting(&p.verdict.addable, &values, p.count, most)
                } else {
                    // The first cards that may be picked, as many as it takes: an answer for a rule of a count alone; one asking
                    // more of the cards together is settled by asking the host as the cards are picked (`Check`).
                    p.verdict.addable.iter().take(p.count.max(0) as usize).copied().collect()
                },
            }),
            PromptPayload::UnlessPaysPromptMessage(_) => {
                AnswerPayload::TakeActionAnswer(TakeActionAnswer::new(ActionKindView::DeclineCost, ViewHandle::NONE))
            }
            PromptPayload::DistributeCardsPromptMessage(p) => AnswerPayload::CardsDistributedAnswer(CardsDistributedAnswer {
                placements: greedy_distribution(p),
                shuffle: false,
            }),
            PromptPayload::ChooseTargetPromptMessage(p) => AnswerPayload::TargetsChosenAnswer(TargetsChosenAnswer {
                targets: if p.optional {
                    Vec::new()
                } else if let Some(most) = p.maximum_total_mana_value {
                    // "With total mana value X or less": the first candidates that fit together, in order.
                    first_fitting(&p.candidates, &p.mana_values, p.count, most)
                } else if let Some(verdict) = &p.verdict {
                    // A rule of the targets together: the first that may be picked, as many as it takes; one asking more of
                    // them together is settled by asking the host as they are picked (`Check`).
                    p.candidates.iter().filter(|c| c.player != 0 || verdict.addable.contains(&c.object)).take(p.count.max(0) as usize).cloned().collect()
                } else {
                    p.candidates.iter().take(p.count.max(0) as usize).cloned().collect()
                },
            }),
            PromptPayload::MayPromptMessage(_) => AnswerPayload::MayChosenAnswer(MayChosenAnswer { accept: false }),
            PromptPayload::ChooseModesPromptMessage(p) => {
                let count = p.count.max(0) as usize;
                let modes = if p.may_repeat {
                    (0..count).map(|i| p.eligible[i.min(p.eligible.len().saturating_sub(1))]).collect()
                } else {
                    // Affordable ones first, then the rest, each group in printed order.
                    let mut preferred: Vec<i32> = p.eligible.iter().copied().filter(|m| p.affordable.is_empty() || p.affordable.contains(m)).collect();
                    preferred.extend(p.eligible.iter().copied().filter(|m| !p.affordable.is_empty() && !p.affordable.contains(m)));
                    preferred.into_iter().take(count).collect()
                };
                AnswerPayload::ModesChosenAnswer(ModesChosenAnswer { modes })
            }
            PromptPayload::ChooseReplacementPromptMessage(_) => AnswerPayload::ReplacementChosenAnswer(ReplacementChosenAnswer { index: 0 }),
            PromptPayload::OrderTriggersPromptMessage(p) => AnswerPayload::TriggersOrderedAnswer(TriggersOrderedAnswer {
                order: (0..p.sources.len() as i32).collect(),
            }),
            PromptPayload::NameCardPromptMessage(p) => AnswerPayload::CardNamedAnswer(CardNamedAnswer { name: p.suggestion.clone() }),
            PromptPayload::ChoosePlayerPromptMessage(p) => {
                AnswerPayload::PlayerChosenAnswer(PlayerChosenAnswer { player: p.candidates.first().copied().unwrap_or(0) })
            }
            PromptPayload::ChooseColorsPromptMessage(p) => AnswerPayload::ColorsChosenAnswer(ColorsChosenAnswer {
                colors: if p.may_repeat {
                    vec![p.choices.first().copied().unwrap_or(ColorView::White); p.count.max(0) as usize]
                } else {
                    p.choices.iter().take(p.count.max(0) as usize).copied().collect()
                },
            }),
            PromptPayload::ChooseDividedTargetsPromptMessage(p) => AnswerPayload::TargetsDividedAnswer(TargetsDividedAnswer {
                shares: match p.candidates.first() {
                    Some(first) if p.total > 0 => vec![TargetShareMessage { target: first.clone(), amount: p.total }],
                    _ => Vec::new(),
                },
            }),
            PromptPayload::RemoveCountersPromptMessage(p) => AnswerPayload::CountersRemovedAnswer(CountersRemovedAnswer { removals: p.default_removals() }),
            PromptPayload::ChooseStartingPlayerPromptMessage(p) => AnswerPayload::PlayerChosenAnswer(PlayerChosenAnswer { player: p.player }),
            PromptPayload::MulliganPromptMessage(_) => AnswerPayload::MayChosenAnswer(MayChosenAnswer { accept: false }),
        }
    }
}

/// Requirements met in order, then everything left over into the first destination that will take
/// it: the placement the engine would fall back to (`CardDistribution.Greedy` in C#).
fn greedy_distribution(prompt: &DistributeCardsPrompt) -> Vec<Vec<ViewHandle>> {
    let mut placements: Vec<Vec<ViewHandle>> = vec![Vec::new(); prompt.destinations.len()];
    let mut left: Vec<ViewHandle> = prompt.cards.clone();
    for (i, destination) in prompt.destinations.iter().enumerate() {
        while (placements[i].len() as i32) < destination.min {
            let Some(pos) = left.iter().position(|c| destination.eligible.is_empty() || destination.eligible.contains(c)) else { break };
            placements[i].push(left.remove(pos));
        }
    }
    for card in left {
        if let Some(i) = prompt.destinations.iter().enumerate().position(|(i, d)| {
            (placements[i].len() as i32) < d.max && (d.eligible.is_empty() || d.eligible.contains(&card))
        }) {
            placements[i].push(card);
        } else if let Some(first) = placements.first_mut() {
            first.push(card);
        }
    }
    placements
}

// --- Events (Messages.cs, GameEventView) ---

/// Something that happened, as a viewer may be told it: one of 32 kinds, each the C# class of the
/// same name.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum GameEventView {
    ObjectMovedEvent(ObjectMovedEvent),
    LifeChangedEvent(LifeChangedEvent),
    DamageMarkedEvent(DamageMarkedEvent),
    TapChangedEvent(TapChangedEvent),
    StepBeganEvent(StepBeganEvent),
    SpellCastEvent(SpellCastEvent),
    SpellFizzledEvent(SpellFizzledEvent),
    LibraryShuffledEvent(LibraryShuffledEvent),
    CardsRevealedEvent(CardsRevealedEvent),
    EventsRetractedEvent(EventsRetractedEvent),
    LibraryArrangedEvent(LibraryArrangedEvent),
    AbilityActivatedEvent(AbilityActivatedEvent),
    TokenCreatedEvent(TokenCreatedEvent),
    CountersPutEvent(CountersPutEvent),
    ExtraTurnAddedEvent(ExtraTurnAddedEvent),
    PlayerLostEvent(PlayerLostEvent),
    GameEndedEvent(GameEndedEvent),
    TransformedEvent(TransformedEvent),
    DieRolledEvent(DieRolledEvent),
    ExtraPhaseAddedEvent(ExtraPhaseAddedEvent),
    DayNightBecameEvent(DayNightBecameEvent),
    MonarchBecameEvent(MonarchBecameEvent),
    RandomPlayerChosenEvent(RandomPlayerChosenEvent),
    StartingPlayerChosenEvent(StartingPlayerChosenEvent),
    MulliganTakenEvent(MulliganTakenEvent),
    HandKeptEvent(HandKeptEvent),
    AbilityResolvedEvent(AbilityResolvedEvent),
    CoinFlippedEvent(CoinFlippedEvent),
    TurnEndedEvent(TurnEndedEvent),
    CardsLookedAtEvent(CardsLookedAtEvent),
    ScriedEvent(ScriedEvent),
    SurveilledEvent(SurveilledEvent),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectMovedEvent {
    /// None when the viewer is not entitled to know which card moved.
    pub object: ViewHandle,
    pub from: ZoneKindView,
    pub from_owner: i32,
    pub to: ZoneKindView,
    pub to_owner: i32,
    pub reason: ZoneChangeReasonView,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LifeChangedEvent {
    pub player: i32,
    pub delta: i32,
    pub new_total: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DamageMarkedEvent {
    pub object: ViewHandle,
    pub amount: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TapChangedEvent {
    pub object: ViewHandle,
    pub tapped: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StepBeganEvent {
    pub turn: i32,
    pub step: TurnStepView,
    pub active_player: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SpellCastEvent {
    pub spell: ViewHandle,
    pub controller: i32,
    pub targets: Vec<TargetMessage>,
    pub x: Option<i32>,
    pub modes: Vec<String>,
    pub kicked: i32,
    pub buyback: bool,
    pub gift_promised_to: Option<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SpellFizzledEvent {
    pub spell: ViewHandle,
    pub controller: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LibraryShuffledEvent {
    pub player: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CardsRevealedEvent {
    pub player: i32,
    pub cards: Vec<ViewHandle>,
}

/// A player looked at hidden cards (a library's top card, a card of a hand): the cards are known to that player alone.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CardsLookedAtEvent {
    pub player: i32,
    pub cards: Vec<ViewHandle>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EventsRetractedEvent {
    pub sequences: Vec<i64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LibraryArrangedEvent {
    pub player: i32,
    pub on_top: i32,
    pub on_bottom: i32,
    pub shuffled: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AbilityActivatedEvent {
    pub ability: ViewHandle,
    pub source: ViewHandle,
    pub controller: i32,
    pub text: String,
    pub targets: Vec<TargetMessage>,
    pub triggered: bool,
    pub modes: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AbilityResolvedEvent {
    pub ability: ViewHandle,
    pub source: ViewHandle,
    pub controller: i32,
    pub first: i64,
    pub targets: Vec<TargetMessage>,
    pub power_toughness: Vec<PowerToughnessChangeView>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowerToughnessChangeView {
    pub object: ViewHandle,
    pub power: i32,
    pub toughness: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TokenCreatedEvent {
    pub object: ViewHandle,
    pub controller: i32,
    pub name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CountersPutEvent {
    pub object: ViewHandle,
    pub name: String,
    pub count: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExtraTurnAddedEvent {
    pub player: i32,
    pub count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerLostEvent {
    pub player: i32,
    pub reason: LossReasonView,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GameEndedEvent {
    pub outcome: GameOutcomeView,
    pub winning_team: i32,
    pub draw_reason: DrawReasonView,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TransformedEvent {
    pub object: ViewHandle,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DieRolledEvent {
    pub player: i32,
    pub sides: i32,
    pub result: i32,
}

/// An effect ended the turn of `player` (CR 724.1): the stack is exiled and the turn goes straight to its cleanup step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TurnEndedEvent {
    pub player: i32,
}

/// `player` scried `count` (CR 701.22): told to every seat, the cards and where they went to nobody but the scrier.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ScriedEvent {
    pub player: i32,
    pub count: i32,
}

/// `player` surveilled `count` (CR 701.25): the cards put into the graveyard arrive as moves.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SurveilledEvent {
    pub player: i32,
    pub count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExtraPhaseAddedEvent {
    pub phase: PhaseView,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DayNightBecameEvent {
    pub designation: DayNightView,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MonarchBecameEvent {
    pub player: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RandomPlayerChosenEvent {
    pub controller: i32,
    pub selected_player: i32,
    pub source: ViewHandle,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StartingPlayerChosenEvent {
    pub chooser: i32,
    pub player: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MulliganTakenEvent {
    pub player: i32,
    pub count: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CoinFlippedEvent {
    pub player: i32,
    /// Whether the player won a flip they called (CR 705.2); false for a flip no one calls.
    pub won: bool,
    /// Whether the flip was called, to be won or lost; one that cares only whether it comes up heads is not (CR 705.2).
    pub called: bool,
    /// For a flip no one calls, whether it came up heads; false otherwise.
    pub heads: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HandKeptEvent {
    pub player: i32,
    pub cards: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_are_tagged_with_the_csharp_class_name() {
        let prompt = PromptPayload::MayPromptMessage(MayPrompt { player: 1, source: ViewHandle(7), verb: "SearchLibrary".into(), amount: 0, ..MayPrompt::default() });
        let json = serde_json::to_string(&prompt).unwrap();
        assert_eq!(json, r#"{"$type":"MayPromptMessage","Player":1,"Source":7,"Verb":"SearchLibrary","Amount":0}"#);
        let back: PromptPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.player(), 1);
    }

    #[test]
    fn answers_round_trip() {
        let answer = AnswerPayload::TakeActionAnswer(TakeActionAnswer::new(ActionKindView::PlayLand, ViewHandle(3)));
        let json = serde_json::to_string(&answer).unwrap();
        assert!(json.starts_with(r#"{"$type":"TakeActionAnswer","Kind":"PlayLand","Source":3"#), "{json}");
        let _: AnswerPayload = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn default_answers_never_name_what_was_not_offered() {
        let prompt = PromptPayload::ChooseTargetPromptMessage(ChooseTargetPrompt {
            player: 1,
            candidates: vec![TargetMessage::player(2)],
            optional: false,
            count: 1,
            ..Default::default()
        });
        match prompt.default_answer() {
            AnswerPayload::TargetsChosenAnswer(t) => assert_eq!(t.targets, vec![TargetMessage::player(2)]),
            other => panic!("{other:?}"),
        }
    }

    /// A payment that can be given up defaults to giving it up; the assisting player's (CR 702.132a), which can't, to closing
    /// their window or announcing nothing more, and to paying with the pool mana offered - never to an option not offered.
    #[test]
    fn a_payment_with_no_cancel_defaults_to_what_it_offers() {
        let pay = |options: Vec<ActionOptionMessage>| {
            let prompt = PromptPayload::PayCostsPromptMessage(PayCostsPrompt { player: 1, options, ..Default::default() });
            match prompt.default_answer() {
                AnswerPayload::TakeActionAnswer(a) => (a.kind, a.mana),
                other => panic!("{other:?}"),
            }
        };
        let cancel = ActionOptionMessage::new(ActionKindView::CancelCasting, ViewHandle::NONE);
        let confirm = ActionOptionMessage::new(ActionKindView::ConfirmCasting, ViewHandle::NONE);
        let mut green = ActionOptionMessage::new(ActionKindView::AssignMana, ViewHandle::NONE);
        green.produces = vec![ManaTypeView::Green];
        assert_eq!(pay(vec![confirm.clone(), cancel.clone()]), (ActionKindView::CancelCasting, None));
        assert_eq!(pay(vec![green.clone(), cancel]), (ActionKindView::CancelCasting, None));
        assert_eq!(pay(vec![confirm]), (ActionKindView::ConfirmCasting, None));
        assert_eq!(pay(vec![green]), (ActionKindView::AssignMana, Some(ManaTypeView::Green)));
    }

    /// Crew 1 with a -4/1 and a 2/1 (CR 107.1b: a negative power takes from the total): the default answer takes only the
    /// creature with positive power, whose total meets the minimum, as the C# `Default()` does.
    #[test]
    fn the_default_total_mana_value_answer_fits_together() {
        // Lively Dirge: up to two, total mana value 4 or less, a 3 and a 2 offered (and a 1 after them).
        let prompt = PromptPayload::ChooseCardsPromptMessage(ChooseCardsPrompt {
            player: 1,
            cards: vec![ViewHandle(3), ViewHandle(4), ViewHandle(5)],
            count: 2,
            count_kind: CountKindView::UpTo,
            verdict: SelectionVerdictView { complete: true, addable: vec![ViewHandle(3), ViewHandle(4), ViewHandle(5)], complete_alone: vec![] },
            mana_values: vec![3, 2, 1],
            maximum_total_mana_value: Some(4),
            ..Default::default()
        });
        let AnswerPayload::CardsChosenAnswer(answer) = prompt.default_answer() else { panic!("cards") };
        assert_eq!(answer.cards, vec![ViewHandle(3), ViewHandle(5)], "the 3 and the 1 fit in 4; the 2 does not follow the 3");
    }

    #[test]
    fn the_default_crew_answer_leaves_out_a_negative_power() {
        let prompt = PromptPayload::ChooseCardsPromptMessage(ChooseCardsPrompt {
            player: 1,
            cards: vec![ViewHandle(3), ViewHandle(4)],
            count: 2,
            count_kind: CountKindView::UpTo,
            minimum_power: 1,
            powers: vec![-4, 2],
            ..Default::default()
        });
        let AnswerPayload::CardsChosenAnswer(chosen) = prompt.default_answer() else { panic!("cards") };
        assert_eq!(chosen.cards, vec![ViewHandle(4)]);
        let total: i32 = chosen.cards.iter().map(|c| [-4, 2][if *c == ViewHandle(3) { 0 } else { 1 }]).sum();
        assert!(total >= 1);
    }
}

/// A card with companion in a deck's sideboard, and whether the deck fulfils its condition (CR 702.139a, 103.2b): the host's
/// answer to `Companions`, which the deck editor shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CompanionCheckView {
    pub card: String,
    pub fulfilled: bool,
}
