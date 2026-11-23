using System.Reflection;
using Cantrip.Client.Embedded;
using Cantrip.Decks;
using Cantrip.Server;
using Cantrip.Server.Rules;

namespace Cantrip.Client.Web;

/// <summary>
/// A game against the tactical bot with no server at all (<c>?local</c>; docs/engine-in-the-browser.md): the rules engine
/// runs in this page (<see cref="BrowserEngine"/>), dealt through the same <see cref="HostAndPlay"/> table the tools play
/// on, and this page's seat and the bot's are loopback transports to it. Everything above the transport - the table, the
/// prompts, the bot - is what a game on a server has.
/// </summary>
public sealed partial class MainPage
{
    /// <summary>The decks a local game can be dealt (<c>?deck=</c> and <c>?opponent=</c>): decks/demo's, built into the page.</summary>
    public static readonly string[] LocalDeckNames = ["1-black-red", "2-white-blue", "3-green-red", "4-red-black"];

    private HostedGame? _localGame;
    private AutoSeat? _localBot;
    private readonly DispatcherTimer _localTicker = new() { Interval = TimeSpan.FromMilliseconds(30) };

    private async Task StartLocalAsync()
    {
        _promptTitle.Text = "Loading the rules engine...";
        try
        {
            await BrowserEngine.LoadAsync(ClientConfig.Engine);
            DeckList mine = LocalDeck(ClientConfig.LocalDeck ?? LocalDeckNames[0]);
            DeckList theirs = LocalDeck(ClientConfig.LocalOpponent ?? LocalDeckNames[1]);
            const string bot = "Tactical bot";

            _localGame = HostAndPlay.StartTable(
                new NativeRulesHostFactory(new BrowserEngineTransport()),
                [mine, theirs],
                DeckFormat.Freeform,
                ClientConfig.LocalSeed ?? (ulong)Random.Shared.NextInt64(),
                [ClientConfig.Name, bot]);

            _gameClient = new GameClient(_localGame.TransportFor(1));
            _gameClient.Join(ClientConfig.Name, preferredSeat: 1);
            var botClient = new GameClient(_localGame.TransportFor(2));
            botClient.Join(bot, preferredSeat: 2);
            _localBot = new AutoSeat(botClient, ClientConfig.NewBot(name: bot));
            _localGame.Start();

            StartPumping();
            _localTicker.Tick += (_, _) => StepLocalGame();
            _localTicker.Start();
        }
        catch (Exception ex)
        {
            _promptTitle.Text = "Could not start a local game";
            _promptDetail.Text = ex.Message;
        }
    }

    /// <summary>The bot answers what it is asked, and the game's clocks run, as a table's do.</summary>
    private void StepLocalGame()
    {
        if (_localGame is null || _localBot is null)
        {
            return;
        }

        _localGame.Tick();
        try
        {
            if (_localBot.Step())
            {
                _needsRedraw = true;
            }
        }
        catch (Exception ex)
        {
            // As a bot seat on a server does (PlayBots): a question the bot faulted on is answered by its default, so the
            // game goes on; what went wrong is written to the browser's console.
            Console.Error.WriteLine($"The local bot faulted: {ex}");
            if (_localBot.Client is { Prompt: { } asked } client)
            {
                client.Answer(asked.Prompt.Default());
            }
        }
    }

    /// <summary>A deck of decks/demo, as built into the page.</summary>
    private static DeckList LocalDeck(string name)
    {
        if (!LocalDeckNames.Contains(name, StringComparer.Ordinal))
        {
            throw new ArgumentException($"There is no deck '{name}' for a local game; there are {string.Join(", ", LocalDeckNames)}.");
        }

        using Stream stream = Assembly.GetExecutingAssembly().GetManifestResourceStream($"deck_{name}")
            ?? throw new InvalidOperationException($"The deck '{name}' is not built into the page.");
        using var reader = new StreamReader(stream);
        return DeckList.Parse(name, reader.ReadToEnd(), $"decks/demo/{name}.txt");
    }
}
