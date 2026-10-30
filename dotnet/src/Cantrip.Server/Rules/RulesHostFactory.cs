namespace Cantrip.Server.Rules;

/// <summary>One seat of a game to be dealt: who sits there and the cards they bring, by name.</summary>
/// <param name="Deck">The deck, one entry per copy, top first when dealt unshuffled.</param>
/// <param name="Sideboard">
/// The cards the seat owns outside the game (CR 100.4, 400.11a): what "a card you own from outside the game" chooses among
/// (North Wind Avatar). A card the host does not know is refused here, as in the deck.
/// </param>
/// <param name="Companion">
/// The card chosen as the seat's companion at deck building (CR 702.139a), dealt into its sideboard beside the cards there
/// and revealed before the game (CR 103.2b); null for none. The server has refused a deck whose companion's condition it
/// does not fulfil (<see cref="Cantrip.Server.Decks.DeckValidation"/>).
/// </param>
public sealed record RulesSeat(string Name, IReadOnlyList<string> Deck, IReadOnlyList<string> Sideboard, string? Companion = null)
{
    public RulesSeat(string name, IReadOnlyList<string> deck) : this(name, deck, [])
    {
    }

    /// <summary>
    /// The decklist's name, where the table has one (<see cref="DeckList.Name"/>): told to the host only so that a
    /// recording host (<c>--ai-record</c>) can label the seat's archetype. Null sends nothing.
    /// </summary>
    public string? DeckName { get; init; }
}

/// <summary>Starting conditions beyond the seed (CR 103).</summary>
public sealed class RulesGameOptions
{
    /// <summary>
    /// The seat to take the first turn, from one; null plays out CR 103.1, where a seat chosen at random decides.
    /// </summary>
    public int? FirstPlayer { get; init; }

    /// <summary>The seat that chooses who goes first instead of one picked at random (CR 103.1); null for random.</summary>
    public int? Chooser { get; init; }

    /// <summary>
    /// The minimum deck size of the table's format (CR 100.2a: sixty in Constructed; 100.2b: forty in Limited, or the event's
    /// own), which the engine is told outright and a companion's condition may name (Yorion, Sky Nomad). A scenario's is its
    /// own.
    /// </summary>
    public required int MinimumDeckSize { get; init; }

    /// <summary>Whether the decks are shuffled. False deals them in order, which is what a scenario wants.</summary>
    public bool Shuffle { get; init; } = true;

    /// <summary>
    /// In a Limited game, the basic lands the event supplies, of which every sideboard holds any number (Magic Tournament
    /// Rules; North Wind Avatar's ruling): one brought into the game from outside it leaves another there. Empty otherwise.
    /// </summary>
    public IReadOnlyList<string> BasicLandsOutside { get; init; } = [];

    /// <summary>A <c>.cantrip</c> scenario to play instead of dealing the seats' decks; null for a dealt game.</summary>
    public ScenarioSpec? Scenario { get; init; }
}

/// <summary>Deals a game on a rules engine, from a description of its seats.</summary>
public interface IRulesHostFactory
{
    /// <summary>
    /// Deals a new game: the seats in turn order, dealt from <paramref name="seed"/>, which with the decks is what the
    /// game is resumed from.
    /// </summary>
    /// <exception cref="InvalidOperationException">A deck names a card the rules do not know.</exception>
    IRulesHost Create(IReadOnlyList<RulesSeat> seats, ulong seed, RulesGameOptions options);

    /// <summary>
    /// Whether a starting deck fulfils the condition of <paramref name="companion"/>'s companion ability (CR 702.139a), the
    /// format's minimum deck size being <paramref name="minimumDeckSize"/>: the rules engine's judgement, which the server
    /// asks before it deals a deck with a companion. False for a card with no companion.
    /// </summary>
    bool CompanionFulfilled(IReadOnlyList<string> deck, string companion, int minimumDeckSize) =>
        NativeRulesHost.Companions(NativeHostProcess.Shared, deck, [companion], minimumDeckSize) is [{ Fulfilled: true }];
}

/// <summary>
/// The rules engine's factory: the native engine's sidecar, <see cref="NativeHostProcess.Shared"/>, or a transport given
/// for a test.
/// </summary>
public static class RulesHostFactory
{
    /// <summary>The factory every table deals its games through unless it is given another.</summary>
    public static IRulesHostFactory Native { get; } = new NativeRulesHostFactory();
}

/// <summary>
/// Deals on the native engine's sidecar: <see cref="NativeHostProcess.Shared"/>, or a transport given for a test. The
/// cards are the engine's own, named as the decks name them; a card it has not ported is the host's refusal to deal
/// (<see cref="RulesRejectedException"/>).
/// </summary>
public sealed class NativeRulesHostFactory : IRulesHostFactory
{
    private readonly IHostTransport? _transport;

    public NativeRulesHostFactory(IHostTransport? transport = null)
    {
        _transport = transport;
    }

    /// <summary>
    /// Whether each game dealt also projects the position for a viewer who is no player (<see cref="RulesStep.Public"/>),
    /// for in-person play's tablets (docs/in-person.md): the host is told <c>"Public": true</c> as each game is dealt. Off, it
    /// works none of that out.
    /// </summary>
    public bool PublicView { get; init; }

    public IRulesHost Create(IReadOnlyList<RulesSeat> seats, ulong seed, RulesGameOptions options) =>
        NativeRulesHost.NewGame(_transport ?? NativeHostProcess.Shared, seats, seed, options, publicView: PublicView);

    public bool CompanionFulfilled(IReadOnlyList<string> deck, string companion, int minimumDeckSize) =>
        NativeRulesHost.Companions(_transport ?? NativeHostProcess.Shared, deck, [companion], minimumDeckSize) is [{ Fulfilled: true }];
}
