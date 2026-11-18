using Cantrip.Client;
using Cantrip.Server;
using Xunit;

namespace Cantrip.BotLab.Tests;

/// <summary>Who sits where with what in a duel's games (<see cref="Program.Seating"/>): a pair of games is one deal.</summary>
public sealed class SeatingTests : IDisposable
{
    private readonly string _pairings = Path.Combine(Path.GetTempPath(), $"cantrip-botlab-pairings-{Guid.NewGuid():N}.txt");

    private static readonly DeckList Ux = DeckList.Parse("ux", "60 Island", "ux.txt");
    private static readonly DeckList Gx = DeckList.Parse("gx", "60 Forest", "gx.txt");
    private static readonly DeckList Rx = DeckList.Parse("rx", "60 Mountain", "rx.txt");

    public SeatingTests() => File.WriteAllLines(_pairings, ["ux:gx", "rx:rx", "gx:rx"]);

    public void Dispose()
    {
        Program.PairingsFile = string.Empty;
        Program.PairingsKeepDecks = false;
        Program.Pairings = [];
        File.Delete(_pairings);
    }

    private static readonly Func<int, int, IAutoPlayer> A = (_, _) => new AutoPlayer();
    private static readonly Func<int, int, IAutoPlayer> B = (game, seat) => new RandomPlay(seat);

    /// <summary>Game <paramref name="game"/>'s seats, seat 1 first: <c>ux gx / A B</c> is A with ux in seat 1.</summary>
    private static string Seat(int game)
    {
        (List<DeckList> decks, List<IAutoPlayer> players) = Program.Seating([Ux, Gx, Rx], game, A, B);
        return $"{string.Join(' ', decks.Select(d => d.Name))} / {string.Join(' ', players.Select(p => p is AutoPlayer ? "A" : "B"))}";
    }

    [Fact]
    public void APairOfGamesFromThePairingsIsOneDealWithThePlayersExchanged()
    {
        Program.PairingsFile = _pairings;
        Program.PairingsKeepDecks = false;
        Program.Pairings = [];

        // Each pair the next line, the same decks in the same seats both games, the players exchanged.
        Assert.Equal("ux gx / A B", Seat(0));
        Assert.Equal("ux gx / B A", Seat(1));
        Assert.Equal("rx rx / A B", Seat(2));
        Assert.Equal("rx rx / B A", Seat(3));
        Assert.Equal("gx rx / A B", Seat(4));
        Assert.Equal("gx rx / B A", Seat(5));
        Assert.Equal("ux gx / B A", Seat(7));

        // And both from one shuffle.
        Assert.Equal(Program.Shuffle(1000, 4), Program.Shuffle(1000, 5));
        Assert.NotEqual(Program.Shuffle(1000, 5), Program.Shuffle(1000, 6));
    }

    [Fact]
    public void KeepingTheDecksMovesEachWithItsPlayerAsBefore()
    {
        Program.PairingsFile = _pairings;
        Program.PairingsKeepDecks = true;
        Program.Pairings = [];

        Assert.Equal("ux gx / A B", Seat(0));
        Assert.Equal("gx ux / B A", Seat(1));
    }
}
