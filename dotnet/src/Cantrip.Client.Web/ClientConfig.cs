using System.Globalization;
using System.Runtime.InteropServices.JavaScript;
using Cantrip.Client.Tactics;
using Cantrip.Protocol;

namespace Cantrip.Client.Web;

/// <summary>
/// Where this client finds its game and its card art, and whom it plays as, read from the page's query string:
/// <c>?server=ws://host:7777/game&amp;seat=1&amp;name=Player%201&amp;art=https://cdn.example/art/{size}/{slug}.webp</c>.
/// <para>
/// Where art is, is the table's to say (<see cref="TableMessage.Art"/>), so it never depends on how the page was reached:
/// a link, a move from another table, or a ticket opened again on another computer. Only a page whose table says nothing -
/// a prompt fixture, or a table whose host was not told - goes by its own <c>?art=</c>, and failing that the game server's
/// art service on the port after the table's. Either way a bare base URL has the standard <c>/art/{size}/{slug}.webp</c>
/// path appended.
/// </para>
/// </summary>
public static class ClientConfig
{
    private const string ArtPath = "/art/{size}/{slug}.webp";

    static ClientConfig()
    {
        JSObject location = JSHost.GlobalThis.GetPropertyAsJSObject("location")!;
        string host = location.GetPropertyAsString("hostname") is { Length: > 0 } name ? name : "localhost";
        Dictionary<string, string> query = Parse(location.GetPropertyAsString("search") ?? string.Empty);

        Page = new Uri(location.GetPropertyAsString("href") ?? $"http://{host}/");
        Server = new Uri(query.GetValueOrDefault("server") ?? $"ws://{host}:7777/game");
        Fixture = query.GetValueOrDefault("prompt");
        // Ticket.js has usually already taken it out of the address and kept it aside.
        Ticket = (JSHost.GlobalThis.GetPropertyAsString("cantripTicket") ?? query.GetValueOrDefault("ticket")) is { Length: > 0 } ticket ? ticket : null;
        Seat = int.TryParse(query.GetValueOrDefault("seat"), out int seat) && seat > 0 ? seat : Ticket is null ? 1 : 0;
        Name = query.GetValueOrDefault("name") is { Length: > 0 } given ? given : "Web Client";
        Perf = query.ContainsKey("perf");
        Test = query.ContainsKey("test");
        CardSheet = query.ContainsKey("cardsheet");
        Decks = query.ContainsKey("decks");
        // The advisor's table (MainPage.Advisor.cs): its API, beside a table of its own; never a real table's page.
        Advisor = query.GetValueOrDefault("advisor") is { Length: > 0 } advisor && Uri.TryCreate(advisor, UriKind.Absolute, out Uri? api) ? api : null;
        DeckId = query.GetValueOrDefault("deck") is { Length: > 0 } deck ? deck : null;
        DeckFormatKey = query.GetValueOrDefault("format") is { Length: > 0 } format ? format : null;
        Auto = query.ContainsKey("auto");
        Narrate = query.ContainsKey("narrate");
        BotSeed = int.TryParse(query.GetValueOrDefault("bot-seed"), out int botSeed) ? botSeed : null;
        Bot = query.GetValueOrDefault("bot") is { Length: > 0 } bot ? bot.ToLowerInvariant() : null;
        BotSeats = [.. (query.GetValueOrDefault("bots") ?? string.Empty)
            .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Select(text => int.TryParse(text, NumberStyles.None, CultureInfo.InvariantCulture, out int botSeat) ? botSeat : 0)
            .Where(botSeat => botSeat > 0 && botSeat != Seat)
            .Distinct()];
        BotTickets = [.. (query.GetValueOrDefault("bot-tickets") ?? string.Empty)
            .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Distinct()];

        // The deck editor has no table to say where art is: it is beside the page, on the site that served it.
        ArtTemplate = query.GetValueOrDefault("art") is { Length: > 0 } art
            ? Template(art)
            : Decks
                ? Page.GetLeftPart(UriPartial.Authority) + ArtPath
                : $"{(Server.Scheme == "wss" ? "https" : "http")}://{Server.Host}:{Server.Port + 1}{ArtPath}";
    }

    private static string Template(string art) =>
        art.Contains("{slug}", StringComparison.Ordinal) ? art : art.TrimEnd('/') + ArtPath;

    /// <summary>The page's own address, which what the server names relative to it - a player's picture - is resolved against.</summary>
    public static Uri Page { get; }

    public static Uri Server { get; }

    /// <summary>
    /// The seat to ask for (<c>?seat=2</c>); when not given, seat 1, or with a ticket whichever seat the ticket was
    /// issued for.
    /// </summary>
    public static int Seat { get; }

    /// <summary>
    /// A seat ticket (<c>?ticket=...</c>): what a host such as the Discord bot
    /// hands one player to claim one seat. A bearer credential, so it is sent in the join, shown nowhere, and taken out
    /// of the address bar once read; a reload resumes the seat from the membership it bought instead.
    /// </summary>
    public static string? Ticket { get; }

    /// <summary>The name to sit down under (<c>?name=Player%202</c>).</summary>
    public static string Name { get; }

    /// <summary>
    /// A prompt fixture to show instead of joining a game (<c>?prompt=blockers</c>): a canned position for that dock, fed through an in-memory transport, as the prompt tests draw it.
    /// </summary>
    public static string? Fixture { get; }

    /// <summary>
    /// Whether to write what drawing card faces costs to the browser's console (<c>?perf</c>): a line a second while faces
    /// are being drawn, and one when the deck is ready (<see cref="FacePerf"/>).
    /// </summary>
    public static bool Perf { get; }

    /// <summary>
    /// Whether to publish the page's state for Playwright driving the browser (<c>?test</c>), since the table is drawn
    /// on canvases with no DOM to find anything in (<see cref="MainPage"/>'s TestState).
    /// </summary>
    public static bool Test { get; }

    /// <summary>Whether to show a sheet of sample card faces instead of the table (<c>?cardsheet</c>), for pictures of how a template lays a face out (<see cref="CardSheetElement"/>).</summary>
    public static bool CardSheet { get; }

    /// <summary>
    /// Whether to show the constructed deck editor instead of a table (<c>?decks</c>): a player's saved decks, the cards
    /// to build them from and the deck in columns, from the bot's site rather than a game server (<see cref="MainPage"/>'s
    /// deck editor).
    /// </summary>
    public static bool Decks { get; }

    /// <summary>
    /// The advisor's API (<c>?advisor=http://localhost:5390/advisor</c>), for a page at the advisor's own table
    /// (dotnet/tools/Cantrip.Advisor): the table as ever, and beside it what the AI would do. Null for every other page.
    /// </summary>
    public static Uri? Advisor { get; }

    /// <summary>
    /// The saved deck the deck editor opens with (<c>?decks&amp;deck=ID</c>, as the site's deck library links it), or
    /// null for a new one.
    /// </summary>
    public static string? DeckId { get; }

    /// <summary>The format a new deck in the deck editor starts in (<c>?decks&amp;format=modern</c>); null for Standard.</summary>
    public static string? DeckFormatKey { get; }

    /// <summary>Whether this seat plays itself (<c>?auto</c>), as a bot seat would, for a test to watch a whole game.</summary>
    public static bool Auto { get; }

    /// <summary>
    /// Whether the tactical seats this page plays for its opponents (<c>?bots=</c>, <c>?bot-tickets=</c>) write each
    /// decision they make and what they weighed to the browser's console (<c>?narrate</c>), a line at a time prefixed
    /// with the seat's name, so a game lost to a person can be read afterwards (<see cref="TacticalPlay.Narrator"/>) -
    /// and in a draft, each pick and the deck built (<see cref="DraftPlay.Narrator"/>).
    /// </summary>
    public static bool Narrate { get; }

    /// <summary>Under <c>?auto</c>, draws at random from this seed (<c>?bot-seed=N</c>) rather than playing the one deterministic way.</summary>
    public static int? BotSeed { get; }

    /// <summary>
    /// How a seat nobody is sitting at is played (<c>?bot=tactical</c>, <c>random</c> or <c>simple</c>), or null for the
    /// default: the tactical seat for the opponents this page plays (<see cref="BotSeats"/>), and for <c>?auto</c> the
    /// one deterministic way, or at random from <see cref="BotSeed"/>, which is what the tests that watch it rely on.
    /// </summary>
    public static string? Bot { get; }

    /// <summary>
    /// The other seats this page plays itself (<c>?bots=2</c>, or <c>?bots=2,3,4</c>), each over a connection of its own
    /// and answered in this browser: an opponent that costs its player's machine and nobody else's (MainPage.Bots).
    /// </summary>
    public static IReadOnlyList<int> BotSeats { get; }

    /// <summary>
    /// Seats this page plays itself that are claimed by ticket rather than by number (<c>?bot-tickets=a,b</c>): what a
    /// host hands a player along with their own ticket when the table's bots are theirs to run.
    /// </summary>
    public static IReadOnlyList<string> BotTickets { get; }

    /// <summary>A new player for a seat nobody is sitting at, as <see cref="Bot"/> says.</summary>
    /// <param name="self">Whether it is this page's own seat under <c>?auto</c>, rather than an opponent's.</param>
    /// <param name="name">The seat's name, which its narration (<see cref="Narrate"/>) is prefixed with.</param>
    public static IAutoPlayer NewBot(bool self = false, string name = "Bot") => Bot switch
    {
        "tactical" => new TacticalPlay(seed: BotSeed) { Narrator = Narration(self, name) },
        "random" => new RandomPlay(BotSeed),
        "simple" => new AutoPlayer(),
        _ when self => BotSeed is { } seed ? new RandomPlay(seed) : new AutoPlayer(),
        _ => new TacticalPlay { Narrator = Narration(self, name) },
    };

    private static TextWriter? Narration(bool self, string name) => Narrate && !self ? new ConsoleNarration(name) : null;

    /// <summary>Each whole line written to it, to the browser's console with the seat's name in front.</summary>
    private sealed class ConsoleNarration(string name) : TextWriter
    {
        private readonly System.Text.StringBuilder _line = new();

        public override System.Text.Encoding Encoding => System.Text.Encoding.UTF8;

        public override void Write(char value)
        {
            if (value == '\n')
            {
                Console.WriteLine($"[{name}] {_line.ToString().TrimEnd('\r')}");
                _line.Clear();
            }
            else
            {
                _line.Append(value);
            }
        }
    }

    /// <summary>A URL with <c>{size}</c> and <c>{slug}</c> in it.</summary>
    public static string ArtTemplate { get; private set; }

    /// <summary>
    /// Takes where art is from the table (<see cref="TableMessage.Art"/>), over anything the page guessed; a table that
    /// says nothing, or names no http(s) address, leaves it be. Whether it changed, when art already asked for at the old
    /// address wants asking again.
    /// </summary>
    public static bool TableArt(string art)
    {
        if (!Uri.TryCreate(art, UriKind.Absolute, out Uri? address) || address.Scheme is not ("http" or "https"))
        {
            return false;
        }

        string template = Template(art);
        if (template == ArtTemplate)
        {
            return false;
        }

        ArtTemplate = template;
        return true;
    }

    public static Uri ArtUrl(string slug, int size) =>
        new(ArtTemplate.Replace("{size}", size.ToString(System.Globalization.CultureInfo.InvariantCulture), StringComparison.Ordinal)
            .Replace("{slug}", Uri.EscapeDataString(slug), StringComparison.Ordinal));

    /// <summary>
    /// Where the art service says who illustrated each picture: beside the sizes, as its cache folder keeps it. Null for an
    /// art address not laid out that way, which has no credits to ask for.
    /// </summary>
    public static Uri? CreditsUrl =>
        ArtTemplate.EndsWith(ArtTail, StringComparison.Ordinal) ? new(ArtTemplate[..^ArtTail.Length] + "artists.json") : null;

    private const string ArtTail = "{size}/{slug}.webp";

    private static Dictionary<string, string> Parse(string search)
    {
        Dictionary<string, string> values = new(StringComparer.OrdinalIgnoreCase);

        foreach (string pair in search.TrimStart('?').Split('&', StringSplitOptions.RemoveEmptyEntries))
        {
            int equals = pair.IndexOf('=');
            if (equals > 0)
            {
                values[Uri.UnescapeDataString(pair[..equals])] = Uri.UnescapeDataString(pair[(equals + 1)..].Replace('+', ' '));
            }
            else if (equals < 0)
            {
                // A switch with no value, such as ?perf.
                values[Uri.UnescapeDataString(pair)] = string.Empty;
            }
        }

        return values;
    }
}
