using System.Net;
using System.Net.Sockets;
using System.Security.Claims;
using Cantrip.Art;
using Cantrip.Decks;
using Cantrip.Discord;
using Cantrip.Server;
using Cantrip.Server.Events;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Playwright;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// The bot's site in a browser, as its players use it: every page but a game table, at a desktop's width and a phone's,
/// signed out and signed in, each with the same menu bar - the logo, the sections, who is signed in - and footer; the
/// way from page to page by that bar, and on a phone by its menu; a player's own page with what they are playing and
/// their events over; dropping out of a draft from it, asked first; the deck library, opening a deck in the editor; and a
/// league's page, with the player's run, its deck, and ending it asked first.
/// Every page is pictured (TestResults\web\SitePagesTests) to be looked at.
/// <para>
/// Discord's sign-in is Discord's, so here the player is a header every request of the page carries, which the site's
/// test hook (<c>before</c>) turns into the claims the sign-in cookie would have.
/// </para>
/// </summary>
[Collection(WebClientCollection.Name)]
public sealed class SitePagesTests(WebClient web) : IAsyncLifetime
{
    private const string AsHeader = "X-Test-Account";
    private const ulong Alice = 1001;
    private const string League = "vintage-test";

    private static readonly (int Width, int Height, string Name)[] Sizes = [(1440, 900, "desktop"), (390, 844, "phone")];

    private readonly string _data = Path.Combine(Path.GetTempPath(), "cantrip-site-pages-" + Guid.NewGuid().ToString("N"));
    private readonly string _pictures = WebClient.Pictures(nameof(SitePagesTests));
    private readonly List<string> _errors = [];
    private WebApplication _site = null!;
    private DraftHost _drafts = null!;
    private DraftQueues _queues = null!;
    private Matchmaker _games = null!;
    private PodHosts _pods = null!;
    private LeagueHosts _leagues = null!;
    private Uri _address = null!;
    private string _live = string.Empty;
    private string _finished = string.Empty;

    public async Task InitializeAsync()
    {
        string built = Path.Combine(WebClient.Root, "dotnet", "src", "Cantrip.Client.Web", "bin", "Debug", "net10.0-browserwasm", "wwwroot");
        List<int> ports = FreePorts(8);
        var store = new EventStore(_data);
        // Bob has a picture; the others are shown by their initial.
        var portraits = new PortraitStore(store.Root);
        await portraits.SaveAsync("discord-1002", Convert.FromBase64String("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="));
        IReadOnlyList<string> cube = CubeList.Load(Path.Combine(WebClient.Root, "decks", "rw-aggro.txt"));
        _drafts = new DraftHost(new DraftHostOptions { ContentRoot = WebClient.Content, Store = store, Cube = "LSVCube", CubeCards = cube, Cards = 18, Ports = ports[1..4] });
        _queues = new DraftQueues(new DraftQueuesOptions
        {
            ContentRoot = WebClient.Content, Store = store, Drafts = _drafts, Ports = ports[1..4], CubeDirectories = [Path.Combine(WebClient.Root, "docs", "cube-lists")],
            Portraits = portraits.PathFor,
        });
        string decks = Path.Combine(WebClient.Root, "decks");
        DeckList rwAggro = DeckList.Load(DeckList.Find(decks, "rw-aggro"));
        _games = new Matchmaker(new MatchmakerOptions
        {
            ContentRoot = WebClient.Content,
            Deck = rwAggro,
            // Decks from the Standard gauntlet, as the bot's site offers Standard: the server deals only a deck legal in it.
            Formats = new Dictionary<string, IReadOnlyList<DeckList>>
            {
                ["standard"] = [DeckList.Load(DeckList.Find(Path.Combine(decks, "standard"), "boros-aggro")), DeckList.Load(DeckList.Find(Path.Combine(decks, "standard"), "dimir-midrange"))],
            },
            Ports = ports[4..],
            Portraits = portraits.PathFor,
        });

        var site = new Uri($"http://127.0.0.1:{ports[0]}/");
        var signIn = new SiteSignInOptions { Site = site, ClientId = "1234567890", ClientSecret = "not-a-secret", Keys = Path.Combine(_data, "site-keys") };
        var status = new EventStatusBoard(store, hosting: () => [.. _drafts.Drafts.Where(draft => draft.IsLive).Select(draft => draft.Id)],
            portraits: portraits.PathFor);
        var queuePage = new QueueSiteOptions { Queues = _queues, Board = status, Drafts = _drafts, Client = site, SignIn = signIn };
        var playPage = new PlaySiteOptions { Games = _games, Client = site, SignIn = signIn };
        var deckPage = new DeckSiteOptions { ContentRoot = WebClient.Content, Store = store, SignIn = signIn };
        // A league with runs being played and over, as the data directory keeps one.
        SeedLeague(store, cube);
        _pods = new PodHosts(new PodHostsOptions { ContentRoot = WebClient.Content, Store = store, Ports = ports[1..4] });
        _leagues = new LeagueHosts(new LeagueHostsOptions
        {
            ContentRoot = WebClient.Content,
            Store = store,
            Pods = _pods,
            CubeDirectories = [Path.Combine(WebClient.Root, "decks")],
            Portraits = portraits.PathFor,
            Leagues =
            [
                new LeagueDefinition
                {
                    Id = League,
                    Name = "Vintage Cube Quick Draft",
                    Description = "Draft the cube against seven bots, then play your deck against other players' quick-draft decks, best of one. Seven wins earns a trophy; three losses ends the run.",
                    CubeName = "MTGO Vintage Cube",
                    Settings = new LeagueSettings { Cube = "rw-aggro" },
                },
            ],
        });
        Assert.Empty(_leagues.Start());
        var leaguePage = new LeagueSiteOptions { Leagues = _leagues, Client = site, SignIn = signIn };

        var home = new HomeSiteOptions { SignIn = signIn, Drafts = queuePage, Constructed = playPage, Leagues = leaguePage, HasDecks = true, Store = store, Portraits = portraits.PathFor };
        var art = new ArtStore(new ArtStoreOptions { ContentRoot = WebClient.Content, CacheDirectory = WebClient.ArtCache });
        _site = await ClientSite.StartAsync("127.0.0.1", ports[0], built, art, status: status, signIn: signIn, queues: queuePage, play: playPage, home: home, decks: deckPage,
            before: SignInByHeader, leagues: leaguePage, portraits: portraits);
        _address = site;

        // Alice's history: a Winston draft she won, and one called off when Bob dropped; and three decks.
        _finished = Finished(store, cube);
        DraftSeating called = await _drafts.OpenAsync(new DiscordPlayer(Alice, "Alice"));
        await _drafts.JoinAsync(called.Draft!.Id, new DiscordPlayer(1002, "Bob"));
        Assert.Null(await _drafts.DropAsync(called.Draft.Id, new DiscordPlayer(1002, "Bob")));
        var library = new DeckStore(store);
        library.Save(Alice, new SavedDeck { Name = "Boros Burn", Format = "modern", Main = [new DeckEntry("Mountain", 20), new DeckEntry("Lightning Bolt", 4)] });
        library.Save(Alice, new SavedDeck { Name = "Mono-Red Aggro", Format = "standard", Main = [new DeckEntry("Mountain", 24)] });
        library.Save(Alice, new SavedDeck { Name = "Aang's Journey", Format = "commander", Commanders = [new DeckEntry("Aang, at the Crossroads", 1)], Main = [new DeckEntry("Forest", 99)] });

        // Being played: a Winston draft against Carol, and a table that is filling for anyone to see.
        DraftSeating live = await _drafts.OpenAsync(new DiscordPlayer(Alice, "Alice"));
        Assert.Null((await _drafts.JoinAsync(live.Draft!.Id, new DiscordPlayer(1003, "Carol"))).Refusal);
        _live = live.Draft.Id;
        Assert.Null((await _queues.CreateTableAsync(new QueueSettings { Format = "grid", Cube = "LSVCube", MinimumDeck = 40 }, new DiscordPlayer(1004, "Dana"))).Refusal);
        Assert.Null((await _games.HostAsync(new DiscordPlayer(1005, "Erin"), "standard", "boros-aggro")).Refusal);
    }

    public async Task DisposeAsync()
    {
        await _site.DisposeAsync();
        await _queues.DisposeAsync();
        await _drafts.DisposeAsync();
        await _games.DisposeAsync();
        await _leagues.DisposeAsync();
        await _pods.DisposeAsync();
        if (Directory.Exists(_data)) Directory.Delete(_data, recursive: true);
    }

    [Fact]
    public async Task Every_page_shares_one_menu_bar_and_footer_at_desktop_and_phone_width_signed_out_and_in()
    {
        string[] pages = ["", "about", "home", "draft", "leagues", $"leagues/{League}", "constructed", "decks", "status", $"status#event={_live}", "no-such-page"];
        foreach ((int width, int height, string size) in Sizes)
        {
            foreach (bool signedIn in new[] { false, true })
            {
                IPage page = await OpenAsync(width, height, signedIn);
                foreach (string path in pages.Where(path => !(signedIn && path.Length == 0)))
                {
                    await page.GotoAsync(new Uri(_address, path).AbsoluteUri);
                    await page.WaitForLoadStateAsync(LoadState.NetworkIdle);
                    await Task.Delay(300);

                    // The logo is the bar's way home, and every section is in it; the page's own is lit.
                    ILocator logo = page.Locator(".site-header .site-logo img");
                    Assert.Equal("Cantrip", await logo.GetAttributeAsync("alt"));
                    Assert.True(await logo.IsVisibleAsync());
                    Assert.Equal(1, await page.Locator(".site-footer").CountAsync());
                    Assert.Equal(signedIn ? 1 : 0, await page.Locator(".site-header .site-me").CountAsync());
                    Assert.Equal(signedIn ? 0 : 1, await page.Locator(".site-header [data-site-signin]").CountAsync());

                    // Nothing wider than the window.
                    int scroll = await page.EvaluateAsync<int>("document.documentElement.scrollWidth");
                    Assert.True(scroll <= width, $"{path} at {size} is {scroll} wide");

                    string name = (path.Length == 0 ? "landing" : path.Replace("#event=", "-event-", StringComparison.Ordinal).Replace('/', '-')) + "-" + size + (signedIn ? "-in" : "-out");
                    await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, name + ".png"), FullPage = true });
                }

                if (width < 600)
                {
                    // On a phone the sections are behind the menu button.
                    await page.GotoAsync(new Uri(_address, "draft").AbsoluteUri);
                    Assert.False(await page.Locator(".site-nav a[href='decks']").IsVisibleAsync());
                    await page.Locator("[data-site-menu]").ClickAsync();
                    Assert.Equal("true", await page.Locator("[data-site-menu]").GetAttributeAsync("aria-expanded"));
                    await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, $"menu-{size}{(signedIn ? "-in" : "-out")}.png") });
                    await page.Locator(".site-nav a[href='decks']").ClickAsync();
                    await page.WaitForURLAsync("**/decks");
                }

                await page.Context.CloseAsync();
            }
        }

        Assert.Empty(_errors);
    }

    [Fact]
    public async Task A_player_finds_everything_from_the_menu_bar_and_drops_out_of_their_draft_from_their_page_after_saying_so()
    {
        IPage page = await OpenAsync(1440, 900, signedIn: true);

        // The front door goes to the player's own page: what they're playing, and what they've played.
        await page.GotoAsync(_address.AbsoluteUri);
        await page.WaitForURLAsync("**/home");
        await page.Locator("h1", new PageLocatorOptions { HasText = "Welcome, Alice" }).WaitForAsync();
        ILocator playing = page.Locator(".card", new PageLocatorOptions { HasText = "Winston draft: LSVCube" }).First;
        await playing.WaitForAsync();
        Assert.Equal(2, await page.Locator("#content .rows .row").CountAsync());
        Assert.True(await page.Locator(".row", new PageLocatorOptions { HasText = "Won" }).IsVisibleAsync());
        Assert.True(await page.Locator(".row", new PageLocatorOptions { HasText = "Called off" }).IsVisibleAsync());
        // Whom she played, by picture and name, a link to message them on Discord.
        await PlayerShownAsync(page.Locator(".row", new PageLocatorOptions { HasText = "Won" }), "Bob", 1002);

        // Every section is a click away, and comes back.
        foreach ((string link, string url, string heading) in new[] { ("Draft", "**/draft", "Draft"), ("Constructed", "**/constructed", "Constructed"), ("Decks", "**/decks", "Decks"), ("Events", "**/status", "Events"), ("Home", "**/home", "Welcome, Alice") })
        {
            await page.Locator(".site-nav a", new PageLocatorOptions { HasText = link }).ClickAsync();
            await page.WaitForURLAsync(url);
            await page.Locator("h1", new PageLocatorOptions { HasText = heading }).WaitForAsync();
        }

        // A draft being played shows on every other page, with the way back to the seat.
        await page.Locator(".site-nav a", new PageLocatorOptions { HasText = "Decks" }).ClickAsync();
        await page.Locator("#site-banner", new PageLocatorOptions { HasText = "Back to my draft" }).WaitForAsync();
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "banner-on-decks.png") });

        // The deck library opens a deck in the editor.
        Assert.Equal(3, await page.Locator("#content .row").CountAsync());
        string? edit = await page.Locator("a[aria-label='Edit Boros Burn']").GetAttributeAsync("href");
        Assert.StartsWith("decks/edit?deck=", edit, StringComparison.Ordinal);

        // Dropping is asked first: no keeps the seat.
        await page.Locator(".site-nav a", new PageLocatorOptions { HasText = "Home" }).ClickAsync();
        await playing.WaitForAsync();
        await playing.Locator("button", new() { HasText = "Drop" }).ClickAsync();
        ILocator dialog = page.Locator("dialog[open]");
        await dialog.WaitForAsync();
        Assert.Contains("Drop from Winston draft: LSVCube?", await dialog.Locator("h2").InnerTextAsync(), StringComparison.Ordinal);
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "drop-question.png") });
        await dialog.Locator("button", new() { HasText = "Cancel" }).ClickAsync();
        Assert.True(_drafts.InDraft(Alice));

        // Yes drops her: the draft is over, listed with the others, and she is free to play again.
        await playing.Locator("button", new() { HasText = "Drop" }).ClickAsync();
        await page.Locator("dialog[open] button", new() { HasText = "Drop" }).ClickAsync();
        await page.Locator(".site-toast", new() { HasText = "You dropped from Winston draft: LSVCube." }).WaitForAsync();
        await page.Locator("#content .rows .row").Nth(2).WaitForAsync();
        Assert.False(_drafts.InDraft(Alice));
        Assert.Equal(3, await page.Locator("#content .rows .row").CountAsync());
        Assert.True(await page.Locator(".empty", new() { HasText = "You're not in anything right now." }).IsVisibleAsync());
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "after-drop.png"), FullPage = true });

        // An event's results are a click away.
        await page.Locator("#content .rows .row a", new() { HasText = "Winston draft" }).First.ClickAsync();
        await page.WaitForURLAsync("**/status#event=*");
        await page.Locator("#drawer.on").WaitForAsync();
        await PlayerShownAsync(page.Locator("#drawer"), "Carol", 1003, picture: false);
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "results.png") });

        // Signing out goes back to the front door, signed out.
        await page.GotoAsync(new Uri(_address, "draft").AbsoluteUri);
        await page.Context.SetExtraHTTPHeadersAsync([]);
        await page.Locator("[data-site-signout]").ClickAsync();
        await page.WaitForURLAsync(_address.AbsoluteUri);
        await page.Locator(".hero h1").WaitForAsync();
        Assert.Empty(_errors);
    }

    [Fact]
    public async Task A_player_sees_their_league_run_and_its_deck_looks_for_a_match_and_is_asked_before_ending_the_run()
    {
        IPage page = await OpenAsync(1440, 900, signedIn: true);

        // Her page lists the league, with the run she is on.
        await page.GotoAsync(new Uri(_address, "home").AbsoluteUri);
        ILocator card = page.Locator("#content .card", new PageLocatorOptions { HasText = "Vintage Cube Quick Draft" });
        await card.WaitForAsync();
        Assert.Contains("3-1", await card.InnerTextAsync(), StringComparison.Ordinal);

        // The leagues page, then the league's: her record, the players, the trophies, her previous decks.
        await page.Locator(".site-nav a", new PageLocatorOptions { HasText = "Leagues" }).ClickAsync();
        await page.WaitForURLAsync("**/leagues");
        await page.Locator(".league-card .standing", new PageLocatorOptions { HasText = "3-1" }).WaitForAsync();
        await page.Locator(".league-card a", new PageLocatorOptions { HasText = "Open my run" }).ClickAsync();
        await page.WaitForURLAsync($"**/leagues/{League}");
        await page.Locator(".run-record", new PageLocatorOptions { HasText = "3-1" }).WaitForAsync();
        Assert.Equal(3, await page.Locator(".pip.win").CountAsync());
        Assert.Equal(1, await page.Locator(".pip.loss").CountAsync());
        Assert.True(await page.Locator("table.data tr.me", new PageLocatorOptions { HasText = "Alice" }).First.IsVisibleAsync());
        Assert.True(await page.Locator("td.trophy-count").First.IsVisibleAsync());
        // Each player by their picture and name, a link to message them on Discord.
        await PlayerShownAsync(page.Locator("table.data").First, "Bob", 1002);
        Assert.Equal(1, await page.Locator("#content .rows .row", new PageLocatorOptions { HasText = "Run #1" }).CountAsync());
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "league-run.png"), FullPage = true });

        // Her deck, as a list; one of a run over too.
        await page.Locator("button", new PageLocatorOptions { HasText = "View or change deck" }).ClickAsync();
        ILocator deck = page.Locator("dialog#deck[open]");
        await deck.WaitForAsync();
        Assert.Contains("40 cards", await deck.Locator("h2").InnerTextAsync(), StringComparison.Ordinal);
        Assert.True(await deck.Locator("[data-deck-edit]").IsVisibleAsync());
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "league-deck.png") });
        await deck.Locator("button", new() { HasText = "Close" }).ClickAsync();
        await page.Locator("#content .rows .row button", new() { HasText = "View deck" }).ClickAsync();
        await deck.WaitForAsync();
        Assert.False(await deck.Locator("[data-deck-edit]").IsVisibleAsync());
        await deck.Locator("button", new() { HasText = "Close" }).ClickAsync();

        // Looking for a match, and stopping.
        await page.Locator("button", new() { HasText = "Find a match" }).ClickAsync();
        await page.Locator(".searching", new() { HasText = "Looking for an opponent" }).WaitForAsync();
        Assert.True(_leagues.Find(League)!.League.IsSearching($"discord-{Alice}"));
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "league-searching.png") });
        await page.Locator("button", new() { HasText = "Stop looking" }).ClickAsync();
        await page.Locator("button", new() { HasText = "Find a match" }).WaitForAsync();

        // Ending the run is asked first: no keeps it.
        await page.Locator("button", new() { HasText = "End run" }).ClickAsync();
        ILocator ask = page.Locator("dialog[open]", new() { HasText = "End your run?" });
        await ask.WaitForAsync();
        Assert.Contains("3 losses", await ask.InnerTextAsync(), StringComparison.Ordinal);
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "league-end-question.png") });
        await ask.Locator("button", new() { HasText = "Cancel" }).ClickAsync();
        Assert.NotNull(_leagues.Find(League)!.League.View($"discord-{Alice}").Mine!.Current);

        // Yes ends it, as three losses, and it joins her previous decks.
        await page.Locator("button", new() { HasText = "End run" }).ClickAsync();
        await page.Locator("dialog[open] button", new() { HasText = "End my run" }).ClickAsync();
        await page.Locator("button", new() { HasText = "Start a draft" }).WaitForAsync();
        Assert.Equal(1, await page.Locator("#content .rows .row", new PageLocatorOptions { HasText = "3-3" }).CountAsync());
        await page.ScreenshotAsync(new PageScreenshotOptions { Path = Path.Combine(_pictures, "league-after-end.png"), FullPage = true });
        Assert.Empty(_errors);
    }

    private async Task<IPage> OpenAsync(int width, int height, bool signedIn)
    {
        IBrowserContext context = await web.Browser.NewContextAsync(new BrowserNewContextOptions
        {
            ViewportSize = new ViewportSize { Width = width, Height = height },
            ExtraHTTPHeaders = signedIn ? [new(AsHeader, $"{Alice}:Alice")] : [],
        });
        IPage page = await context.NewPageAsync();
        page.Console += (_, message) =>
        {
            if (message.Type == "error" && !message.Text.Contains("404", StringComparison.Ordinal)) _errors.Add($"{page.Url}: {message.Text}");
        };
        page.PageError += (_, error) => _errors.Add($"{page.Url}: {error}");
        return page;
    }

    /// <summary>
    /// A player shown in <paramref name="within"/>: their picture, loaded - or their initial, for one with none - and their
    /// name, a link to message them on Discord.
    /// </summary>
    private static async Task PlayerShownAsync(ILocator within, string name, ulong discord, bool picture = true)
    {
        ILocator player = within.Locator("a.who", new LocatorLocatorOptions { HasText = name }).First;
        await player.WaitForAsync();
        Assert.Equal($"https://discord.com/users/{discord}", await player.GetAttributeAsync("href"));
        Assert.Equal("_blank", await player.GetAttributeAsync("target"));
        if (!picture)
        {
            Assert.Equal(name[..1], await player.Locator(".face").InnerTextAsync());
            return;
        }

        await player.Locator("img").WaitForAsync();
        Assert.True(await player.Locator("img").EvaluateAsync<bool>("img => img.complete && img.naturalWidth > 0"));
    }

    /// <summary>A Winston draft Alice won against Bob, as the data directory keeps one over.</summary>
    private static string Finished(EventStore store, IReadOnlyList<string> cube)
    {
        CreatedEvent created = DraftPod.CreateWinston(store, Cantrip.Server.ContentCache.Get(WebClient.Content), new WinstonDraftSetup
        {
            Name = "Winston draft: LSVCube", Cube = "LSVCube", CubeCards = cube, Cards = 18, MinimumDeck = 10, Seed = 3,
            Players = [($"discord-{Alice}", "Alice"), ("discord-1002", "Bob")],
        });
        EventRecord record = store.LoadEvent(created.Record.Id)!;
        DateTimeOffset now = DateTimeOffset.UtcNow;
        record.Stage = EventStage.Complete;
        record.Round = 1;
        record.Detail = "Alice won round 1.";
        record.Matches.Add(new MatchRecord
        {
            Id = "r1", Round = 1, Players = [$"discord-{Alice}", "discord-1002"], Winners = [$"discord-{Alice}"], Complete = true,
            Games = [new GameRecord { Number = 1, Started = now.AddMinutes(-20), Ended = now.AddMinutes(-2), Ending = "won", Winners = [$"discord-{Alice}"], Detail = "Bob lost." }],
        });
        store.SaveEvent(record);
        return record.Id;
    }

    /// <summary>
    /// A league as the data directory keeps one partway through: Alice on her second run at 3-1 after a trophy, Bob and
    /// Carol with runs going, Carol with two trophies, and Erin knocked out.
    /// </summary>
    private static void SeedLeague(EventStore store, IReadOnlyList<string> cube)
    {
        DateTimeOffset now = DateTimeOffset.UtcNow;
        List<string> pool = [.. cube.Distinct(StringComparer.Ordinal).Where(name => name is not ("Mountain" or "Plains")).Take(23)];
        EventDeck Built() => new() { Main = [.. Enumerable.Range(0, pool.Count)], Basics = new() { ["Mountain"] = 9, ["Plains"] = 40 - pool.Count - 9 }, Submitted = true };
        (string Id, string Name)[] players = [($"discord-{Alice}", "Alice"), ("discord-1002", "Bob"), ("discord-1003", "Carol"), ("discord-1005", "Erin")];
        LeagueRun Run(int number, int player, int entry, LeagueRunStage stage, string ending, int daysAgo) => new()
        {
            Id = $"run-{number}", PlayerId = players[player].Id, Name = players[player].Name, Entry = entry, DraftId = $"draft-{number}", Stage = stage,
            Pool = pool, Deck = Built(), Started = now.AddDays(-daysAgo - 1), Ended = stage == LeagueRunStage.Over ? now.AddDays(-daysAgo) : null, Ending = ending,
        };

        List<LeagueRun> runs =
        [
            Run(1, 0, 1, LeagueRunStage.Over, LeagueEndings.Trophy, 2),
            Run(2, 2, 1, LeagueRunStage.Over, LeagueEndings.Trophy, 3),
            Run(3, 2, 2, LeagueRunStage.Over, LeagueEndings.Trophy, 1),
            Run(4, 3, 1, LeagueRunStage.Over, LeagueEndings.Eliminated, 1),
            Run(5, 0, 2, LeagueRunStage.Playing, string.Empty, 0),
            Run(6, 1, 1, LeagueRunStage.Playing, string.Empty, 0),
            Run(7, 2, 3, LeagueRunStage.Playing, string.Empty, 0),
        ];
        var record = new EventRecord
        {
            Id = League, Kind = EventKinds.League, Name = "Vintage Cube Quick Draft", Stage = EventStage.Playing, Created = now.AddDays(-5), Updated = now, Seed = 9,
            League = new LeagueRecord { Settings = new LeagueSettings { Cube = "rw-aggro" }, Runs = runs },
            Participants = [.. players.Select((player, seat) => new EventParticipant { PlayerId = player.Id, Name = player.Name, Seat = seat + 1, Seated = true })],
        };

        // Each run's matches, won or lost, against one of the runs being played: what its record is counted from.
        void Results(LeagueRun run, int wins, int losses, LeagueRun against)
        {
            for (int i = 0; i < wins + losses; i++)
            {
                int number = record.Matches.Count + 1;
                string winner = i < wins ? run.PlayerId : against.PlayerId;
                record.Matches.Add(new MatchRecord
                {
                    Id = $"m{number}", Round = number, Players = [run.PlayerId, against.PlayerId], Entries = [run.Id, "run-0"], Winners = [winner], Complete = true,
                    Detail = $"{(winner == run.PlayerId ? run.Name : against.Name)} won 1-0",
                    Games = [new GameRecord { Number = 1, Started = now.AddMinutes(-number * 30), Ended = now.AddMinutes(-number * 30 + 20), Ending = "won", Winners = [winner] }],
                });
            }
        }

        Results(runs[0], 7, 1, runs[5]);
        Results(runs[1], 7, 2, runs[4]);
        Results(runs[2], 7, 0, runs[5]);
        Results(runs[3], 1, 3, runs[6]);
        Results(runs[4], 3, 1, runs[5]);
        Results(runs[5], 2, 2, runs[6]);
        Results(runs[6], 1, 0, runs[4]);
        store.SaveEvent(record);
        foreach ((string id, string name) in players) store.Enlist(id, name, record, now.AddDays(-5));
    }

    private static Task SignInByHeader(HttpContext context, Func<Task> next)
    {
        if (context.Request.Headers.TryGetValue(AsHeader, out var who) && who.ToString().Split(':', 2) is [{ } id, { } name])
        {
            context.User = new ClaimsPrincipal(new ClaimsIdentity([new Claim(ClaimTypes.NameIdentifier, id), new Claim(ClaimTypes.Name, name)], "test"));
        }

        return next();
    }

    private static List<int> FreePorts(int count)
    {
        List<TcpListener> listeners = [.. Enumerable.Range(0, count).Select(_ => new TcpListener(IPAddress.Loopback, 0))];
        listeners.ForEach(listener => listener.Start());
        List<int> ports = [.. listeners.Select(listener => ((IPEndPoint)listener.LocalEndpoint).Port)];
        listeners.ForEach(listener => listener.Stop());
        return ports;
    }
}
