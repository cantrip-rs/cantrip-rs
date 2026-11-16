using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using Cantrip.Art;
using Cantrip.Decks;
using Cantrip.Protocol;
using Cantrip.Server;
using Cantrip.Server.Events;
using Cantrip.TestKit;
using Xunit;

namespace Cantrip.Discord.Tests;

/// <summary>
/// The draft queue and constructed play pages as the bot serves them, over real HTTP with real tables behind them: what
/// anyone may see of the queues, the tables forming and the games, that taking a seat wants a Discord sign-in, and that
/// signing in goes to Discord asking only who the player is, to come back to the address registered for it.
/// </summary>
[Trait("Category", "WallClock")]
public sealed class SiteTests : IDisposable
{
    private const ulong Channel = 4242;
    private const string AsHeader = "X-Test-Account";
    private readonly string _data = Path.Combine(Path.GetTempPath(), "cantrip-site-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_data)) Directory.Delete(_data, recursive: true);
    }

    [Fact]
    public async Task The_queue_page_shows_every_queue_and_table_by_name_and_a_seat_wants_signing_in()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);
        Assert.Null(await fixture.Queues.ConfigureAsync(new QueueSettings { Channel = Channel, Cube = "rw-aggro", Format = "grid", Packs = 4, MinimumDeck = 10 }));
        QueueSeating alice = await fixture.Queues.CreateTableAsync(new QueueSettings { Cube = "rw-aggro", Cards = 18, MinimumDeck = 20 }, new DiscordPlayer(1001, "Alice"));
        Assert.Null(alice.Refusal);

        HttpResponseMessage page = await fixture.Http.GetAsync(fixture.Address("draft"));
        Assert.Equal(HttpStatusCode.OK, page.StatusCode);
        Assert.Contains("Draft Queues", await page.Content.ReadAsStringAsync(), StringComparison.Ordinal);

        string text = await fixture.Http.GetStringAsync(fixture.Address("draft/api/board"));
        using JsonDocument board = JsonDocument.Parse(text);
        JsonElement value = board.RootElement.GetProperty("value");
        Assert.False(value.GetProperty("signIn").GetBoolean());
        Assert.Equal(JsonValueKind.Null, value.GetProperty("me").ValueKind);
        Assert.Contains("rw-aggro", value.GetProperty("cubes").EnumerateArray().Select(cube => cube.GetString()));

        // The channel's id as text, past what a number in JavaScript holds; the table of Alice's own in no channel.
        JsonElement queue = Assert.Single(value.GetProperty("queues").EnumerateArray());
        Assert.Equal(Channel.ToString(System.Globalization.CultureInfo.InvariantCulture), queue.GetProperty("id").GetString());
        Assert.Equal("grid", queue.GetProperty("settings").GetProperty("format").GetString());
        JsonElement table = Assert.Single(value.GetProperty("tables").EnumerateArray());
        Assert.Equal(JsonValueKind.Null, table.GetProperty("queue").ValueKind);
        Assert.Equal(["Alice"], table.GetProperty("players").EnumerateArray().Select(player => player.GetProperty("name").GetString()));
        Assert.Equal(2, table.GetProperty("seats").GetInt32());

        // Each player by name and Discord account, which the page links to a message to them by: no player id, and no ticket.
        Assert.Equal("1001", table.GetProperty("players")[0].GetProperty("discord").GetString());
        Assert.DoesNotContain("discord-", text, StringComparison.Ordinal);
        Assert.DoesNotContain(alice.Ticket!, text, StringComparison.Ordinal);

        // Nobody signed in: no seat, at the table or from the queue, and no table of one's own.
        foreach ((string path, string body) in new[]
        {
            ("draft/api/join", $$"""{"table":"{{table.GetProperty("id").GetString()}}"}"""),
            ("draft/api/join", $$"""{"queue":"{{Channel}}"}"""),
            ("draft/api/create", """{"format":"winston","cube":"rw-aggro","cards":18}"""),
            ("draft/api/return", "{}"),
        })
        {
            HttpResponseMessage answer = await fixture.Http.PostAsync(fixture.Address(path), new StringContent(body, Encoding.UTF8, "application/json"));
            Assert.Equal(HttpStatusCode.Unauthorized, answer.StatusCode);
        }

        // Without sign-in set up, Sign in only comes back to the page.
        HttpResponseMessage signIn = await fixture.Http.GetAsync(fixture.Address("signin?then=draft"));
        Assert.Equal(HttpStatusCode.Redirect, signIn.StatusCode);
        Assert.Equal("/draft", signIn.Headers.Location!.OriginalString);

        // And signing in to nowhere in particular comes back to the player's own page.
        HttpResponseMessage anywhere = await fixture.Http.GetAsync(fixture.Address("signin"));
        Assert.Equal("/home", anywhere.Headers.Location!.OriginalString);
    }

    [Fact]
    public async Task The_play_page_lists_each_formats_decks_and_its_games_by_their_players_names_alone()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);
        Seating hosted = await fixture.Games.HostAsync(new DiscordPlayer(1001, "Alice"), "limited", "ub-reanimator");
        Assert.Null(hosted.Refusal);
        await fixture.Games.HostAsync(new DiscordPlayer(1002, "Bob"));

        HttpResponseMessage page = await fixture.Http.GetAsync(fixture.Address("constructed"));
        Assert.Equal(HttpStatusCode.OK, page.StatusCode);
        Assert.Contains("Constructed", await page.Content.ReadAsStringAsync(), StringComparison.Ordinal);

        string text = await fixture.Http.GetStringAsync(fixture.Address("constructed/api/board"));
        using JsonDocument board = JsonDocument.Parse(text);
        JsonElement value = board.RootElement.GetProperty("value");
        JsonElement format = Assert.Single(value.GetProperty("formats").EnumerateArray());
        Assert.Equal("limited", format.GetProperty("key").GetString());
        Assert.Equal(["rw-aggro", "ub-reanimator"], format.GetProperty("decks").EnumerateArray().Select(deck => deck.GetProperty("name").GetString()).Order());

        // Alice's game, waiting, with her name and not her deck; Bob's game from Discord is not the page's.
        JsonElement game = Assert.Single(value.GetProperty("games").EnumerateArray());
        Assert.Equal((hosted.Game!.Id, "Open"), (game.GetProperty("id").GetString(), game.GetProperty("stage").GetString()));
        Assert.Equal(["Alice"], game.GetProperty("players").EnumerateArray().Select(player => player.GetProperty("name").GetString()));
        Assert.False(game.TryGetProperty("deck", out _));
        Assert.DoesNotContain(hosted.Ticket!, text, StringComparison.Ordinal);

        foreach ((string path, string body) in new[]
        {
            ("constructed/api/host", """{"format":"limited","deck":"rw-aggro"}"""),
            ("constructed/api/join", $$"""{"game":"{{hosted.Game.Id}}","deck":"rw-aggro"}"""),
            ("constructed/api/cancel", $$"""{"game":"{{hosted.Game.Id}}"}"""),
        })
        {
            HttpResponseMessage answer = await fixture.Http.PostAsync(fixture.Address(path), new StringContent(body, Encoding.UTF8, "application/json"));
            Assert.Equal(HttpStatusCode.Unauthorized, answer.StatusCode);
        }
    }

    [Fact]
    public async Task The_bare_address_is_the_home_page_and_an_address_with_a_table_in_its_query_is_the_client()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);

        string landing = await fixture.Http.GetStringAsync(fixture.Address(string.Empty));
        Assert.Contains("https://github.com/cantrip-rs/", landing, StringComparison.Ordinal);
        Assert.Contains("https://discord.gg/x24qcVeT2w", landing, StringComparison.Ordinal);
        foreach (string link in new[] { "signin?then=home", "href=\"draft\"", "href=\"constructed\"" })
            Assert.Contains(link, landing, StringComparison.Ordinal);

        // The logo is on it, and served beside it; and it says what Cantrip is for.
        Assert.Contains("class=\"hero-logo\" src=\"logo.png\"", landing, StringComparison.Ordinal);
        Assert.Contains("rules engine fast enough to train modern AI on", landing, StringComparison.Ordinal);
        Assert.Contains("A fast, accurate game of Magic in your browser comes along as a bonus", landing, StringComparison.Ordinal);
        Assert.Equal(landing, await fixture.Http.GetStringAsync(fixture.Address("about")));
        HttpResponseMessage logo = await fixture.Http.GetAsync(fixture.Address("logo.png"));
        Assert.Equal(HttpStatusCode.OK, logo.StatusCode);
        Assert.Equal("image/png", logo.Content.Headers.ContentType?.MediaType);
        Assert.Equal(new byte[] { 0x89, (byte)'P', (byte)'N', (byte)'G' }, (await logo.Content.ReadAsByteArrayAsync())[..4]);

        // A seat's link: the client, with the table in the query.
        string client = await fixture.Http.GetStringAsync(fixture.Address("?server=ws%3A%2F%2Flocalhost%3A7782%2Fgame"));
        Assert.DoesNotContain("github.com", client, StringComparison.Ordinal);
        Assert.Contains("<title>Cantrip</title>", client, StringComparison.Ordinal);

        // The pages' earlier names still lead to them.
        foreach ((string old, string now) in new[] { ("queue", "/draft"), ("play", "/constructed") })
        {
            HttpResponseMessage moved = await fixture.Http.GetAsync(fixture.Address(old));
            Assert.Equal(HttpStatusCode.MovedPermanently, moved.StatusCode);
            Assert.Equal(now, moved.Headers.Location!.OriginalString);
        }
    }

    [Fact]
    public async Task The_players_page_shows_nothing_to_nobody_and_the_table_and_game_a_player_is_in_to_them()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);
        HttpResponseMessage page = await fixture.Http.GetAsync(fixture.Address("home"));
        Assert.Equal(HttpStatusCode.OK, page.StatusCode);
        Assert.Contains("<title>Cantrip - Home</title>", await page.Content.ReadAsStringAsync(), StringComparison.Ordinal);

        using JsonDocument board = JsonDocument.Parse(await fixture.Http.GetStringAsync(fixture.Address("home/api/board")));
        JsonElement value = board.RootElement.GetProperty("value");
        Assert.Equal(JsonValueKind.Null, value.GetProperty("me").ValueKind);
        Assert.Equal(JsonValueKind.Null, value.GetProperty("draft").ValueKind);
        Assert.Equal(JsonValueKind.Null, value.GetProperty("constructed").ValueKind);

        // Alice, signed in, at a table that is filling and in a game of her own; Bob, in neither.
        QueueSeating table = await fixture.Queues.CreateTableAsync(new QueueSettings { Cube = "rw-aggro", Cards = 18, MinimumDeck = 20 }, new DiscordPlayer(1001, "Alice"));
        Assert.Null(table.Refusal);
        Seating game = await fixture.Games.HostAsync(new DiscordPlayer(1001, "Alice"), "limited", "ub-reanimator");
        Assert.Null(game.Refusal);

        string alice = JsonSerializer.Serialize(HomeSite.Board(fixture.Home, new WebAccount(1001, "Alice", null)), new JsonSerializerOptions(JsonSerializerDefaults.Web));
        using JsonDocument aliceBoard = JsonDocument.Parse(alice);
        JsonElement mine = aliceBoard.RootElement;
        Assert.Equal("Alice", mine.GetProperty("me").GetProperty("name").GetString());
        Assert.Equal(["Alice"], mine.GetProperty("draft").GetProperty("players").EnumerateArray().Select(player => player.GetProperty("name").GetString()));
        Assert.Equal(game.Game!.Id, mine.GetProperty("constructed").GetProperty("id").GetString());
        Assert.DoesNotContain(table.Ticket!, alice, StringComparison.Ordinal);
        Assert.DoesNotContain(game.Ticket!, alice, StringComparison.Ordinal);

        using JsonDocument bob = JsonDocument.Parse(JsonSerializer.Serialize(HomeSite.Board(fixture.Home, new WebAccount(1002, "Bob", null))));
        Assert.Equal(JsonValueKind.Null, bob.RootElement.GetProperty("draft").ValueKind);
        Assert.Equal(JsonValueKind.Null, bob.RootElement.GetProperty("constructed").ValueKind);
    }

    [Fact]
    public async Task The_deck_editor_has_every_card_with_its_legalities_and_a_players_decks_want_signing_in()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);

        // The library is the site's page; the editor is the web client, opened on a deck or a new one in a format.
        HttpResponseMessage library = await fixture.Http.GetAsync(fixture.Address("decks"));
        Assert.Equal(HttpStatusCode.OK, library.StatusCode);
        Assert.Contains("<title>Cantrip - Decks</title>", await library.Content.ReadAsStringAsync(), StringComparison.Ordinal);
        foreach ((string path, string client) in new[] { ("decks/edit", "/?decks"), ("decks/edit?deck=deck-1", "/?decks&deck=deck-1"), ("decks/edit?format=modern", "/?decks&format=modern") })
        {
            HttpResponseMessage editor = await fixture.Http.GetAsync(fixture.Address(path));
            Assert.Equal(HttpStatusCode.Redirect, editor.StatusCode);
            Assert.Equal(client, editor.Headers.Location!.OriginalString);
        }

        using (JsonDocument formats = JsonDocument.Parse(await fixture.Http.GetStringAsync(fixture.Address("decks/api/formats"))))
        {
            Assert.Equal(DeckFormat.All.Select(format => format.Key), formats.RootElement.GetProperty("value").EnumerateArray().Select(format => format.GetProperty("key").GetString()));
        }

        // The catalog, for anyone: every card as printed, with where it is legal and its color identity.
        HttpResponseMessage answer = await fixture.Http.GetAsync(fixture.Address("decks/api/catalog"));
        Assert.Equal(HttpStatusCode.OK, answer.StatusCode);
        Assert.Equal("application/x-msgpack", answer.Content.Headers.ContentType!.MediaType);
        CardCatalogView catalog = MessagePack.MessagePackSerializer.Deserialize<CardCatalogView>(await answer.Content.ReadAsByteArrayAsync());
        Assert.NotEmpty(catalog.LegalityDate);
        Assert.True(catalog.Cards.Count > 1000);
        CatalogCard aang = Assert.Single(catalog.Cards, card => card.Name == "Aang, at the Crossroads");
        Assert.Equal([ColorView.White, ColorView.Blue, ColorView.Green], aang.Identity.Order());
        Assert.Equal(Legality.Legal, aang.LegalityIn("commander"));
        Assert.NotNull(aang.Card.OtherFace);
        // Every format but Freeform, which holds no card to a list, has its legality data.
        Assert.Equal(DeckFormat.All.Where(format => !format.IsFreeform).Select(format => format.Key).Order(), aang.Legalities.Keys.Order());

        // A player's decks are theirs: signed out, none.
        foreach (string path in new[] { "decks/api/list", "decks/api/deck/deck-1" })
        {
            Assert.Equal(HttpStatusCode.Unauthorized, (await fixture.Http.GetAsync(fixture.Address(path))).StatusCode);
        }

        foreach ((string path, string body) in new[] { ("decks/api/save", """{"name":"Mine","format":"modern"}"""), ("decks/api/delete", """{"id":"deck-1"}""") })
        {
            HttpResponseMessage refused = await fixture.Http.PostAsync(fixture.Address(path), new StringContent(body, Encoding.UTF8, "application/json"));
            Assert.Equal(HttpStatusCode.Unauthorized, refused.StatusCode);
        }
    }

    [Fact]
    public async Task Every_page_has_the_same_menu_bar_with_the_logo_its_own_section_lit_and_the_same_footer()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);
        foreach ((string path, string? section) in new[]
        {
            ("", null), ("about", null), ("home", "home"), ("draft", "draft"), ("constructed", "constructed"), ("decks", "decks"), ("status", "status"),
        })
        {
            string page = await fixture.Http.GetStringAsync(fixture.Address(path));
            Assert.DoesNotContain("<!--site:", page, StringComparison.Ordinal);
            Assert.Contains("<base href=\"/\">", page, StringComparison.Ordinal);
            Assert.Contains("<link rel=\"stylesheet\" href=\"site.css\">", page, StringComparison.Ordinal);
            Assert.Contains("<script src=\"site.js\"></script>", page, StringComparison.Ordinal);

            // The logo, never the word, in the menu bar; every section, the page's own marked as the current one.
            Assert.Contains("<a class=\"site-logo\" href=\"./\" aria-label=\"Cantrip home\"><img src=\"logo.png\" alt=\"Cantrip\"", page, StringComparison.Ordinal);
            foreach (string each in new[] { "home", "draft", "constructed", "decks", "status" })
                Assert.Contains(each == section ? $"<a href=\"{each}\" aria-current=\"page\">" : $"<a href=\"{each}\">", page, StringComparison.Ordinal);
            Assert.Equal(section is null ? 0 : 1, page.Split("aria-current=\"page\"").Length - 1);

            Assert.Contains("class=\"site-footer\"", page, StringComparison.Ordinal);
            Assert.Contains("<a href=\"about\">About</a>", page, StringComparison.Ordinal);
            Assert.Contains("id=\"main\"", page, StringComparison.Ordinal);
        }

        // What every page shares is served beside them.
        foreach ((string path, string type) in new[] { ("site.css", "text/css"), ("site.js", "text/javascript") })
        {
            HttpResponseMessage asset = await fixture.Http.GetAsync(fixture.Address(path));
            Assert.Equal(HttpStatusCode.OK, asset.StatusCode);
            Assert.Equal(type, asset.Content.Headers.ContentType?.MediaType);
        }

        // Nobody signed in, and nobody able to: no account in the bar, and no sign-in to offer.
        string home = await fixture.Http.GetStringAsync(fixture.Address("home"));
        string account = home.Split("<div class=\"site-account\">")[1].Split("</div>")[0];
        Assert.DoesNotContain("data-site-signout", account, StringComparison.Ordinal);
        Assert.DoesNotContain("data-site-signin", account, StringComparison.Ordinal);

        // A page that is not there says so in the site's own page, with the way on; an API that is not there is just not there.
        HttpResponseMessage missing = await fixture.Http.GetAsync(fixture.Address("no/such/page"));
        Assert.Equal(HttpStatusCode.NotFound, missing.StatusCode);
        string nowhere = await missing.Content.ReadAsStringAsync();
        Assert.Contains("There's no page here", nowhere, StringComparison.Ordinal);
        Assert.Contains("class=\"site-header\"", nowhere, StringComparison.Ordinal);
        HttpResponseMessage api = await fixture.Http.GetAsync(fixture.Address("home/api/nothing"));
        Assert.Equal(HttpStatusCode.NotFound, api.StatusCode);
        Assert.Empty(await api.Content.ReadAsStringAsync());
    }

    [Fact]
    public async Task A_signed_in_player_goes_from_the_front_door_to_their_own_page_which_names_them_in_the_menu_bar()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);
        HttpResponseMessage door = await fixture.AsAsync(1001, "Alice", HttpMethod.Get, string.Empty);
        Assert.Equal(HttpStatusCode.Redirect, door.StatusCode);
        Assert.Equal("/home", door.Headers.Location!.OriginalString);

        // The about page is still theirs to read; every page names them, with a way to sign out.
        string about = await fixture.AsStringAsync(1001, "Alice", "about");
        Assert.Contains("rules engine fast enough to train modern AI on", about, StringComparison.Ordinal);
        foreach (string path in new[] { "home", "draft", "decks", "status" })
        {
            string page = await fixture.AsStringAsync(1001, "Alice", path);
            Assert.Contains("<span class=\"site-name\">Alice</span>", page, StringComparison.Ordinal);
            Assert.Contains("data-site-signout", page, StringComparison.Ordinal);
        }

        // A name is text, never markup.
        string mallory = await fixture.AsStringAsync(1009, "<b>Mallory</b>", "home");
        Assert.Contains("&lt;b&gt;Mallory&lt;/b&gt;", mallory, StringComparison.Ordinal);
        Assert.DoesNotContain("<b>Mallory</b>", mallory, StringComparison.Ordinal);

        using JsonDocument board = JsonDocument.Parse(await fixture.AsStringAsync(1001, "Alice", "home/api/board"));
        JsonElement value = board.RootElement.GetProperty("value");
        Assert.Equal("Alice", value.GetProperty("me").GetProperty("name").GetString());
        Assert.Empty(value.GetProperty("events").EnumerateArray());
        Assert.Equal(0, value.GetProperty("decks").GetInt32());

        // What every page asks, for the seat a player has somewhere, leaves out what they have played and kept.
        using JsonDocument brief = JsonDocument.Parse(await fixture.AsStringAsync(1001, "Alice", "home/api/board?brief=1"));
        Assert.Equal(JsonValueKind.Null, brief.RootElement.GetProperty("value").GetProperty("decks").ValueKind);
    }

    [Fact]
    public async Task A_player_drops_out_of_their_draft_from_their_page_and_it_is_listed_as_over_with_how_it_ended()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);
        var alice = new DiscordPlayer(1001, "Alice");
        var bob = new DiscordPlayer(1002, "Bob");
        DraftSeating opened = await fixture.Drafts.OpenAsync(alice);
        Assert.Null((await fixture.Drafts.JoinAsync(opened.Draft!.Id, bob)).Refusal);
        string id = opened.Draft.Id;

        // Being played: on Bob's page, with a way back to his seat and a way out of it.
        using (JsonDocument before = JsonDocument.Parse(await fixture.AsStringAsync(1002, "Bob", "home/api/board")))
        {
            JsonElement value = before.RootElement.GetProperty("value");
            Assert.True(value.GetProperty("draft").GetProperty("drafting").GetBoolean());
            Assert.Equal(id, value.GetProperty("draft").GetProperty("event").GetString());
            JsonElement live = Assert.Single(value.GetProperty("events").EnumerateArray());
            Assert.Equal((id, true, true, "winston"), (live.GetProperty("id").GetString(), live.GetProperty("live").GetBoolean(), live.GetProperty("canDrop").GetBoolean(), live.GetProperty("format").GetString()));
        }

        // Signed out, nobody drops anyone; signed in, not out of someone else's event.
        HttpResponseMessage anonymous = await fixture.Http.PostAsync(fixture.Address("home/api/drop"), new StringContent($$"""{"event":"{{id}}"}""", Encoding.UTF8, "application/json"));
        Assert.Equal(HttpStatusCode.Unauthorized, anonymous.StatusCode);
        Assert.Contains("not in that draft", await Refusal(await fixture.AsAsync(1003, "Carol", HttpMethod.Post, "home/api/drop", $$"""{"event":"{{id}}"}""")), StringComparison.Ordinal);

        HttpResponseMessage dropped = await fixture.AsAsync(1002, "Bob", HttpMethod.Post, "home/api/drop", $$"""{"event":"{{id}}"}""");
        Assert.Null(await Refusal(dropped));
        Assert.False(fixture.Drafts.InDraft(bob.Id));
        Assert.Contains("not being played", await Refusal(await fixture.AsAsync(1002, "Bob", HttpMethod.Post, "home/api/drop", $$"""{"event":"{{id}}"}""")), StringComparison.Ordinal);

        // Over for both, each told how it ended for them, and neither in a draft.
        foreach ((ulong player, string name) in new[] { (1002UL, "Bob"), (1001UL, "Alice") })
        {
            using JsonDocument after = JsonDocument.Parse(await fixture.AsStringAsync(player, name, "home/api/board"));
            JsonElement value = after.RootElement.GetProperty("value");
            Assert.Equal(JsonValueKind.Null, value.GetProperty("draft").ValueKind);
            JsonElement over = Assert.Single(value.GetProperty("events").EnumerateArray());
            Assert.False(over.GetProperty("live").GetBoolean());
            Assert.False(over.GetProperty("canDrop").GetBoolean());
            Assert.Equal(("Called off", player == bob.Id), (over.GetProperty("outcome").GetString(), over.GetProperty("dropped").GetBoolean()));
            Assert.Equal("Bob dropped from the draft.", over.GetProperty("detail").GetString());
        }
    }

    [Fact]
    public async Task A_player_leaves_a_table_still_filling_from_the_site()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: false);
        QueueSeating table = await fixture.Queues.CreateTableAsync(new QueueSettings { Cube = "rw-aggro", Cards = 18, MinimumDeck = 20 }, new DiscordPlayer(1001, "Alice"));
        Assert.Null(table.Refusal);
        Assert.NotNull(fixture.Queues.TableOf(1001));

        Assert.Equal(HttpStatusCode.Unauthorized, (await fixture.Http.PostAsync(fixture.Address("home/api/leave"), new StringContent("{}", Encoding.UTF8, "application/json"))).StatusCode);
        Assert.Contains("not at a table", await Refusal(await fixture.AsAsync(1002, "Bob", HttpMethod.Post, "home/api/leave", "{}")), StringComparison.Ordinal);
        Assert.Null(await Refusal(await fixture.AsAsync(1001, "Alice", HttpMethod.Post, "home/api/leave", "{}")));
        Assert.Null(fixture.Queues.TableOf(1001));
    }

    /// <summary>An API's refusal, or null for none, from a 200 answer.</summary>
    private static async Task<string?> Refusal(HttpResponseMessage answer)
    {
        Assert.Equal(HttpStatusCode.OK, answer.StatusCode);
        using JsonDocument body = JsonDocument.Parse(await answer.Content.ReadAsStringAsync());
        JsonElement refusal = body.RootElement.GetProperty("value").GetProperty("refusal");
        return refusal.ValueKind == JsonValueKind.Null ? null : refusal.GetString();
    }

    [Fact]
    public void A_players_decks_are_saved_legal_or_not_listed_by_format_opened_and_deleted_and_nobody_elses()
    {
        var decks = new DeckStore(new EventStore(_data));
        const ulong Alice = 1001, Bob = 1002;

        // Not legal for Modern - three cards - and kept anyway.
        DeckSaved first = decks.Save(Alice, new SavedDeck { Name = "  Bears  ", Format = "modern", Main = [new DeckEntry("Grizzly Bears", 3)] });
        Assert.Null(first.Refusal);
        SavedDeck kept = first.Deck!;
        Assert.Equal("Bears", kept.Name);
        Assert.NotEmpty(kept.Id);
        Assert.True(File.Exists(Path.Combine(_data, "decks", "discord-1001", kept.Id + ".json")));

        DeckSaved commander = decks.Save(Alice, new SavedDeck { Name = "Aang", Format = "commander", Commanders = [new DeckEntry("Aang, at the Crossroads", 1)], Main = [new DeckEntry("Forest", 99)] });
        Assert.Null(commander.Refusal);
        DeckSaved standard = decks.Save(Alice, new SavedDeck { Name = "Zebras", Format = "standard", Main = [new DeckEntry("Plains", 60)] });

        // By format in the order they are offered - Standard, Modern, Commander - and the cards counted.
        Assert.Equal([("Zebras", "standard", 60), ("Bears", "modern", 3), ("Aang", "commander", 100)],
            decks.List(Alice).Select(deck => (deck.Name, deck.Format, deck.Cards)));
        Assert.Empty(decks.List(Bob));

        // Opened as saved, and saved again under the same id.
        SavedDeck opened = decks.Load(Alice, kept.Id)!;
        Assert.Equal([new DeckEntry("Grizzly Bears", 3)], opened.Main);
        opened.Main.Add(new DeckEntry("Forest", 57));
        Assert.Equal(kept.Id, decks.Save(Alice, opened).Deck!.Id);
        Assert.Equal(60, decks.List(Alice).Single(deck => deck.Id == kept.Id).Cards);

        // Bob can neither open, overwrite nor delete Alice's deck; an id that is no deck's is nothing.
        Assert.Null(decks.Load(Bob, kept.Id));
        Assert.Equal("That deck is not one of yours.", decks.Save(Bob, opened).Refusal);
        Assert.False(decks.Delete(Bob, kept.Id));
        Assert.Null(decks.Load(Alice, "../players/discord-1001"));

        // Refused only for being malformed.
        Assert.Equal("\"pauper\" is not a format a deck is built for.", decks.Save(Alice, new SavedDeck { Name = "P", Format = "pauper" }).Refusal);
        Assert.Equal("A deck needs a name.", decks.Save(Alice, new SavedDeck { Name = " ", Format = "modern" }).Refusal);
        Assert.Equal("Forest: a count is from 1 to 999.", decks.Save(Alice, new SavedDeck { Name = "F", Format = "modern", Main = [new DeckEntry("Forest", 0)] }).Refusal);

        Assert.True(decks.Delete(Alice, kept.Id));
        Assert.Null(decks.Load(Alice, kept.Id));
        Assert.False(decks.Delete(Alice, kept.Id));
        Assert.Equal(2, decks.List(Alice).Count);
        Assert.Null(standard.Refusal);
    }

    [Fact]
    public void A_deck_is_named_as_its_list_names_it_or_from_its_file()
    {
        string decks = Path.Combine(RepoRoot.Path, "decks");
        Assert.Equal("Mono-Green Landfall", PlaySite.Title(DeckList.Load(Path.Combine(decks, "standard", "mono-green-landfall.txt"))));
        Assert.Equal("Rw Aggro", PlaySite.Title(DeckList.Parse("rw-aggro", "4 Mountain", Path.Combine(decks, "no-such-file.txt"))));
    }

    [Fact]
    public async Task Signing_in_goes_to_Discord_for_the_accounts_identity_alone_and_back_to_the_registered_address()
    {
        await using Fixture fixture = await Fixture.StartAsync(_data, signIn: true);
        HttpResponseMessage answer = await fixture.Http.GetAsync(fixture.Address("signin?then=constructed"));
        Assert.Equal(HttpStatusCode.Redirect, answer.StatusCode);
        Uri discord = answer.Headers.Location!;
        Assert.Equal("https://discord.com/oauth2/authorize", discord.GetLeftPart(UriPartial.Path));

        Dictionary<string, string> query = discord.Query.TrimStart('?').Split('&')
            .Select(pair => pair.Split('=', 2)).ToDictionary(pair => pair[0], pair => Uri.UnescapeDataString(pair[1]));
        Assert.Equal("1234567890", query["client_id"]);
        Assert.Equal("identify", query["scope"]);
        Assert.Equal("code", query["response_type"]);
        Assert.Equal("S256", query["code_challenge_method"]);
        Assert.NotEmpty(query["state"]);

        // Back to the site's public address, whatever address the request came in at.
        Assert.Equal("https://cantrip.example.com/signin/discord", query["redirect_uri"]);
        Assert.Equal(new Uri("https://cantrip.example.com/signin/discord"), SiteSignIn.Callback(new Uri("https://cantrip.example.com/")));

        // The secret goes to Discord from the server alone, never through the browser.
        Assert.DoesNotContain("not-a-secret", discord.AbsoluteUri, StringComparison.Ordinal);

        // Still a 401 for a seat, rather than a redirect the page's fetch would follow.
        HttpResponseMessage seat = await fixture.Http.PostAsync(fixture.Address("draft/api/return"), new StringContent("{}", Encoding.UTF8, "application/json"));
        Assert.Equal(HttpStatusCode.Unauthorized, seat.StatusCode);
    }

    /// <summary>The bot's site with both pages on it, over queues and games with real tables, and a client that follows no redirect.</summary>
    private sealed class Fixture : IAsyncDisposable
    {
        private readonly string _root;
        private readonly int _port;
        private readonly Microsoft.AspNetCore.Builder.WebApplication _site;
        private readonly DraftHost _drafts;

        private Fixture(string root, int port, Microsoft.AspNetCore.Builder.WebApplication site, DraftQueues queues, DraftHost drafts, Matchmaker games, HomeSiteOptions home, EventStore store)
        {
            Home = home;
            Store = store;
            _root = root;
            _port = port;
            _site = site;
            Queues = queues;
            _drafts = drafts;
            Games = games;
        }

        public DraftQueues Queues { get; }

        public Matchmaker Games { get; }

        public HomeSiteOptions Home { get; }

        public EventStore Store { get; }

        public HttpClient Http { get; } = new(new HttpClientHandler { AllowAutoRedirect = false });

        public Uri Address(string path) => new($"http://127.0.0.1:{_port}/{path}");

        public DraftHost Drafts => _drafts;

        /// <summary>
        /// Discord's sign-in is Discord's, so here a player is a header on the request - "1001:Alice" - which this turns
        /// into the claims the sign-in cookie would carry.
        /// </summary>
        private static Task SignInByHeader(Microsoft.AspNetCore.Http.HttpContext context, Func<Task> next)
        {
            if (context.Request.Headers.TryGetValue(AsHeader, out var who) && who.ToString().Split(':', 2) is [{ } id, { } name])
            {
                context.User = new System.Security.Claims.ClaimsPrincipal(new System.Security.Claims.ClaimsIdentity(
                    [new(System.Security.Claims.ClaimTypes.NameIdentifier, id), new(System.Security.Claims.ClaimTypes.Name, name)], "test"));
            }

            return next();
        }

        /// <summary>A request made as a player signed in.</summary>
        public async Task<HttpResponseMessage> AsAsync(ulong id, string name, HttpMethod method, string path, string? body = null)
        {
            using var request = new HttpRequestMessage(method, Address(path));
            request.Headers.Add(AsHeader, $"{id}:{name}");
            if (body is not null) request.Content = new StringContent(body, Encoding.UTF8, "application/json");
            return await Http.SendAsync(request);
        }

        /// <summary>A page or an API's value, as a player signed in.</summary>
        public async Task<string> AsStringAsync(ulong id, string name, string path)
        {
            using HttpResponseMessage response = await AsAsync(id, name, HttpMethod.Get, path);
            Assert.Equal(HttpStatusCode.OK, response.StatusCode);
            return await response.Content.ReadAsStringAsync();
        }

        public static async Task<Fixture> StartAsync(string data, bool signIn)
        {
            List<int> ports = FreePorts(6);
            var store = new EventStore(data);
            var drafts = new DraftHost(new DraftHostOptions
            {
                ContentRoot = RepoRoot.ContentRoot, Store = store, Cube = "rw-aggro", CubeCards = CubeList.Load(Path.Combine(RepoRoot.Path, "decks", "rw-aggro.txt")), Cards = 18, Ports = ports[1..4],
            });
            var queues = new DraftQueues(new DraftQueuesOptions
            {
                ContentRoot = RepoRoot.ContentRoot, Store = store, Drafts = drafts, Ports = ports[1..4], CubeDirectories = [Path.Combine(RepoRoot.Path, "decks")],
            });
            string decks = Path.Combine(RepoRoot.Path, "decks");
            DeckList rwAggro = DeckList.Load(DeckList.Find(decks, "rw-aggro"));
            var games = new Matchmaker(new MatchmakerOptions
            {
                ContentRoot = RepoRoot.ContentRoot,
                Deck = rwAggro,
                Formats = new Dictionary<string, IReadOnlyList<DeckList>> { ["limited"] = [rwAggro, DeckList.Load(DeckList.Find(decks, "ub-reanimator"))] },
                Ports = ports[4..],
            });

            string root = Directory.CreateTempSubdirectory("cantrip-site-client").FullName;
            await File.WriteAllTextAsync(Path.Combine(root, "index.html"), "<!doctype html><title>Cantrip</title>");
            var art = new ArtStore(new ArtStoreOptions { ContentRoot = root, CacheDirectory = Path.Combine(root, "art") });
            var site = new Uri(signIn ? "https://cantrip.example.com/" : $"http://127.0.0.1:{ports[0]}/");
            var account = new SiteSignInOptions
            {
                Site = site,
                ClientId = signIn ? "1234567890" : null,
                ClientSecret = signIn ? "not-a-secret" : null,
                Keys = Path.Combine(data, "site-keys"),
            };
            var queuePage = new QueueSiteOptions
            {
                Queues = queues,
                Board = new EventStatusBoard(store, hosting: () => [.. drafts.Drafts.Select(draft => draft.Id)]),
                Drafts = drafts,
                Client = site,
                SignIn = account,
            };
            var playPage = new PlaySiteOptions { Games = games, Client = site, SignIn = account };
            var home = new HomeSiteOptions { SignIn = account, Drafts = queuePage, Constructed = playPage, HasDecks = true, Store = store };
            var deckSite = new DeckSiteOptions { ContentRoot = RepoRoot.ContentRoot, Store = store, SignIn = account };
            Microsoft.AspNetCore.Builder.WebApplication web = await ClientSite.StartAsync("127.0.0.1", ports[0], root, art, status: queuePage.Board, signIn: account, queues: queuePage, play: playPage, home: home, decks: deckSite, before: SignInByHeader);
            return new Fixture(root, ports[0], web, queues, drafts, games, home, store);
        }

        public async ValueTask DisposeAsync()
        {
            Http.Dispose();
            await _site.DisposeAsync();
            await Queues.DisposeAsync();
            await _drafts.DisposeAsync();
            await Games.DisposeAsync();
            Directory.Delete(_root, recursive: true);
        }
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
