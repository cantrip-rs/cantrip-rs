namespace Cantrip.Protocol;

// The wire's own copy of the rules vocabulary.
//
// Mirrored rather than shared, because Protocol must not reference the engine: that arrow is what keeps
// the rules engine out of the client's dependency closure. The duplication is guarded by a test that
// compares member names and values against the engine's enums, so drift fails the build rather than
// producing a client that silently misreads a step.

/// <summary>Steps of a turn (CR 500-514).</summary>
public enum TurnStepView
{
    Untap = 0,
    Upkeep = 1,
    Draw = 2,
    PrecombatMain = 3,
    BeginCombat = 4,
    DeclareAttackers = 5,
    DeclareBlockers = 6,
    CombatDamage = 7,
    EndOfCombat = 8,
    PostcombatMain = 9,
    End = 10,
    Cleanup = 11,
}

/// <summary>Phases of a turn (CR 500.1).</summary>
public enum PhaseView
{
    Beginning = 0,
    PrecombatMain = 1,
    Combat = 2,
    PostcombatMain = 3,
    Ending = 4,
}

/// <summary>Zones (CR 400.1).</summary>
public enum ZoneKindView
{
    Library = 0,
    Hand = 1,
    Battlefield = 2,
    Graveyard = 3,
    Stack = 4,
    Exile = 5,
    Command = 6,

    /// <summary>
    /// A player's sideboard: cards of theirs outside the game. Nothing in a game reaches into it; a companion waits
    /// there until its owner takes it (CR 116.2g), and a revealed companion is shown to everyone.
    /// </summary>
    Sideboard = 7,
}

/// <summary>Card types (CR 205.2a).</summary>
public enum CardTypeView
{
    Artifact = 0,
    Battle = 1,
    Creature = 2,
    Enchantment = 3,
    Instant = 4,
    Kindred = 5,
    Land = 6,
    Planeswalker = 7,
    Sorcery = 8,
}

/// <summary>Supertypes (CR 205.4a).</summary>
public enum SupertypeView
{
    Basic = 0,
    Legendary = 1,
    Ongoing = 2,
    Snow = 3,
    World = 4,
}

/// <summary>The five colors (CR 105.1).</summary>
public enum ColorView
{
    White = 0,
    Blue = 1,
    Black = 2,
    Red = 3,
    Green = 4,
}

/// <summary>Mana types: the five colors plus colorless (CR 106.1).</summary>
public enum ManaTypeView
{
    White = 0,
    Blue = 1,
    Black = 2,
    Red = 3,
    Green = 4,
    Colorless = 5,
}

public enum PlayerStatusView
{
    Active = 0,
    Lost = 1,
    Conceded = 2,
}

/// <summary>
/// Day and night (CR 731.1), the designation the game itself has rather than any player. A game starts with
/// <see cref="Neither"/> and, once it has become day or night, has exactly one of the two from then on.
/// </summary>
public enum DayNightView
{
    /// <summary>The game has neither designation, which is how every game starts (CR 731.1).</summary>
    Neither = 0,

    /// <summary>It is day (CR 731.1).</summary>
    Day = 1,

    /// <summary>It is night (CR 731.1).</summary>
    Night = 2,
}

public enum GameOutcomeView
{
    InProgress = 0,
    TeamWon = 1,
    Draw = 2,
}

/// <summary>Why a game is a draw (CR 104.4).</summary>
public enum DrawReasonView
{
    /// <summary>A game that was won, or a draw with no reason given.</summary>
    None = 0,

    /// <summary>Every player left in the game lost at once (CR 104.4a).</summary>
    AllLost = 1,

    /// <summary>The game went round a loop in which every player only passed (CR 104.4b; docs/rules-exceptions.md, RX-4).</summary>
    MandatoryLoop = 2,

    /// <summary>The game reached the most events the engine records (<c>docs/rules-exceptions.md</c>, RX-1).</summary>
    EventLimit = 3,
}

/// <summary>Why a player is no longer in the game (CR 104.3).</summary>
public enum LossReasonView
{
    /// <summary>Life at zero or less.</summary>
    Life = 0,

    /// <summary>A draw attempted from an empty library.</summary>
    EmptyLibrary = 1,

    /// <summary>Ten or more poison counters.</summary>
    Poison = 2,

    Conceded = 3,

    /// <summary>An effect said so, including an effect that stated somebody else wins.</summary>
    Effect = 4,
}

public enum ObjectKindView
{
    Card = 0,
    Token = 1,
    SpellCopy = 2,
    AbilityOnStack = 3,
    Emblem = 4,

    /// <summary>
    /// A copy of a card made to be cast (CR 707.12): a preparation card's exiled copy (CR 722.3c), or a copy an effect
    /// lets its controller cast (paradigm, CR 702.192a).
    /// </summary>
    CardCopy = 5,
}

public enum ActionKindView
{
    PlayLand = 0,
    CastSpell = 1,
    ActivateAbility = 2,
    UndoManaAbility = 3,

    /// <summary>Stops casting the spell being paid for (CR 733.1). Only ever an answer to a pay-costs prompt.</summary>
    CancelCasting = 4,

    /// <summary>Pays an optional cost from the pool. Only ever an answer to an unless-pays prompt.</summary>
    PayCost = 5,

    /// <summary>Declines an optional cost. Only ever an answer to an unless-pays prompt.</summary>
    DeclineCost = 6,

    /// <summary>Puts one mana from the pool towards the spell being paid for. Only while paying for a spell with {X}.</summary>
    AssignMana = 7,

    /// <summary>Raises X by one while paying for a spell with {X}.</summary>
    IncreaseX = 8,

    /// <summary>Lowers X by one while paying for a spell with {X}.</summary>
    DecreaseX = 9,

    /// <summary>Casts the spell with X as it stands. Offered only once nothing is owed.</summary>
    ConfirmCasting = 10,

    /// <summary>Taps the named permanent to pay {1} of a waterbend cost. Only while paying for one.</summary>
    TapToPay = 11,

    /// <summary>Turns the named face-down permanent face up for its mana cost (CR 701.40b). A special action.</summary>
    TurnFaceUp = 12,

    /// <summary>Exiles the named graveyard card to pay {1} of a delve cost. Only while paying for one.</summary>
    ExileToPay = 13,

    /// <summary>Pays a Phyrexian mana symbol still owed with life instead of the mana it names (CR 107.4f).</summary>
    PayLifeForSymbol = 14,

    /// <summary>Takes your companion into your hand for {3} (CR 116.2g): a special action, once per game.</summary>
    TakeCompanion = 15,

    /// <summary>Exiles the named card from your hand for its plot cost (CR 702.170a). A special action, at sorcery timing.</summary>
    Plot = 16,

    /// <summary>Exiles the named card from your hand with time counters for its suspend cost (CR 702.62a). A special action.</summary>
    Suspend = 17,

    /// <summary>Exiles the named card from your hand face down for {2} (CR 702.143a). A special action, during your turn.</summary>
    Foretell = 18,

    /// <summary>Unlocks a locked door of the named Room by paying that door's mana cost (CR 709.5e). A special action, at sorcery timing.</summary>
    UnlockDoor = 19,
}

/// <summary>Why a client is choosing cards.</summary>
public enum CardChoicePurposeView
{
    Discard = 0,

    /// <summary>Put on top of the library in the order chosen, so the last card chosen ends up on top.</summary>
    PutOnTop = 1,

    /// <summary>The chosen legendary permanent is kept and the others with its name go to the graveyard (CR 704.5j).</summary>
    KeepLegend = 2,

    /// <summary>The chosen permanent is sacrificed to pay the source's activation cost, or its alternative cost (CR 701.21).</summary>
    Sacrifice = 3,

    /// <summary>The chosen cards are exiled by an effect as it resolves, from wherever the prompt's cards are: a hand, graveyards, the battlefield.</summary>
    Exile = 4,

    /// <summary>The chosen permanent is returned to its owner's hand to pay the alternative cost of the spell being cast.</summary>
    ReturnToHand = 5,

    /// <summary>The chosen permanents are untapped (CR 701.26), as Frantic Search untaps up to three lands.</summary>
    Untap = 6,

    /// <summary>The chosen card is put onto the battlefield from the chooser's own hand, as Sneak Attack does; choosing nothing declines.</summary>
    PutOntoBattlefield = 7,

    /// <summary>The chosen card is exiled from hand to pay the alternative cost of the spell being cast (CR 118.9).</summary>
    ExileToPay = 8,

    /// <summary>The chosen permanent, of the kind the prompt names, is kept and the rest of <c>Cards</c> sacrificed.</summary>
    Keep = 9,

    /// <summary>The chosen card is put into the chooser's hand from among cards an effect just moved, as Barrowgoyf's milled cards; choosing nothing declines.</summary>
    PutIntoHand = 10,
    /// <summary>The chosen card is cast as the effect resolves, the chooser paying its costs (Bilbo, from the graveyard); choosing nothing declines.</summary>
    Cast = 11,

    /// <summary>The chosen creatures are tapped to pay a cost, their power adding up to the prompt's <c>MinimumPower</c> (saddle, CR 702.171a).</summary>
    TapToPay = 12,

    OrderSimultaneous = 13,

    /// <summary>The chosen permanent is destroyed as the effect resolves (CR 701.7); chosen, not targeted, from the battlefield <c>Owner</c> names.</summary>
    Destroy = 14,

    /// <summary>The chosen object is the asked player's vote (CR 701.38a); the effect counts the tally once everyone has voted.</summary>
    Vote = 15,

    /// <summary>The chosen card is put into its owner's graveyard by an effect, from wherever it is (a store of cards exiled with a permanent).</summary>
    PutIntoGraveyard = 16,

    /// <summary>The chosen card is granted a lasting permission to be played this turn without paying its mana cost (CR 118.9).</summary>
    Play = 17,

    /// <summary>Reveal cards from hand and add their text to the spell being cast, in the chosen order (CR 702.47).</summary>
    Splice = 18,

    /// <summary>Put on the bottom of the chooser's library: the cards put back for the mulligans taken, once a hand is kept (CR 103.5).</summary>
    PutOnBottom = 19,

    /// <summary>Attach the chosen Auras or Equipment to a permanent as an effect resolves (CR 701.3a).</summary>
    Attach = 20,

    /// <summary>Exile the spell with cipher encoded on the chosen creature you control, or on none (CR 702.99a).</summary>
    Encode = 21,

    /// <summary>Choose a permanent you control, or a card in your hand to reveal, of the quality the prompt names, to pay a cost (CR 701.4a); nothing is lost.</summary>
    Behold = 22,

    /// <summary>Choose the creature you control that gets the -1/-1 counters a cost puts on a creature (CR 701.68a).</summary>
    Blight = 23,

    /// <summary>The chosen card is cast as the effect resolves without paying its mana cost; choosing nothing declines (Etali, Improvisation Capstone, ripple).</summary>
    CastFree = 24,

    /// <summary>A copy of the chosen card is cast as the effect resolves without paying its mana cost (CR 707.12); choosing nothing declines (Uldaros Theorix).</summary>
    CastCopyFree = 25,

    /// <summary>Up to one untapped creature you control is tapped as part of casting, and its power (the prompt's <c>Powers</c>) comes off the generic mana of the total cost; choosing nothing pays the full cost (harmonize, CR 702.180).</summary>
    TapToReduce = 26,

    /// <summary>The permanent that gets counters, chosen without targeting: the prompt's <c>Counter</c> and <c>CounterCount</c> say which kind and how many (empower Jace's loyalty counters on a Jace token, amass's +1/+1 counters on an Army).</summary>
    PutCounters = 27,

    /// <summary>Craft's materials (CR 702.167a): permanents you control and cards in your graveyard, each of the quality the prompt's <c>Kind</c> names, exiled with the crafting permanent to pay its cost; <c>Minimum</c> to <c>Count</c> of them, or exactly <c>Count</c>.</summary>
    Craft = 28,

    /// <summary>A card chosen to be revealed from among cards just looked at (CR 701.20a); what becomes of it is the effect's to say, so the prompt's wording is only "reveal" (Break Out).</summary>
    Reveal = 29,

    /// <summary>Whose damage a prevention shield prevents first, of two or more sources that would deal damage at the same time to the shielded permanent or player (CR 615.7): one of the sources. The prompt's <c>Source</c> made the shield and <c>PreventLeft</c> is how much it has left to prevent; the rest of it goes to the others' damage.</summary>
    PreventDamageFrom = 30,

    /// <summary>The chosen permanents are tapped by an effect as it resolves (CR 701.26a), not to pay a cost: Thousand Moons Smithy's "you may tap five untapped artifacts and/or creatures you control".</summary>
    Tap = 31,

    /// <summary>
    /// The other attacking creatures that attack in a band with the source, a creature with banding (CR 702.22c, 508.1e): any
    /// number with banding and at most one of <c>WithoutBanding</c>. None chosen forms no band. A band is blocked as one
    /// (CR 702.22h).
    /// </summary>
    Band = 32,

    /// <summary>Cards in the chooser's hand drawn this turn, for each of which they then pay the prompt's <c>Life</c> life or put it on top of their library (Sylvan Library): the cards the payments are about, not those that go back.</summary>
    PayLifeOrPutOnTop = 33,

    /// <summary>The chosen card is exiled by an effect and becomes plotted (CR 702.170a, 702.170d): its owner may cast it as a sorcery on a later turn without paying its mana cost (Jace Reawakened). Something gained, not given up; choosing nothing, where the prompt is "up to", declines.</summary>
    Plot = 34,

    /// <summary>
    /// The creature the source's enlist ability taps as the source attacks (CR 702.154a, 508.1g): up to one untapped creature
    /// you control that isn't attacking and has haste or has been yours since the turn began. Choosing none pays nothing;
    /// choosing one, the source gets +X/+0 until end of turn, X that creature's power.
    /// </summary>
    Enlist = 35,

    /// <summary>
    /// A permanent chosen as the source enters, which the source's text then refers to as "the chosen [<c>Kind</c>]" (CR 614.12a,
    /// 607.2d): Dauntless Bodyguard's "another creature you control". Chosen, not targeted; nothing happens to it now.
    /// </summary>
    ChooseAsEnters = 36,

    /// <summary>
    /// Objects chosen as an effect resolves, not targeted, that nothing happens to yet (CR 608.2d): Spry and Mighty's two
    /// creatures.
    /// </summary>
    Choose = 37,
}

/// <summary>Where cards found by a library search go (CR 701.23).</summary>
public enum SearchDestinationView
{
    Hand = 0,
    TopOfLibrary = 1,
    Graveyard = 2,
    Battlefield = 3,
    Exile = 4,
    ExileFaceDown = 5,
    Cast = 6,

    /// Onto the battlefield tapped (Nissa, Winds of Abandon).
    BattlefieldTapped = 7,

    /// Cast without paying its mana cost (Sunforger).
    CastFree = 8,

    /// What becomes of the card depends on which is found (Transmute Artifact): the search says only what it looks for.
    Found = 9,

    /// A land card onto the battlefield tapped, any other card into the searcher's hand (Archdruid's Charm): the type of the
    /// card found decides.
    LandTappedElseHand = 10,
}

/// <summary>One of the places a card may be sent by a distribution: scry's bottom, surveil's graveyard, Stock Up's hand.</summary>
public enum CardDestinationKindView
{
    LibraryTop = 0,
    LibraryBottom = 1,
    Graveyard = 2,
    Hand = 3,
    Exile = 4,
    BattlefieldFaceDown = 5,
}

/// <summary>What some mana may be spent on (CR 106.6): the filter half of a restriction.</summary>
public enum ManaSpendFilterView
{
    None = 0,
    CreatureSpells = 1,
    InstantOrSorcerySpells = 2,
    ActivatedAbilities = 3,
    ArtifactSpells = 4,
    LegendarySpells = 5,
    NotNonartifactSpells = 6,
    PlaneswalkerSpells = 7,
    /// <summary>Only to activate abilities of land sources: a land, or a land card in a hand or graveyard.</summary>
    LandAbilities = 8,
    /// <summary>Only to activate abilities of artifact sources.</summary>
    ArtifactAbilities = 9,
    /// <summary>Only to cast a spell with one of the subtypes named beside it: "Dragon spells", "a Dragon spell or an Omen spell".</summary>
    SubtypeSpells = 10,
    /// <summary>Only to cast a creature spell with one of the subtypes named beside it: "a creature spell of the chosen type".</summary>
    SubtypeCreatureSpells = 11,
    /// <summary>Only to cast a spell that is not a creature spell: "a noncreature spell".</summary>
    NoncreatureSpells = 12,
    /// <summary>Only to cast a spell with the mana value named beside it or greater: "spells with mana value 4 or greater".</summary>
    ManaValueSpells = 13,
    /// <summary>Only to cast a spell with the mana value named beside it or greater, or with {X} in its mana cost.</summary>
    ManaValueOrXSpells = 14,
    /// <summary>Only to cast a spell from exile: "spend this mana only to cast spells from exile".</summary>
    SpellsFromExile = 15,
    /// <summary>On anything but a spell cast from the hand: "this mana can't be spent to cast spells from your hand".</summary>
    NotSpellsFromHand = 16,
    /// <summary>Only to activate abilities of creature sources.</summary>
    CreatureAbilities = 17,
    /// <summary>Only to cast a spell with one of the subtypes named beside it or activate an ability of a source with one:
    /// "Elemental spells or abilities of Elemental sources".</summary>
    SubtypeSpellsOrAbilities = 18,
    /// <summary>Only to cast a creature spell with one of the subtypes named beside it or activate an ability of a creature
    /// source with one: "a creature spell of the chosen type or an ability of a creature source of the chosen type".</summary>
    SubtypeCreatureSpellsOrAbilities = 19,
    /// <summary>Only to activate abilities of artifacts: artifact permanents, never an artifact card in a hand or graveyard
    /// (CR 109.2).</summary>
    AbilitiesOfArtifacts = 20,
    /// <summary>Only to cast creature spells or activate abilities of creatures: creature permanents (CR 109.2).</summary>
    CreatureSpellsOrAbilitiesOfCreatures = 21,
    /// <summary>Only to cast creature spells or activate abilities of creature sources, in any zone (CR 109.2c).</summary>
    CreatureSpellsOrAbilities = 22,
    /// <summary>Only to cast artifact spells or activate abilities of artifacts: artifact permanents (CR 109.2).</summary>
    ArtifactSpellsOrAbilitiesOfArtifacts = 23,
    /// <summary>Only to cast a spell with one of the subtypes named beside it or activate an ability of a permanent with one
    /// (CR 109.2): "Elemental spells or abilities of Elementals".</summary>
    SubtypeSpellsOrAbilitiesOfPermanents = 24,
    /// <summary>Only to cast artifact spells or activate abilities of any source: "an artifact spell or ... an ability".</summary>
    ArtifactSpellsOrAnyAbility = 25,
}

/// <summary>What spending some mana does to whatever it pays for (CR 106.6): the effect half of a restriction.</summary>
public enum ManaSpendEffectView
{
    None = 0,
    GainsHaste = 1,
    CantBeCountered = 2,
}

public enum AttackTargetKindView
{
    Player = 0,
    Planeswalker = 1,
    Battle = 2,
}
