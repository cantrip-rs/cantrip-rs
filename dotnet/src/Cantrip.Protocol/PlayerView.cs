using MessagePack;

namespace Cantrip.Protocol;

/// <summary>
/// Everything one player is entitled to see, and nothing else.
/// <para>
/// Produced by a single pure function from game state. That is the whole hidden-information design: one
/// auditable place where information crosses from the server's knowledge to a player's, rather than
/// leak-avoidance spread across every message the server sends. Deltas, when they arrive, are produced
/// by diffing successive projections <em>per viewer</em>, never by translating internal mutations —
/// internal mutations talk about hidden state, so a delta derived from them leaks by construction.
/// </para>
/// <para>
/// Every field carries an explicit numeric <see cref="KeyAttribute"/>. Never implicit ordering and never
/// string keys: a player-hosted server and a client built from a different commit must either interop or
/// fail loudly at the handshake, and silent field misalignment is a bug nobody can debug from a
/// screenshot.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class PlayerView
{
    /// <summary>Protocol version, checked at the handshake so mismatched builds fail loudly.</summary>
    public const int ProtocolVersion = 221;

    [Key(0)]
    public int Version { get; set; } = ProtocolVersion;

    /// <summary>Which seat is looking. Seats are numbered from one.</summary>
    [Key(1)]
    public int Viewer { get; set; }

    [Key(2)]
    public int Turn { get; set; }

    [Key(3)]
    public TurnStepView Step { get; set; }

    [Key(4)]
    public int ActivePlayer { get; set; }

    /// <summary>Who holds priority, or zero while nobody does.</summary>
    [Key(5)]
    public int PriorityPlayer { get; set; }

    [Key(6)]
    public List<PlayerSummaryView> Players { get; set; } = [];

    /// <summary>
    /// Objects this viewer can observe, in handle order. Cards in a library, or in another player's hand,
    /// are not here at all: those zones report a count and nothing more.
    /// </summary>
    [Key(7)]
    public List<ObjectView> Objects { get; set; } = [];

    [Key(8)]
    public List<ZoneView> Zones { get; set; } = [];

    [Key(9)]
    public CombatView Combat { get; set; } = new();

    [Key(10)]
    public GameOutcomeView Outcome { get; set; }

    [Key(11)]
    public int WinningTeam { get; set; }

    /// <summary>
    /// Whether it is day or night (CR 731.1). Public, as the monarch and the city's blessing are, and on the view
    /// itself rather than on a player: the designation belongs to the game and there is nobody who has it.
    /// </summary>
    [Key(12)]
    public DayNightView DayNight { get; set; }

    public override string ToString() =>
        $"view for seat {Viewer}: turn {Turn} {Step}, {Objects.Count} object(s)";
}

/// <summary>A player as seen by someone. Nothing here is hidden from anyone.</summary>
[MessagePackObject]
public sealed class PlayerSummaryView
{
    [Key(0)]
    public int Seat { get; set; }

    [Key(1)]
    public string Name { get; set; } = string.Empty;

    [Key(2)]
    public int Team { get; set; }

    [Key(3)]
    public int Life { get; set; }

    [Key(4)]
    public PlayerStatusView Status { get; set; }

    [Key(5)]
    public int PoisonCounters { get; set; }

    /// <summary>
    /// How many cards are in hand. A count is public information; which cards they are is not, so for
    /// anyone but the owner this is all that is sent.
    /// </summary>
    [Key(6)]
    public int HandCount { get; set; }

    [Key(7)]
    public int LibraryCount { get; set; }

    [Key(8)]
    public int LandsPlayedThisTurn { get; set; }

    /// <summary>
    /// Unspent mana that may be spent on anything, indexed by <see cref="ManaTypeView"/>. Public: a mana pool is
    /// open information.
    /// </summary>
    [Key(9)]
    public int[] Mana { get; set; } = new int[6];

    /// <summary>Unspent mana that may only be spent on certain things, such as creature spells (CR 106.6).</summary>
    [Key(10)]
    public List<RestrictedManaView> RestrictedMana { get; set; } = [];

    /// <summary>Whether this player is the monarch (CR 725). Public: a designation everyone can see.</summary>
    [Key(11)]
    public bool IsMonarch { get; set; }

    /// <summary>Whether this player has the initiative (CR 726). Public, as the monarch is.</summary>
    [Key(12)]
    public bool HasInitiative { get; set; }

    /// <summary>The dungeon this player is in (CR 309), or empty. A dungeon in the command zone is public.</summary>
    [Key(13)]
    public string Dungeon { get; set; } = string.Empty;

    /// <summary>The name of the room their venture marker is on, or empty with no dungeon.</summary>
    [Key(14)]
    public string DungeonRoom { get; set; } = string.Empty;

    /// <summary>How many dungeons this player has completed (CR 309.7).</summary>
    [Key(15)]
    public int DungeonsCompleted { get; set; }

    /// <summary>Energy counters this player has (CR 107.16, 122.1). A player's energy is public information.</summary>
    [Key(16)]
    public int Energy { get; set; }

    /// <summary>
    /// Whether this player has the city's blessing (CR 702.131c). Public, as the monarch and the initiative are, and
    /// unlike either of them any number of players may have it at once.
    /// </summary>
    [Key(17)]
    public bool HasCitysBlessing { get; set; }

    /// <summary>
    /// Counters on this player, by name (CR 122.1) — experience counters, rad counters. Public: a counter on a
    /// player is as open as one on a permanent. Poison and energy are sent separately above, because the rules give
    /// each of those a meaning of its own and the board already draws each under its own name.
    /// </summary>
    [Key(18)]
    public List<CounterView> Counters { get; set; } = [];

    /// <summary>
    /// Effects waiting on this player that sit on no object, one line each: "The next spell you cast this turn can't be
    /// countered" (CR 611.2c). Public, as the effects on the table are.
    /// </summary>
    [Key(19)]
    public List<string> PendingEffects { get; set; } = [];

    /// <summary>
    /// Whether this player has an enduring story (CR 702.195b), which storied gives. Public, as the city's blessing is,
    /// and like it had by any number of players at once and kept for the rest of the game.
    /// </summary>
    [Key(20)]
    public bool HasEnduringStory { get; set; }

    /// <summary>
    /// This player's speed (CR 702.179), 1 to 4, which start your engines! gives and which rises once on each of their
    /// turns an opponent loses life; null while they have no speed (CR 702.179b). Public. At 4 they have max speed, which
    /// max speed abilities ask for (CR 702.178a).
    /// </summary>
    [Key(21)]
    public int? Speed { get; set; }

    /// <summary>
    /// The seat making this player's choices and decisions now, while another player controls them (CR 723: Mindslaver,
    /// Emrakul, the Promised End) - every question asked of this player goes to that seat (<see cref="PromptMessage.ForSeat"/>),
    /// though this player is still the active player and controls their objects (CR 723.3). Zero while they make their own.
    /// Public, as the effect is.
    /// </summary>
    [Key(22)]
    public int ControlledBy { get; set; }

    /// <summary>
    /// The companion this player revealed before the game (CR 103.2b, 702.139a), while it waits outside the game, revealed to
    /// everyone: an object of the view, in its owner's sideboard. Once during the game, at sorcery timing, its owner may pay
    /// {3} to put it into their hand (CR 116.2g, <see cref="ActionKindView.TakeCompanion"/>). None once it is in the game, or
    /// with none revealed.
    /// </summary>
    [Key(23)]
    public ViewHandle Companion { get; set; }

    public override string ToString() => $"seat {Seat} '{Name}' {Life} life";
}

/// <summary>Some mana in a pool and what it may be spent on.</summary>
[MessagePackObject]
public sealed class RestrictedManaView
{
    [Key(0)]
    public ManaTypeView Type { get; set; }

    [Key(2)]
    public int Amount { get; set; }

    [Key(3)]
    public ManaSpendFilterView Filter { get; set; }

    [Key(4)]
    public ManaSpendEffectView Effect { get; set; }

    /// <summary>
    /// For <see cref="ManaSpendFilterView.SubtypeSpells"/>, <see cref="ManaSpendFilterView.SubtypeCreatureSpells"/>,
    /// <see cref="ManaSpendFilterView.SubtypeSpellsOrAbilities"/> and <see cref="ManaSpendFilterView.SubtypeCreatureSpellsOrAbilities"/>:
    /// the subtypes, any one of which the spell (or the ability's source) must have ("Dragon", "Omen"). Empty for a chosen
    /// type never chosen, which nothing has.
    /// </summary>
    [Key(5)]
    public List<string> SpendSubtypes { get; set; } = [];

    /// <summary>
    /// For <see cref="ManaSpendFilterView.ManaValueSpells"/> and <see cref="ManaSpendFilterView.ManaValueOrXSpells"/>: the
    /// least mana value the spell may have ("mana value 4 or greater"). 0 otherwise.
    /// </summary>
    [Key(6)]
    public int SpendManaValue { get; set; }

    /// <summary>
    /// The permanent that produced it, for ordinary mana a rule tells apart by its source (CR 106.3: "mana produced by
    /// Barracks of the Thousand"): a kind of mana of its own, which an <see cref="ActionKindView.AssignMana"/> option names
    /// as its <c>Source</c>. None for restricted mana and for mana no rule tells apart.
    /// </summary>
    [Key(7)]
    public ViewHandle Source { get; set; }
}

/// <summary>
/// One object the viewer can see.
/// <para>
/// Characteristics here are computed, not printed: what comes out of the copy-effect prestage and layers
/// 2 through 7. A client showing printed power would disagree with the rules the moment an anthem exists.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class ObjectView
{
    [Key(0)]
    public ViewHandle Handle { get; set; }

    [Key(1)]
    public ZoneKindView Zone { get; set; }

    /// <summary>Seat owning the zone, or zero for a shared zone.</summary>
    [Key(2)]
    public int ZoneOwner { get; set; }

    [Key(3)]
    public ObjectKindView Kind { get; set; }

    [Key(4)]
    public int Owner { get; set; }

    [Key(5)]
    public int Controller { get; set; }

    /// <summary>
    /// False when the viewer can see that something is there but not what it is, as for a face-down
    /// permanent. The remaining fields then carry only what the viewer is entitled to.
    /// </summary>
    [Key(6)]
    public bool Known { get; set; } = true;

    /// <summary>Content id, for art and oracle text lookup. Empty when the object is not known to the viewer.</summary>
    [Key(7)]
    public string CardId { get; set; } = string.Empty;

    [Key(8)]
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Mana cost in oracle notation, for example <c>{1}{R}</c>. <c>{0}</c> for a cost of zero, and empty for an object
    /// with no mana cost at all (CR 202.1b), which a client shows differently: a Mox costs {0}, a land costs nothing.
    /// <para>
    /// A string because the client only ever displays it. The structured cost is what the engine solves against
    /// when paying (hybrid, filters, CR 601.2e-f reductions), and none of that is a client's business: a client
    /// that reasoned about cost would be deciding affordability, which is the server's answer to give.
    /// </para>
    /// </summary>
    [Key(20)]
    public string ManaCost { get; set; } = string.Empty;

    /// <summary>
    /// Mana value (CR 202.3). Sent rather than left to be counted off <see cref="ManaCost"/>, because it is a
    /// number the rules compute and a client that derived it would be parsing a cost to answer a rules question.
    /// </summary>
    [Key(21)]
    public int ManaValue { get; set; }

    /// <summary>Rules text as it currently reads (CR 109.3). Empty for a card with none.</summary>
    [Key(22)]
    public string OracleText { get; set; } = string.Empty;

    [Key(9)]
    public List<SupertypeView> Supertypes { get; set; } = [];

    [Key(10)]
    public List<CardTypeView> CardTypes { get; set; } = [];

    [Key(11)]
    public List<string> Subtypes { get; set; } = [];

    [Key(12)]
    public List<ColorView> Colors { get; set; } = [];

    [Key(13)]
    public int? Power { get; set; }

    [Key(14)]
    public int? Toughness { get; set; }

    [Key(15)]
    public bool Tapped { get; set; }

    [Key(16)]
    public bool FaceDown { get; set; }

    /// <summary>
    /// Damage marked this turn (CR 120.3), sent separately from toughness rather than subtracted from it,
    /// because the rules compare the two and a client that subtracted would show the wrong number under
    /// any toughness-changing effect.
    /// </summary>
    [Key(17)]
    public int DamageMarked { get; set; }

    [Key(18)]
    public List<CounterView> Counters { get; set; } = [];

    /// <summary>
    /// Whether this creature has been under its controller's control since their most recent turn began
    /// (CR 302.6). Computed server-side: the client has no business deriving it, and needs it to grey out
    /// a creature that cannot attack.
    /// </summary>
    [Key(19)]
    public bool SummoningSick { get; set; }

    /// <summary>
    /// The other seats that know what this card is although it is in a zone they cannot look into: the cards left in a
    /// hand after Duress showed it, or a card put back on top of a library in front of everyone. Only ever filled in for
    /// the viewer's own cards, so a player can see what their opponents know about them.
    /// </summary>
    [Key(23)]
    public List<int> KnownBy { get; set; } = [];

    /// <summary>
    /// What this spell or ability on the stack is aimed at, in the order its card names the targets (CR 601.2c). Empty
    /// for everything else, and for anything on the stack that targets nothing.
    /// </summary>
    [Key(24)]
    public List<SpellTargetView> Targets { get; set; } = [];

    /// <summary>
    /// For a card in exile "until" a permanent leaves the battlefield (CR 610.3), as Portable Hole's victim: that
    /// permanent, while it is still there and so still holding the card. None for everything else, so a client can show
    /// the card tucked under what is keeping it.
    /// <para>
    /// A preparation card's copy (CR 722.3c, <see cref="ObjectKindView.CardCopy"/>) names its permanent here too.
    /// It is the same fact — this card is in exile because that permanent is on the battlefield keeping it there —
    /// and the copy is castable, so it has to be drawn somewhere the player can reach it.
    /// </para>
    /// </summary>
    [Key(25)]
    public ViewHandle ExiledWith { get; set; }

    /// <summary>
    /// Whether this object has a mana ability (CR 605.1a), printed or granted. Sent so a client can lay out mana rocks
    /// beside the lands they work with, without reading rules text to find out which artifacts make mana.
    /// </summary>
    [Key(26)]
    public bool ProducesMana { get; set; }

    /// <summary>
    /// For anything face down, the name of the card it really is, sent only to a viewer allowed to look at it: a
    /// permanent's controller (CR 708.5), or, for a card exiled face down, whoever CR 406.3 lets examine it — whoever
    /// could see where it came from, and whoever has been shown it since. Everything else about the object is what it
    /// is face down, a nameless 2/2 on the battlefield (CR 708.2a) and no characteristics at all in exile (CR 406.3a),
    /// and that goes to every seat. Empty for anything face up.
    /// </summary>
    [Key(27)]
    public string FaceDownName { get; set; } = string.Empty;

    /// <summary>Whether this permanent is monstrous (CR 701.37b): a designation it keeps until it leaves the battlefield.</summary>
    [Key(28)]
    public bool Monstrous { get; set; }

    /// <summary>
    /// For an Aura on the battlefield, what it is attached to (CR 303.4): a permanent, or a card in a graveyard while
    /// Animate Dead's trigger waits. None for everything else, and for an object the viewer cannot see.
    /// </summary>
    [Key(29)]
    public ViewHandle AttachedTo { get; set; }

    /// <summary>
    /// The qualities chosen for this permanent and kept on it (CR 614.12a) — Anointed Peacekeeper's card name,
    /// Painter's Servant's color, Gollum's odd or even — in key order. Public: such a quality is chosen in front of
    /// everyone. Empty for anything that chose none.
    /// </summary>
    [Key(30)]
    public List<ChoiceView> Choices { get; set; } = [];

    /// <summary>
    /// Whether this permanent is exerted (CR 701.43a): it won't untap during the next untap step of the player who
    /// exerted it.
    /// </summary>
    [Key(31)]
    public bool Exerted { get; set; }

    /// <summary>Whether this permanent is saddled (CR 702.171b): a designation it keeps until end of turn or until it leaves.</summary>
    [Key(32)]
    public bool Saddled { get; set; }

    /// <summary>
    /// Whether this creature is goaded (CR 701.15): until the goading player's next turn it attacks each combat if able,
    /// and attacks a player other than them if able.
    /// </summary>
    [Key(33)]
    public bool Goaded { get; set; }

    /// <summary>The attacking band it is in (CR 702.22c), numbered from 1 in the order announced; 0 for none.</summary>
    [Key(58)]
    public int Band { get; set; }

    /// <summary>
    /// For a double-faced card, the face that is not up, exactly as it is printed (CR 712.2); null for a card with one
    /// face and for anything the viewer may not identify.
    /// <para>
    /// Both faces of a double-faced card are printed on it and both are public, so a player may read the one facing
    /// away whenever they may read the card at all. The client holds a card up to be read; with only the face that is
    /// up, half of a transforming card is something a player has to already know.
    /// </para>
    /// <para>
    /// Printed and nothing more: no handle, no zone, no computed power, no counters, because this face is not an
    /// object and none of that means anything about it. An <see cref="ObjectView"/> rather than a type of its own so a
    /// client draws it with the code it draws every other card face with, as <see cref="KnownCardView.Card"/> is.
    /// </para>
    /// </summary>
    [Key(34)]
    public ObjectView? OtherFace { get; set; }

    /// <summary>Whether this permanent has the harnessed designation (CR 701.64).</summary>
    [Key(35)]
    public bool Harnessed { get; set; }

    /// <summary>The seat this Aura enchants (CR 303.4b), or zero when it enchants an object or nothing.</summary>
    [Key(36)]
    public int AttachedToPlayer { get; set; }

    /// <summary>
    /// Abilities an effect has given this object (CR 613.1f) that its <see cref="OracleText"/> does not already say, in
    /// the order it has them: Nadu's trigger on each of its controller's creatures, the flying an anthem hands out.
    /// Rules text rather than names, so a client shows them beneath the printed text with nothing to look up.
    /// </summary>
    [Key(37)]
    public List<AbilityTextView> GrantedAbilities { get; set; } = [];

    /// <summary>
    /// The lines of <see cref="OracleText"/>, counted from zero between newlines, whose ability has done all it may this
    /// turn or this game — triggered as often as it says it may (CR 603.2h), or a loyalty, boast or exhaust ability
    /// already used (CR 606.3, 702.142a, 702.177a), or a Case's "To solve" once it is solved and its "Solved" ability while it
    /// is not (CR 719.3a, 719.3c) — so a client can grey them out. A granted ability says so on its own
    /// <see cref="AbilityTextView.Spent"/>.
    /// </summary>
    [Key(38)]
    public List<int> SpentLines { get; set; } = [];

    /// <summary>
    /// A digest of everything else here, as the server sent it, or zero when nothing was worked out. The same handle with
    /// the same fingerprint as the last view is the same object, unchanged, so a client can go on using what it made of
    /// it last time rather than working it all out again for every object on every view (<see cref="ObjectFingerprint"/>).
    /// </summary>
    [Key(39)]
    public long Fingerprint { get; set; }

    /// <summary>
    /// Whether this object is a copy of something it is not, which nothing else here says: a card a copy effect has made
    /// another (CR 707.2, Phyrexian Metamorph as a Solitude), or a token that is a copy of a card or permanent - made as
    /// one, or a copied permanent spell resolved (CR 707.10f) - which reads exactly as that card does.
    /// False for a token with characteristics of its own, and for anything face down (CR 708.10).
    /// </summary>
    [Key(40)]
    public bool IsCopy { get; set; }

    /// <summary>
    /// For a card a copy effect has made another, the card itself as printed (as <see cref="DeckCardView.Card"/>), for a
    /// close look to show beside what it is now; null for everything else. A card's own identity is public whenever
    /// it is face up, and the copy effect does not hide it.
    /// </summary>
    [Key(41)]
    public ObjectView? CopyingCard { get; set; }

    /// <summary>
    /// For a preparation card, its prepare spell exactly as it is printed (CR 722.2a), as Emeritus of Abundance's
    /// Regrowth; null for everything else, the exiled copy that is the prepare spell among them.
    /// <para>
    /// The prepare spell is printed on the card beside the creature, so a player may read it whenever they may read the
    /// card at all, prepared or not: the client holds it up beside the card as it does a double-faced card's
    /// <see cref="OtherFace"/>, and asks for its art with the card's. Printed and nothing more, as that face is.
    /// </para>
    /// </summary>
    [Key(42)]
    public ObjectView? PrepareSpell { get; set; }

    /// <summary>
    /// For a modal spell or ability on the stack, the modes it was put there with, as their text, in the order they
    /// happen (CR 700.2d). Public from the moment they are chosen (CR 601.2b, 603.3c), including a mode with no target
    /// to show it by. Empty for everything else.
    /// </summary>
    [Key(43)]
    public List<string> ChosenModes { get; set; } = [];

    /// <summary>
    /// This permanent's class level (CR 716.2b), a designation and not a counter; 0 for a permanent that has none,
    /// which the rules treat as level 1 (CR 716.2d).
    /// </summary>
    [Key(44)]
    public int ClassLevel { get; set; }

    /// <summary>
    /// The seat protecting this battle (CR 310.9): its defending player, the one seat that may not attack it and the
    /// one whose creatures may block those that do. Zero for anything that is not a battle on the battlefield, and for
    /// a battle with none yet.
    /// </summary>
    [Key(45)]
    public int Protector { get; set; }

    /// <summary>
    /// For a split card anywhere but the stack, its halves exactly as they are printed (CR 709.1), in the order they are
    /// printed - Leave, then Chance; empty for everything else. The card's own name, cost and text above are the
    /// whole card's (CR 709.4); these are what a client lays out side by side to draw it the way it is printed. On the
    /// stack the card is the half that was cast, and has none. Printed and nothing more, as <see cref="OtherFace"/> is.
    /// </summary>
    [Key(46)]
    public List<ObjectView> Halves { get; set; } = [];

    /// <summary>A Room on the battlefield (CR 709.5): whether its left and right doors are unlocked, in that order; empty
    /// for anything else. A locked door is unlocked by an activation offered on the Room (CR 709.5e).</summary>
    [Key(48)]
    public List<bool> UnlockedDoors { get; set; } = [];

    /// <summary>
    /// For a planeswalker or a battle anywhere but the battlefield, the loyalty or defense it would enter with
    /// (CR 306.5a, 310.4a), which is what such a card shows: it has no counters yet (CR 306.5c, 310.4c). Null on the
    /// battlefield, where the counters are the number, and for every other card.
    /// </summary>
    [Key(47)]
    public int? StartingCounter { get; set; }

    /// <summary>
    /// Whether it is every creature type (Changeling, CR 702.73a). <see cref="Subtypes"/> then holds all of them, about
    /// three hundred, for anything that asks whether it is a Goblin; a type line prints <see cref="TypeLineSubtypes"/>
    /// and says the rest in words (<see cref="ShownSubtypes"/>).
    /// </summary>
    [Key(49)]
    public bool EveryCreatureType { get; set; }

    /// <summary>The subtypes a type line prints when <see cref="EveryCreatureType"/>: those it has of its own. Empty otherwise.</summary>
    [Key(50)]
    public List<string> TypeLineSubtypes { get; set; } = [];

    /// <summary>The subtypes to print on a type line: <see cref="Subtypes"/>, or for an every-creature-type object its own.</summary>
    [IgnoreMember]
    public IReadOnlyList<string> ShownSubtypes => EveryCreatureType ? TypeLineSubtypes : Subtypes;

    /// <summary>
    /// Permissions to play this card from the zone it is in (CR 601.3, 305.1) - an impulse draw's "this turn", a flashback
    /// - each with the seat that holds it and what it says. Public: a card in exile is open to all.
    /// </summary>
    [Key(51)]
    public List<PlayPermissionView> PlayPermissions { get; set; } = [];

    /// <summary>A spell that can't be countered because of how it was cast (CR 101.2): Mistrise Village's next spell.</summary>
    [Key(52)]
    public bool CantBeCountered { get; set; }

    /// <summary>
    /// The abilities it printed and an effect has taken from it (CR 613.1f), as they read: keywords by name. The abilities it
    /// was given are <see cref="GrantedAbilities"/>.
    /// </summary>
    [Key(53)]
    public List<string> LostAbilities { get; set; } = [];

    /// <summary>
    /// For an adventurer or omen card, the Adventure or Omen printed in the inset frame of its text box (CR 715.2, 720.2),
    /// exactly as printed, as Twinmaw Stormbrood's Charring Bite; null for everything else, and for the spell on the stack
    /// that was cast as that face, which is the face itself (CR 715.3b, 720.3b).
    /// <para>
    /// The card has only its normal characteristics everywhere but the stack (CR 715.4, 720.4), but the inset is printed
    /// on it, so a player may read it whenever they may read the card: the client holds it up beside the card as it does a
    /// prepare spell. Printed and nothing more, as <see cref="OtherFace"/> is.
    /// </para>
    /// </summary>
    [Key(54)]
    public ObjectView? InsetSpell { get; set; }

    /// <summary>
    /// Whether this permanent is solved (CR 719.3b): a Case whose "To solve" ability has made it so. A designation it keeps
    /// until it leaves the battlefield; it turns on the Case's "Solved" ability (CR 719.3c).
    /// </summary>
    [Key(55)]
    public bool Solved { get; set; }

    /// <summary>
    /// Whether this permanent is suspected (CR 701.60b): a designation it keeps until it leaves the battlefield, giving it
    /// menace and "This creature can't block" (CR 701.60c).
    /// </summary>
    [Key(56)]
    public bool Suspected { get; set; }

    /// <summary>
    /// Whether this permanent is phased out (CR 702.26b): it is shown with the battlefield, but the game treats it as though
    /// it does not exist - nothing can target it, it can't attack or block, its abilities do nothing - until it phases in
    /// (CR 502.1, 610.4).
    /// </summary>
    [Key(57)]
    public bool PhasedOut { get; set; }

    /// <summary>
    /// The power and toughness box a station card prints in its striation (CR 721.2b), "2/2": the base power and toughness it
    /// has as a creature once it has enough charge counters, drawn in the striation in any zone. Presentation, never a
    /// characteristic (CR 721.2c: it has none until then); empty for any other card.
    /// </summary>
    [Key(59)]
    public string StationPt { get; set; } = string.Empty;

    /// <summary>
    /// For a prototype card, its prototype's keyword line as printed, "Prototype {2}{R} — 3/2" (CR 702.160a, 718.1): part of
    /// the card in every zone, though it has those values only while prototyped (CR 718.4). Empty for any other card.
    /// </summary>
    [Key(60)]
    public string Prototype { get; set; } = string.Empty;

    /// <summary>
    /// A prototyped spell or permanent, or a copy of one (CR 718.3b-d): its mana cost, power, toughness and colors, as this view
    /// gives them, are the prototype's; everything else is the card's. Not a copy of anything.
    /// </summary>
    [Key(61)]
    public bool Prototyped { get; set; }

    /// <summary>
    /// What is printed on this card besides what it shows, to be read beside it: its other face, then its prepare spell,
    /// then its Adventure or Omen.
    /// </summary>
    public IEnumerable<ObjectView> AlsoPrinted() => new[] { OtherFace, PrepareSpell, InsetSpell }.OfType<ObjectView>();

    public override string ToString() => Known ? $"{Handle} {Name}" : $"{Handle} (unknown)";
}

/// <summary>A standing permission to play a card, and whose.</summary>
[MessagePackObject]
public sealed class PlayPermissionView
{
    /// <summary>The seat that may play it.</summary>
    [Key(0)]
    public int Seat { get; set; }

    /// <summary>What it may do and until when, as it reads: "Playable this turn".</summary>
    [Key(1)]
    public string Text { get; set; } = string.Empty;
}

/// <summary>One ability an effect has given an object, as it reads.</summary>
[MessagePackObject]
public sealed class AbilityTextView
{
    [Key(0)]
    public string Text { get; set; } = string.Empty;

    /// <summary>Whether it has done all it may this turn or this game; see <see cref="ObjectView.SpentLines"/>.</summary>
    [Key(1)]
    public bool Spent { get; set; }

    public override string ToString() => Spent ? $"{Text} (spent)" : Text;
}

/// <summary>
/// One thing a spell or ability on the stack is aimed at: what it is, what the spell says happens to it, and whether it
/// is still a legal target.
/// </summary>
[MessagePackObject]
public sealed class SpellTargetView
{
    /// <summary>Set when the target is a player; zero otherwise.</summary>
    [Key(0)]
    public int Player { get; set; }

    /// <summary>Set when the target is an object; none otherwise.</summary>
    [Key(1)]
    public ViewHandle Object { get; set; }

    /// <summary>
    /// What the spell does to this target, as the card words it: "deals 2 damage to it", or a chosen mode's text. Empty
    /// when the spell has a single target, where its own text already says it.
    /// </summary>
    [Key(2)]
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// Whether this is still a legal target. A target that has stopped being one is kept rather than dropped, because
    /// what a spell was aimed at is public and stays worth showing; the spell simply does nothing to it (CR 608.2b).
    /// </summary>
    [Key(3)]
    public bool StillLegal { get; set; }

    [IgnoreMember]
    public bool IsPlayer => Player != 0;
}

/// <summary>One quality chosen about a permanent and kept on it (CR 614.12a): what kind of choice, and what was chosen.</summary>
[MessagePackObject]
public sealed class ChoiceView
{
    /// <summary>The kind of choice, as the card names it: <c>name</c>, <c>color</c>, <c>parity</c>.</summary>
    [Key(0)]
    public string Key { get; set; } = string.Empty;

    /// <summary>What was chosen, as it is shown.</summary>
    [Key(1)]
    public string Value { get; set; } = string.Empty;

    /// <summary>The permanent chosen, for a choice of one (<c>creature</c>, <c>permanent</c>, CR 607.2d): what is pointed at. None otherwise.</summary>
    [Key(2)]
    public ViewHandle Object { get; set; }
}

/// <summary>
/// A kind of counter on a permanent, and how many of it (CR 122).
/// <para>
/// Named rather than enumerated, because a counter <em>is</em> its name: a card may print a counter nobody has
/// heard of -- Gemstone Mine's mining counters -- and a client that has never heard of it still has to draw it and
/// call it what the card calls it. The well-known names are in <see cref="Cantrip.Core.CounterNames"/> for the
/// handful of places a client treats one specially, such as drawing loyalty in a planeswalker's own box.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class CounterView
{
    /// <summary>The counter's name as the card prints it, lower case: <c>+1/+1</c>, <c>stun</c>, <c>mining</c>.</summary>
    [Key(0)]
    public string Name { get; set; } = string.Empty;

    [Key(1)]
    public int Count { get; set; }
}

/// <summary>
/// A zone as the viewer sees it.
/// <para>
/// Either the contents are listed, or only a count is given. Which of the two is decided by the projector,
/// and a zone the viewer cannot look into never lists handles at all — not even opaque ones, since a
/// persistent opaque handle is exactly what lets a client track a card through a shuffle.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class ZoneView
{
    [Key(0)]
    public ZoneKindView Kind { get; set; }

    /// <summary>Seat owning the zone, or zero for a shared zone.</summary>
    [Key(1)]
    public int Owner { get; set; }

    [Key(2)]
    public int Count { get; set; }

    /// <summary>
    /// Contents in zone order for zones the viewer can look into, empty otherwise. Order is meaningful in
    /// a graveyard and on the stack, so it is preserved rather than sorted.
    /// </summary>
    [Key(3)]
    public List<ViewHandle> Contents { get; set; } = [];

    /// <summary>True when <see cref="Contents"/> is the whole story and not just a count.</summary>
    [Key(4)]
    public bool ContentsVisible { get; set; }

    /// <summary>
    /// Cards in a zone the viewer cannot look into, that the viewer nonetheless knows: what Duress showed of a hand and
    /// is still there, or cards put on top of a library that has not been shuffled since.
    /// <para>
    /// Descriptions, not handles. A handle is an identity, and an identity that persisted through a hidden zone would say
    /// which of two identical cards was played. A hand's known cards come in name order rather than hand order for the
    /// same reason; a library's come top first, with where each one is, because there the position is what is known.
    /// </para>
    /// </summary>
    [Key(5)]
    public List<KnownCardView> Known { get; set; } = [];

    public override string ToString() =>
        ContentsVisible ? $"{Kind}({Owner}) {Count} visible" : $"{Kind}({Owner}) {Count} hidden";
}

/// <summary>A card the viewer knows in a zone they cannot look into.</summary>
[MessagePackObject]
public sealed class KnownCardView
{
    /// <summary>How many cards are above it in a library, so zero for the top card; -1 in a zone with no order, such as a hand.</summary>
    [Key(0)]
    public int FromTop { get; set; } = -1;

    /// <summary>The card, with no handle: see <see cref="ZoneView.Known"/>.</summary>
    [Key(1)]
    public ObjectView Card { get; set; } = new();

    public override string ToString() => FromTop < 0 ? Card.Name : $"{Card.Name} ({FromTop} from top)";
}

[MessagePackObject]
public sealed class CombatView
{
    [Key(0)]
    public List<AttackerView> Attackers { get; set; } = [];

    [Key(1)]
    public List<BlockView> Blocks { get; set; } = [];
}

[MessagePackObject]
public sealed class AttackerView
{
    [Key(0)]
    public ViewHandle Attacker { get; set; }

    [Key(1)]
    public AttackTargetKindView TargetKind { get; set; }

    /// <summary>Seat being attacked, when the target is a player.</summary>
    [Key(2)]
    public int TargetPlayer { get; set; }

    /// <summary>Permanent being attacked, when the target is a planeswalker or battle.</summary>
    [Key(3)]
    public ViewHandle TargetPermanent { get; set; }

    [Key(4)]
    public bool WasBlocked { get; set; }

    /// <summary>The attacking band it is in (CR 702.22c), numbered from 1; 0 for none. A band is blocked as one (CR 702.22h).</summary>
    [Key(5)]
    public int Band { get; set; }
}

[MessagePackObject]
public sealed class BlockView
{
    [Key(0)]
    public ViewHandle Attacker { get; set; }

    /// <summary>Blockers in damage assignment order (CR 509.2), which is the attacking player's choice.</summary>
    [Key(1)]
    public List<ViewHandle> Blockers { get; set; } = [];
}
