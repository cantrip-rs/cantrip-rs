using System.Text.Json;
using System.Text.Json.Nodes;
using Cantrip.Advisor;
using Cantrip.Server.Host;
using Microsoft.Playwright;
using Xunit;
using Xunit.Abstractions;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// The advisor's table (<c>?advisor</c>, dotnet/tools/Cantrip.Advisor): a person in the AI's seat against the tactical bot,
/// the AI's advice beside the table at every decision, the play made through this seat's own client, and every decision
/// recorded. It needs a trained model, which is not in the repository (<c>CANTRIP_ADVISOR_MODEL</c>, else the champion
/// in force in the data directory beside it, <c>../ai-data/champions/current</c>) and the advisor's AI host built
/// (<c>cargo build --profile cards -p cantrip-ai-host</c>); without them it says so and passes. The AI evaluates on the CPU here, with a short budget.
/// </summary>
[Collection(WebClientCollection.Name)]
public sealed class AdvisorTests(WebClient web, ITestOutputHelper output)
{
    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(90);

    [Fact]
    public async Task The_advice_is_shown_played_hidden_recorded_and_the_truth_revealed_after_the_game()
    {
        string? model = Model();
        if (model is null || !File.Exists(Path.Combine(WebClient.Root, "target", "cards", "cantrip-advisor-host.exe")))
        {
            output.WriteLine("No AI model or advisor host on this machine: the advisor's test does not run here.");
            return;
        }

        var logs = new System.Collections.Concurrent.ConcurrentQueue<string>();
        string sessions = Path.Combine(WebClient.Root, "TestResults", "advisor-sessions");
        string decks = Path.Combine(WebClient.Root, "decks");
        await using AdvisorTable table = await AdvisorTable.StartAsync(new AdvisorOptions
        {
            Root = WebClient.Root,
            MyDeck = Cantrip.Server.DeckList.Find(decks, "demo/1-black-red"),
            TheirDeck = Cantrip.Server.DeckList.Find(decks, "demo/2-white-blue"),
            Opponent = "tactical",
            Model = model,
            BudgetMs = 400,
            Worlds = 4,
            Seed = 11,
            SessionsRoot = sessions,
            ArtCache = WebClient.ArtCache,
            Log = logs.Enqueue,
        });
        await using Microsoft.AspNetCore.Builder.WebApplication site = AdvisorSite.Build(table, WebClientFiles.Built(WebClient.Root)!, 0);
        await site.StartAsync();
        string api = site.Urls.First() + "/advisor";

        await using TablePage page = await TablePage.OpenAsync(
            web,
            $"server={Uri.EscapeDataString(table.Table.AbsoluteUri)}&ticket={table.Ticket}&name=You&advisor={Uri.EscapeDataString(api)}",
            WebClient.Pictures(nameof(AdvisorTests)));

        // The first decision of this seat with a real choice: the advice, with its candidates, beside the prompt.
        JsonNode advice = await AdviceAsync(page, a => a["status"]!.GetValue<string>() == "ready" && a["matchesPrompt"]!.GetValue<bool>() && !a["trivial"]!.GetValue<bool>());
        Assert.StartsWith("Bot: ", advice["choice"]!.GetValue<string>(), StringComparison.Ordinal);
        Assert.True(advice["candidates"]!.GetValue<int>() >= 1);
        await page.SettledAsync();
        await page.PictureAsync("advice");

        // Hidden until chosen: the suggestion goes, and comes back when the box is cleared. Read again once the table has
        // settled, so the panel's controls are where they are drawn now.
        advice = await AdviceAsync(page, a => Control(a, "hide") is not null);
        await ClickAsync(page, advice, "hide");
        await AdviceAsync(page, a => a["choice"]!.GetValue<string>().Contains("hidden", StringComparison.Ordinal));
        await page.PictureAsync("hidden");
        advice = await AdviceAsync(page, a => a["controls"]!.AsArray().Any(c => c!["name"]!.GetValue<string>() == "hide"));
        await ClickAsync(page, advice, "hide");
        advice = await AdviceAsync(page, a => a["choice"]!.GetValue<string>().StartsWith("Bot: ", StringComparison.Ordinal) && Control(a, "accept") is not null);

        // A reason, then the bot's choice played with one click: the record says so.
        int requestId = advice["requestId"]!.GetValue<int>();
        await ClickAsync(page, advice, "reason");
        await page.Page.Keyboard.TypeAsync("keeping a two-lander on the draw");
        await page.Page.Keyboard.PressAsync("Enter");
        await ClickAsync(page, advice, "accept");
        JsonNode record = await RecordAsync(table, requestId, r => r["Answers"]![0]!["Reason"]!.GetValue<string>().Length > 0 && r["Answers"]![0]!["Via"] is not null);
        Assert.True(record["Answers"]![0]!["Agreed"]!.GetValue<bool>());
        Assert.Equal("accept", record["Answers"]![0]!["Via"]!.GetValue<string>());
        Assert.Equal("keeping a two-lander on the draw", record["Answers"]![0]!["Reason"]!.GetValue<string>());
        Assert.NotNull(record["Truth"]);
        Assert.NotNull(record["Stats"]);

        // The rest of the game, the bot's choice taken each time, to its end.
        DateTime giveUp = DateTime.UtcNow + TimeSpan.FromMinutes(8);
        int last = requestId;
        bool pictured = false;
        JsonNode? state = null;
        while (DateTime.UtcNow < giveUp)
        {
            state = await ReadAsync(page);
            if (state?["over"]?.GetValue<bool>() == true)
            {
                break;
            }

            if (state is not null && state["status"]!.GetValue<string>() == "ready" && state["matchesPrompt"]!.GetValue<bool>()
                && !state["trivial"]!.GetValue<bool>() && state["requestId"]!.GetValue<int>() != last && Control(state, "accept") is not null)
            {
                if (!pictured && state["statusText"]!.GetValue<string>().Contains("Priority", StringComparison.Ordinal) && state["candidates"]!.GetValue<int>() > 1)
                {
                    pictured = true;
                    await page.PictureAsync("priority");
                }

                last = state["requestId"]!.GetValue<int>();
                await ClickAsync(page, state, "accept");
            }

            await Task.Delay(150);
        }

        foreach (string line in logs.Where(l => l.Contains("DIVERGED", StringComparison.Ordinal)))
        {
            output.WriteLine(line);
        }

        Assert.True(state?["over"]?.GetValue<bool>() == true, $"The game did not end in time: {state}");
        Assert.DoesNotContain("lost track", state!["statusText"]!.GetValue<string>(), StringComparison.Ordinal);
        await table.Finished.WaitAsync(Patience);

        // After the game, and only then, the true hidden cards on asking.
        Assert.Equal(string.Empty, state["truth"]!.GetValue<string>());
        JsonNode over = await AdviceAsync(page, a => Control(a, "reveal") is not null);
        await ClickAsync(page, over, "reveal");
        over = await AdviceAsync(page, a => a["truth"]!.GetValue<string>().Contains("hand", StringComparison.Ordinal));
        await page.PictureAsync("revealed");

        // The session: the game, its commands, its trace, and a record per decision.
        string dir = table.SessionDirectory;
        Assert.True(File.Exists(Path.Combine(dir, "game.json")));
        Assert.True(File.Exists(Path.Combine(dir, "commands.jsonl")));
        JsonNode trace = JsonNode.Parse(await File.ReadAllTextAsync(Path.Combine(dir, "trace.json")))!;
        Assert.NotEmpty(trace["steps"]!.AsArray());
        Assert.NotEmpty(Directory.GetFiles(Path.Combine(dir, "decisions"), "*.json"));
        Assert.Null(JsonNode.Parse(await File.ReadAllTextAsync(Path.Combine(dir, "game.json")))!["MirrorDivergence"]);
        output.WriteLine($"Recorded in {dir}; trace complete: {trace["complete"]}.");
    }

    /// <summary>
    /// A game of the Standard decks against the neural bot, the AI on the GPU inference server, as the advisor is played:
    /// only where <c>CANTRIP_ADVISOR_SERVER</c> names a running server (<c>python -m cantrip_ai.serve</c>), and
    /// <c>CANTRIP_ADVISOR_DECKS</c> holds the decks (<c>../ai-data/decks</c> by default). Pictures of the panel at a
    /// priority, a combat declaration and an unless-payment, the bot's choice taken each time.
    /// </summary>
    [Fact]
    public async Task A_standard_game_against_the_neural_bot_is_advised_from_the_inference_server()
    {
        string? model = Model();
        string? server = Environment.GetEnvironmentVariable("CANTRIP_ADVISOR_SERVER");
        string decks = Environment.GetEnvironmentVariable("CANTRIP_ADVISOR_DECKS") is { Length: > 0 } given ? given : Path.Combine(DataDirectory, "decks");
        if (model is null || server is not { Length: > 0 } || !Directory.Exists(decks))
        {
            output.WriteLine("No inference server named (CANTRIP_ADVISOR_SERVER): this game is not played here.");
            return;
        }

        var logs = new System.Collections.Concurrent.ConcurrentQueue<string>();
        await using AdvisorTable table = await AdvisorTable.StartAsync(new AdvisorOptions
        {
            Root = WebClient.Root,
            MyDeck = Cantrip.Server.DeckList.Find(decks, "dimir-midrange"),
            TheirDeck = Cantrip.Server.DeckList.Find(decks, "boros-dragons"),
            Opponent = "neural",
            Model = model,
            InferenceServer = server,
            BudgetMs = 3000,
            OpponentBudgetMs = 1000,
            Seed = 4242,
            Prior = decks,
            SessionsRoot = Path.Combine(WebClient.Root, "TestResults", "advisor-sessions"),
            ArtCache = WebClient.ArtCache,
            Log = logs.Enqueue,
        });
        await using Microsoft.AspNetCore.Builder.WebApplication site = AdvisorSite.Build(table, WebClientFiles.Built(WebClient.Root)!, 0);
        await site.StartAsync();
        await using TablePage page = await TablePage.OpenAsync(
            web,
            $"server={Uri.EscapeDataString(table.Table.AbsoluteUri)}&ticket={table.Ticket}&name=You&advisor={Uri.EscapeDataString(site.Urls.First() + "/advisor")}",
            WebClient.Pictures(nameof(AdvisorTests) + "-neural"));

        HashSet<string> pictured = [];
        DateTime giveUp = DateTime.UtcNow + TimeSpan.FromMinutes(12);
        int last = 0;
        JsonNode? state = null;
        while (DateTime.UtcNow < giveUp && pictured.Count < 4)
        {
            state = await ReadAsync(page);
            if (state?["over"]?.GetValue<bool>() == true)
            {
                break;
            }

            if (state is not null && state["status"]!.GetValue<string>() == "ready" && state["matchesPrompt"]!.GetValue<bool>()
                && !state["trivial"]!.GetValue<bool>() && state["requestId"]!.GetValue<int>() != last && Control(state, "accept") is not null)
            {
                string kind = state["statusText"]!.GetValue<string>();
                foreach (string what in (string[])["Mulligan", "Priority", "DeclareAttackers", "DeclareBlockers", "UnlessPays"])
                {
                    if (kind.Contains(what, StringComparison.Ordinal) && (what != "Priority" || state["candidates"]!.GetValue<int>() > 1) && pictured.Add(what))
                    {
                        await page.SettledAsync();
                        await page.PictureAsync(what);
                    }
                }

                last = state["requestId"]!.GetValue<int>();
                await ClickAsync(page, state, "accept");
            }

            await Task.Delay(150);
        }

        foreach (string line in logs.Where(l => l.Contains("DIVERGED", StringComparison.Ordinal)))
        {
            output.WriteLine(line);
        }

        Assert.DoesNotContain(logs, l => l.Contains("DIVERGED", StringComparison.Ordinal));
        Assert.NotEmpty(pictured);
        output.WriteLine($"Pictured {string.Join(", ", pictured)}; recorded in {table.SessionDirectory}.");
    }

    /// <summary>The data directory beside the repository, where models and decks are kept (docs/ai-training.md).</summary>
    private static string DataDirectory => Path.GetFullPath(Path.Combine(WebClient.Root, "..", "ai-data"));

    /// <summary>The model the advice is asked of: <c>CANTRIP_ADVISOR_MODEL</c>, else the champion in force.</summary>
    private static string? Model()
    {
        if (Environment.GetEnvironmentVariable("CANTRIP_ADVISOR_MODEL") is { Length: > 0 } given)
        {
            return Directory.Exists(given) ? given : null;
        }

        string candidate = Path.Combine(DataDirectory, "champions", "current");
        return File.Exists(Path.Combine(candidate, "manifest.json")) ? candidate : null;
    }

    private static async Task<JsonNode?> ReadAsync(TablePage page)
    {
        string? json = await page.Page.EvaluateAsync<string?>("() => globalThis.cantripAdvisor ?? null");
        return json is null ? null : JsonNode.Parse(json);
    }

    private static async Task<JsonNode> AdviceAsync(TablePage page, Func<JsonNode, bool> until)
    {
        DateTime giveUp = DateTime.UtcNow + Patience;
        JsonNode? state = null;
        while (DateTime.UtcNow < giveUp)
        {
            state = await ReadAsync(page);
            if (state is not null && until(state))
            {
                return state;
            }

            await Task.Delay(100);
        }

        Assert.Fail($"The advisor's panel never showed what was waited for. Last: {state}. Page errors: {string.Join(" | ", page.Errors)}");
        return null!;
    }

    private static JsonNode? Control(JsonNode state, string name) =>
        state["controls"]!.AsArray().FirstOrDefault(c => c!["name"]!.GetValue<string>() == name);

    private static async Task ClickAsync(TablePage page, JsonNode state, string name)
    {
        JsonNode control = Control(state, name) ?? throw new InvalidOperationException($"No control {name} on the advisor's panel: {state}");
        float x = control["x"]!.GetValue<float>() + (Math.Min(control["width"]!.GetValue<float>(), 40) / 2);
        float y = control["y"]!.GetValue<float>() + (control["height"]!.GetValue<float>() / 2);
        await page.Page.Mouse.ClickAsync(x, y);
    }

    private static async Task<JsonNode> RecordAsync(AdvisorTable table, int requestId, Func<JsonNode, bool> until)
    {
        DateTime giveUp = DateTime.UtcNow + Patience;
        while (DateTime.UtcNow < giveUp)
        {
            string dir = Path.Combine(table.SessionDirectory, "decisions");
            string? file = Directory.Exists(dir) ? Directory.GetFiles(dir, $"*-r{requestId}.json").FirstOrDefault() : null;
            if (file is not null)
            {
                try
                {
                    JsonNode record = JsonNode.Parse(await File.ReadAllTextAsync(file))!;
                    if (until(record))
                    {
                        return record;
                    }
                }
                catch (Exception error) when (error is IOException or JsonException)
                {
                    // Being written.
                }
            }

            await Task.Delay(200);
        }

        Assert.Fail($"No record of request {requestId} as expected in {table.SessionDirectory}.");
        return null!;
    }
}
