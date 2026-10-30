using System.Diagnostics;
using System.Globalization;
using System.Text.Json.Nodes;
using Cantrip.Advisor;
using Cantrip.Server.Host;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Logging;

// The advisor's table (tools\advisor.cmd): a person in the AI's seat against a bot, the AI's advice at every decision,
// every decision recorded. It needs a trained model (--model), which is not in the repository.
//
// Usage: cantrip-advisor --mine DECK --theirs DECK --model DIR [--opponent neural|tactical] [--server HOST:PORT]
//                        [--budget MS] [--opponent-budget MS] [--worlds N] [--seed N] [--port N] [--sessions DIR]
//                        [--prior DIR] [--rater ID] [--no-browser]
//
// DECK is a decklist path or a name under --decks (..\ai-data\decks beside the repository by default). --server is the
// inference server the AI host evaluates on (python -m cantrip_ai.serve); without it the AI host evaluates on the CPU.
// The page is served on --port (5390 by default) and opened in the default browser. Sessions go under --sessions
// (..\ai-data\sessions by default).

string root = FindRoot();
string decks = Arg("--decks") ?? Path.GetFullPath(Path.Combine(root, "..", "ai-data", "decks"));
string? mine = Arg("--mine");
string? theirs = Arg("--theirs");
string? model = Arg("--model");
if (mine is null || theirs is null || model is null)
{
    Console.Error.WriteLine("Usage: cantrip-advisor --mine DECK --theirs DECK --model DIR [--opponent neural|tactical] [--server HOST:PORT] [--budget MS] [--port N]");
    return 2;
}

string? web = WebClientFiles.Built(root);
if (web is null)
{
    Console.Error.WriteLine("The web client is not built: dotnet build dotnet\\src\\Cantrip.Client.Web -f net10.0-browserwasm.");
    return 1;
}

ulong seed = ulong.TryParse(Arg("--seed"), NumberStyles.None, CultureInfo.InvariantCulture, out ulong s) && s > 0
    ? s
    : (ulong)Random.Shared.NextInt64(1, int.MaxValue);
var options = new AdvisorOptions
{
    Root = root,
    MyDeck = Deck(mine),
    TheirDeck = Deck(theirs),
    Model = Path.GetFullPath(model),
    Opponent = (Arg("--opponent") ?? "neural").ToLowerInvariant(),
    InferenceServer = Arg("--server"),
    BudgetMs = Number("--budget", 3000),
    OpponentBudgetMs = Number("--opponent-budget", Number("--budget", 3000)),
    Worlds = Number("--worlds", 8),
    Seed = seed,
    SessionsRoot = Arg("--sessions") ?? Path.GetFullPath(Path.Combine(root, "..", "ai-data", "sessions")),
    Prior = Arg("--prior") ?? decks,
    Rater = Arg("--rater") ?? "user",
    ArtCache = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Cantrip", "art-cache"),
};
if (options.Opponent is not ("neural" or "tactical"))
{
    Console.Error.WriteLine("--opponent is neural or tactical.");
    return 2;
}

Console.WriteLine($"Seed {seed}. You: {Path.GetFileNameWithoutExtension(options.MyDeck)}. The bot ({options.Opponent}): {Path.GetFileNameWithoutExtension(options.TheirDeck)}.");
await using AdvisorTable table = await AdvisorTable.StartAsync(options);
await using WebApplication app = AdvisorSite.Build(table, web, Number("--port", 5390));
await app.StartAsync();
string site = app.Urls.First();
string page = $"{site}/?server={Uri.EscapeDataString(table.Table.ToString())}&ticket={table.Ticket}&name={Uri.EscapeDataString(options.PlayerName)}&advisor={Uri.EscapeDataString(site + "/advisor")}";
Console.WriteLine($"The table: {page}");
if (!args.Contains("--no-browser"))
{
    Process.Start(new ProcessStartInfo(page) { UseShellExecute = true });
}

using var stopping = new CancellationTokenSource();
Console.CancelKeyPress += (_, e) =>
{
    e.Cancel = true;
    stopping.Cancel();
};
try
{
    await table.Finished.WaitAsync(stopping.Token);
    Console.WriteLine($"The game is over. Recorded in {table.SessionDirectory}. The page stays up for review; Ctrl+C to stop.");
    await Task.Delay(Timeout.Infinite, stopping.Token);
}
catch (OperationCanceledException)
{
    Console.WriteLine("Stopping.");
}

return 0;

string Deck(string deck) => File.Exists(deck) ? Path.GetFullPath(deck) : Cantrip.Server.DeckList.Find(decks, deck);

string? Arg(string name)
{
    int at = Array.IndexOf(args, name);
    return at >= 0 && at + 1 < args.Length ? args[at + 1] : null;
}

int Number(string name, int fallback) => int.TryParse(Arg(name), NumberStyles.Integer, CultureInfo.InvariantCulture, out int n) ? n : fallback;

static string FindRoot()
{
    for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir is not null; dir = dir.Parent)
    {
        if (Directory.Exists(Path.Combine(dir.FullName, "content", "mtg")) && File.Exists(Path.Combine(dir.FullName, "Cargo.toml")))
        {
            return dir.FullName;
        }
    }

    throw new DirectoryNotFoundException("No repository root above the advisor.");
}

/// <summary>The advisor's site: the web client's files and the advisor's API, on one port.</summary>
public static class AdvisorSite
{
    /// <summary>
    /// The site for <paramref name="table"/>: <c>/advisor/state?since=N</c> (long-polled), <c>/advisor/settings</c> and
    /// <c>/advisor/note</c>, and the built client in <paramref name="web"/> at the root. The API answers any origin: it is
    /// on the loopback only, and the browser tests serve the page from a port of their own.
    /// </summary>
    public static WebApplication Build(AdvisorTable table, string web, int port)
    {
        ArgumentNullException.ThrowIfNull(table);
        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.Logging.ClearProviders();
        builder.WebHost.UseUrls($"http://127.0.0.1:{port}");
        WebApplication app = builder.Build();
        app.Use(async (context, next) =>
        {
            if (context.Request.Path.StartsWithSegments("/advisor", StringComparison.Ordinal))
            {
                context.Response.Headers.AccessControlAllowOrigin = "*";
                context.Response.Headers.AccessControlAllowHeaders = "Content-Type";
                context.Response.Headers.AccessControlAllowMethods = "GET, POST, OPTIONS";
                context.Response.Headers.CacheControl = "no-store";
                if (HttpMethods.IsOptions(context.Request.Method))
                {
                    context.Response.StatusCode = StatusCodes.Status204NoContent;
                    return;
                }
            }

            await next(context);
        });
        app.MapGet("/advisor/state", async (int? since, HttpContext context) =>
        {
            JsonObject state = await table.StateAsync(since ?? -1, TimeSpan.FromSeconds(20), context.RequestAborted);
            return Results.Text(state.ToJsonString(), "application/json");
        });
        app.MapPost("/advisor/settings", async (HttpContext context) =>
        {
            JsonNode? body = await JsonNode.ParseAsync(context.Request.Body);
            table.Settings(body?["Hidden"]?.GetValue<bool>(), body?["AutoTrivial"]?.GetValue<bool>());
            return Results.NoContent();
        });
        app.MapPost("/advisor/note", async (HttpContext context) =>
        {
            JsonNode? body = await JsonNode.ParseAsync(context.Request.Body);
            if (body?["RequestId"]?.GetValue<int>() is int requestId)
            {
                table.Note(requestId, body["Reason"]?.GetValue<string>(), body["Flagged"]?.GetValue<bool>(), body["Via"]?.GetValue<string>());
            }

            return Results.NoContent();
        });
        app.UseWebClient(web);
        return app;
    }
}
