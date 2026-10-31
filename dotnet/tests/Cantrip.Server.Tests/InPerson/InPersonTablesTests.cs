using System.Net;
using System.Net.Http.Json;
using System.Net.Sockets;
using System.Text.Json;
using Cantrip.Client.InPerson;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.Events;
using Cantrip.Server.InPerson;
using Cantrip.TestKit;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.Logging;
using Xunit;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>
/// In-person tables hosted on a public site (<see cref="InPersonTables"/>; docs/in-person.md, "Hosted in person"): anyone
/// may make one, so how many, for how long and from what are bounded, and a table's codes are shown only to whoever holds
/// its creator key. Each table is a real <c>GameServer</c> on a port of the pool, dealing nothing (no game is played here);
/// the host's clock is a test's.
/// </summary>
public sealed class InPersonTablesTests : IDisposable
{
    private readonly string _data = Path.Combine(Path.GetTempPath(), "cantrip-in-person-tables-" + Guid.NewGuid().ToString("N"));
    private readonly EventStore _store;
    private readonly Clock _clock = new();

    public InPersonTablesTests()
    {
        _store = new EventStore(Path.Combine(_data, "events"));
    }

    public void Dispose()
    {
        try
        {
            Directory.Delete(_data, recursive: true);
        }
        catch (IOException)
        {
        }
    }

    private sealed class Clock : TimeProvider
    {
        private DateTimeOffset _now = new(2026, 10, 30, 12, 0, 0, TimeSpan.Zero);
        public override DateTimeOffset GetUtcNow() => _now;
        public void Advance(TimeSpan duration) => _now += duration;
    }

    private static readonly Uri Site = new("https://cantrip.example.com/");

    private InPersonTables Tables(int mostGames = 2, int ports = 3) => new(new InPersonTablesOptions
    {
        ContentRoot = RepoRoot.ContentRoot,
        Store = _store,
        CubeDirectories = [Path.Combine(RepoRoot.Path, "docs", "cube-lists")],
        Ports = FreePorts(ports),
        Site = Site,
        Art = Site,
        TableAddress = port => new Uri($"wss://cantrip.example.com/table/{port}/game"),
        MostGames = mostGames,
        Rules = new CapturingRulesFactory(),
        TimeProvider = _clock,
    });

    private static SetupRequest Form(int layout = 3, string cube = "LSVCube", int packs = 2, params string[] names) =>
        new(names.Length == 0 ? ["Alice", "Bob"] : names, cube, packs, layout, false, null);

    private static List<JsonElement> Devices(object state) =>
        [.. JsonSerializer.SerializeToElement(state).GetProperty("devices").EnumerateArray()];

    [Fact]
    public async Task A_tables_links_reach_the_client_and_the_table_through_the_public_site()
    {
        await using InPersonTables tables = Tables();
        CreatedInPersonTable created = await tables.CreateAsync(Form(layout: 4));

        List<string> links = [.. Devices(tables.State(created.Id, created.Key)!).Select(device => device.GetProperty("url").GetString()!)];
        Assert.Equal(4, links.Count);
        int port = Assert.Single(Enumerable.Range(1, 65535), tables.Holds);
        string table = Uri.EscapeDataString($"wss://cantrip.example.com/table/{port}/game");
        Assert.StartsWith($"https://cantrip.example.com/?inperson=phone&server={table}#ticket=", links[0], StringComparison.Ordinal);
        Assert.StartsWith($"https://cantrip.example.com/?inperson=tablet&side=1&art={Uri.EscapeDataString(Site.AbsoluteUri)}#device=", links[2], StringComparison.Ordinal);
        Assert.Equal(new Uri("wss://cantrip.example.com/inperson/socket"), DeviceConnection.AddressFor(new Uri(links[2])));

        // The players are the table's own, never whoever their names would name in the data directory.
        EventRecord record = _store.LiveEvents().Single();
        Assert.All(record.Participants, participant => Assert.StartsWith("inperson-", participant.PlayerId, StringComparison.Ordinal));
        Assert.Equal("site", record.HostData[InPersonTables.HostedKey]);
    }

    [Fact]
    public async Task Only_the_creators_key_shows_a_tables_codes()
    {
        await using InPersonTables tables = Tables();
        CreatedInPersonTable created = await tables.CreateAsync(Form());

        Assert.NotNull(tables.State(created.Id, created.Key));
        Assert.Null(tables.State(created.Id, created.Key[..^1] + (created.Key[^1] == '0' ? '1' : '0')));
        Assert.Null(tables.State(created.Id, string.Empty));
        Assert.Null(tables.State(created.Id, null));
        Assert.Null(tables.State("no-such-table", created.Key));
        Assert.Equal(64, created.Key.Length);

        CreatedInPersonTable other = await tables.CreateAsync(Form());
        Assert.Null(tables.State(created.Id, other.Key));
        Assert.Null(tables.State(other.Id, created.Key));
    }

    [Fact]
    public async Task Tables_are_capped_and_a_full_site_says_every_table_is_busy()
    {
        await using InPersonTables tables = Tables(mostGames: 2);
        await tables.CreateAsync(Form());
        await tables.CreateAsync(Form());

        InPersonBusyException busy = await Assert.ThrowsAsync<InPersonBusyException>(() => tables.CreateAsync(Form()));
        Assert.Contains("All in-person tables are busy", busy.Message, StringComparison.Ordinal);
        Assert.Equal(2, tables.Count);
        Assert.Equal(2, _store.LiveEvents().Count());
    }

    [Fact]
    public async Task With_every_port_held_a_table_is_refused_as_busy_and_its_event_called_off()
    {
        await using InPersonTables tables = Tables(mostGames: 2, ports: 1);
        await tables.CreateAsync(Form());

        await Assert.ThrowsAsync<InPersonBusyException>(() => tables.CreateAsync(Form()));
        Assert.Single(_store.LiveEvents());
    }

    [Theory]
    [InlineData(5, "LSVCube", 2)]
    [InlineData(3, "NoSuchCube", 2)]
    [InlineData(3, "../../secrets", 2)]
    [InlineData(3, "LSVCube", 0)]
    [InlineData(3, "LSVCube", InPersonSetup.MostPacks + 1)]
    public async Task A_form_outside_its_limits_makes_nothing(int layout, string cube, int packs)
    {
        await using InPersonTables tables = Tables();
        await Assert.ThrowsAsync<ArgumentException>(() => tables.CreateAsync(Form(layout, cube, packs)));
        await Assert.ThrowsAsync<ArgumentException>(() => tables.CreateAsync(Form(names: ["Alice", new string('x', InPersonSetup.LongestName + 1)])));
        await Assert.ThrowsAsync<ArgumentException>(() => tables.CreateAsync(Form(names: ["Alice", "Bo\u0007b"])));
        await Assert.ThrowsAsync<ArgumentException>(() => tables.CreateAsync(Form(names: ["Alice", "Bob", "Carol"])));
        Assert.Equal(0, tables.Count);
        Assert.Empty(_store.LiveEvents());
    }

    [Fact]
    public async Task A_table_nobody_comes_to_is_closed_and_its_port_opens_on_a_ticket_no_more()
    {
        await using InPersonTables tables = Tables();
        CreatedInPersonTable created = await tables.CreateAsync(Form());
        int port = Assert.Single(Enumerable.Range(1, 65535), tables.Holds);

        _clock.Advance(TimeSpan.FromMinutes(14));
        await tables.SweepAsync();
        Assert.Equal(1, tables.Count);

        _clock.Advance(TimeSpan.FromMinutes(2));
        await tables.SweepAsync();
        Assert.Equal(0, tables.Count);
        Assert.False(tables.Holds(port));
        Assert.Null(tables.State(created.Id, created.Key));
        Assert.Empty(_store.LiveEvents());

        // Its slot is free for the next.
        await tables.CreateAsync(Form());
        Assert.Equal(1, tables.Count);
    }

    [Fact]
    public async Task A_table_every_device_has_left_is_closed_once_they_have_been_gone_long_enough()
    {
        await using InPersonTables tables = Tables();
        CreatedInPersonTable created = await tables.CreateAsync(Form());
        string link = Devices(tables.State(created.Id, created.Key)!)[2].GetProperty("url").GetString()!;
        DeviceHelloMessage hello = DeviceConnection.Hello(device: link[(link.IndexOf("#device=", StringComparison.Ordinal) + "#device=".Length)..]);

        // The hello names this table's relay, and no other's.
        InPersonRelay relay = tables.RelayFor(hello)!;
        Assert.NotNull(relay);
        Assert.Null(tables.RelayFor(DeviceConnection.Hello(device: new string('0', 64))));
        RelayDevice tablet = relay.Attach();
        relay.Receive(tablet, hello);
        Assert.True(relay.AnyConnected);

        // Here past the first window: kept.
        _clock.Advance(TimeSpan.FromMinutes(20));
        await tables.SweepAsync();
        Assert.Equal(1, tables.Count);

        relay.Detach(tablet);
        _clock.Advance(TimeSpan.FromMinutes(29));
        await tables.SweepAsync();
        Assert.Equal(1, tables.Count);

        _clock.Advance(TimeSpan.FromMinutes(2));
        await tables.SweepAsync();
        Assert.Equal(0, tables.Count);
        Assert.Null(tables.RelayFor(hello));
    }

    [Fact]
    public async Task Closing_a_table_lets_its_devices_go()
    {
        await using InPersonTables tables = Tables();
        CreatedInPersonTable created = await tables.CreateAsync(Form());
        string link = Devices(tables.State(created.Id, created.Key)!)[2].GetProperty("url").GetString()!;
        DeviceHelloMessage hello = DeviceConnection.Hello(device: link[(link.IndexOf("#device=", StringComparison.Ordinal) + "#device=".Length)..]);
        InPersonRelay relay = tables.RelayFor(hello)!;
        RelayDevice tablet = relay.Attach();
        relay.Receive(tablet, hello);
        relay.Detach(tablet);
        RelayDevice again = relay.Attach();
        relay.Receive(again, DeviceConnection.Hello(device: hello.Device));

        relay.CloseAll("closed");
        List<DeviceServerMessage> told = [];
        await foreach (DeviceServerMessage message in again.Outbox.ReadAllAsync()) told.Add(message);
        Assert.Contains(told, message => message is DeviceErrorMessage { Code: Cantrip.Protocol.ErrorCode.TableClosed });
        Assert.False(relay.Recognizes(hello));
    }

    [Fact]
    public async Task Its_site_serves_the_page_to_anyone_and_a_tables_codes_only_with_its_key_in_the_body()
    {
        await using InPersonTables tables = Tables(mostGames: 1);
        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.WebHost.UseUrls("http://127.0.0.1:0");
        builder.Logging.SetMinimumLevel(LogLevel.Warning);
        await using WebApplication app = builder.Build();
        app.UseWebSockets();
        app.UseRouting();
        tables.Map(app);
        await app.StartAsync();
        using var http = new HttpClient { BaseAddress = new Uri(app.Urls.First()) };

        Assert.Contains("In person", await http.GetStringAsync("/inperson"), StringComparison.Ordinal);
        using (JsonDocument choices = JsonDocument.Parse(await http.GetStringAsync("/inperson/api/choices")))
        {
            Assert.False(choices.RootElement.GetProperty("busy").GetBoolean());
            Assert.Contains("LSVCube", choices.RootElement.GetProperty("cubes").EnumerateArray().Select(cube => cube.GetString()));
        }

        Assert.Equal(HttpStatusCode.BadRequest, (await http.PostAsJsonAsync("/inperson/api/start", Form(cube: "NoSuchCube"))).StatusCode);
        using (var huge = new StringContent("{\"names\":[\"" + new string('a', 64 * 1024) + "\"]}", System.Text.Encoding.UTF8, "application/json"))
            Assert.Equal(HttpStatusCode.BadRequest, (await http.PostAsync("/inperson/api/start", huge)).StatusCode);

        HttpResponseMessage started = await http.PostAsJsonAsync("/inperson/api/start", Form());
        Assert.True(started.IsSuccessStatusCode, await started.Content.ReadAsStringAsync());
        Assert.Equal("no-store", started.Headers.CacheControl?.ToString());
        using JsonDocument made = JsonDocument.Parse(await started.Content.ReadAsStringAsync());
        string id = made.RootElement.GetProperty("id").GetString()!, key = made.RootElement.GetProperty("key").GetString()!;

        HttpResponseMessage shown = await http.PostAsJsonAsync("/inperson/api/table", new { id, key });
        Assert.Equal(HttpStatusCode.OK, shown.StatusCode);
        Assert.Contains("#ticket=", await shown.Content.ReadAsStringAsync(), StringComparison.Ordinal);
        Assert.Equal(HttpStatusCode.NotFound, (await http.PostAsJsonAsync("/inperson/api/table", new { id, key = (key[0] == '0' ? "1" : "0") + key[1..] })).StatusCode);
        Assert.Equal(HttpStatusCode.NotFound, (await http.PostAsJsonAsync("/inperson/api/table", new { id })).StatusCode);

        HttpResponseMessage full = await http.PostAsJsonAsync("/inperson/api/start", Form());
        Assert.Equal(HttpStatusCode.ServiceUnavailable, full.StatusCode);
        Assert.Contains("All in-person tables are busy", await full.Content.ReadAsStringAsync(), StringComparison.Ordinal);
        using JsonDocument busy = JsonDocument.Parse(await http.GetStringAsync("/inperson/api/choices"));
        Assert.True(busy.RootElement.GetProperty("busy").GetBoolean());
    }

    [Fact]
    public async Task A_signed_in_creator_reaches_their_table_without_its_key_and_makes_no_second_while_it_is_going()
    {
        await using InPersonTables tables = Tables(mostGames: 3);
        CreatedInPersonTable mine = await tables.CreateAsync(Form(), owner: "42");
        CreatedInPersonTable anyones = await tables.CreateAsync(Form());

        Assert.Equal([mine.Id], tables.Mine("42"));
        Assert.Empty(tables.Mine("43"));
        Assert.Empty(tables.Mine(null));
        Assert.NotNull(tables.State(mine.Id, null, "42"));
        Assert.NotNull(tables.State(mine.Id, mine.Key));
        Assert.Null(tables.State(mine.Id, null, "43"));
        Assert.Null(tables.State(anyones.Id, null, "42"));

        // A second is refused with the one going named, before the site's limit is even asked.
        InPersonAlreadyHostingException again = await Assert.ThrowsAsync<InPersonAlreadyHostingException>(() => tables.CreateAsync(Form(), "42"));
        Assert.Equal(mine.Id, again.Id);
        Assert.Equal(2, tables.Count);
    }

    [Fact]
    public async Task A_creator_cancels_their_table_by_its_key_or_their_account_and_nobody_else_can()
    {
        await using InPersonTables tables = Tables(mostGames: 2);
        CreatedInPersonTable keyed = await tables.CreateAsync(Form());
        CreatedInPersonTable owned = await tables.CreateAsync(Form(), "42");

        Assert.False(await tables.CancelAsync(keyed.Id, owned.Key));
        Assert.False(await tables.CancelAsync(keyed.Id, null, "42"));
        Assert.False(await tables.CancelAsync(owned.Id, null, "43"));
        Assert.False(await tables.CancelAsync("no-such-table", keyed.Key));
        Assert.Equal(2, tables.Count);

        Assert.True(await tables.CancelAsync(keyed.Id, keyed.Key));
        Assert.True(await tables.CancelAsync(owned.Id, null, "42"));
        Assert.Equal(0, tables.Count);
        Assert.Empty(_store.LiveEvents());
        Assert.DoesNotContain(Enumerable.Range(1, 65535), tables.Holds);
        Assert.False(await tables.CancelAsync(keyed.Id, keyed.Key));

        // The creator may start another, and the site has room for it.
        await tables.CreateAsync(Form(), "42");
        Assert.Equal(1, tables.Count);
    }

    [Fact]
    public async Task Tables_an_earlier_run_left_live_are_called_off_and_no_others()
    {
        await using (InPersonTables before = Tables())
        {
            await before.CreateAsync(Form());
        }

        CreatedEvent other = DraftPod.CreateGrid(_store, ContentCache.Get(RepoRoot.ContentRoot), new GridDraftSetup
        {
            Name = "Someone else's",
            Cube = "LSVCube",
            CubeCards = CubeList.Load(Path.Combine(RepoRoot.Path, "docs", "cube-lists", "LSVCube.txt")),
            Packs = 1,
            Players = [("alice", "Alice"), ("bob", "Bob")],
        });

        await using InPersonTables after = Tables();
        Assert.Equal(1, after.CancelLeftOver());
        Assert.Equal(other.Record.Id, Assert.Single(_store.LiveEvents()).Id);
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
