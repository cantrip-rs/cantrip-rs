using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Cantrip.Client.InPerson;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.Events;
using Cantrip.Server.InPerson;
using Cantrip.TestKit;
using Microsoft.AspNetCore.Http;
using Xunit;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>What the setup page decides, with no server: the clocks, the address, the line a tablet shows, who it is served to.</summary>
public sealed class InPersonSetupTests
{
    [Fact]
    public void Clocks_off_give_no_pick_clock_no_match_clock_and_forfeit_nobody_for_a_sleeping_phone()
    {
        DraftTiming off = InPersonHost.Timing(clocks: false);
        Assert.Equal(TimeSpan.Zero, off.GridPick);
        Assert.Equal(TimeSpan.Zero, off.MatchClock);
        Assert.True(off.Away >= TimeSpan.FromHours(1) && off.NoShow >= TimeSpan.FromHours(1));
        Assert.True(InPersonHost.NoClock > TimeSpan.FromDays(1));

        DraftTiming on = InPersonHost.Timing(clocks: true);
        var usual = new DraftTiming();
        Assert.Equal((usual.GridPick, usual.MatchClock, usual.Away), (on.GridPick, on.MatchClock, on.Away));
    }

    [Theory]
    [InlineData("192.168.1.20", true)]
    [InlineData("10.0.0.5", true)]
    [InlineData("172.16.0.1", true)]
    [InlineData("172.31.255.1", true)]
    [InlineData("172.32.0.1", false)]
    [InlineData("169.254.3.4", false)]
    [InlineData("8.8.8.8", false)]
    [InlineData("::1", false)]
    public void Only_a_private_IPv4_address_is_offered_to_the_devices(string address, bool offered) =>
        Assert.Equal(offered, LanAddress.IsPrivate(IPAddress.Parse(address)));

    [Fact]
    public void Every_address_offered_is_private()
    {
        Assert.All(LanAddress.Candidates(), each => Assert.True(LanAddress.IsPrivate(each.Address)));
    }

    [Fact]
    public void The_setup_page_is_served_to_this_computer_alone()
    {
        static HttpContext From(string address) => new DefaultHttpContext { Connection = { RemoteIpAddress = IPAddress.Parse(address) } };
        Assert.True(InPersonSite.IsLoopback(From("127.0.0.1")));
        Assert.True(InPersonSite.IsLoopback(From("::1")));
        Assert.False(InPersonSite.IsLoopback(From("192.168.1.30")));
        Assert.False(InPersonSite.IsLoopback(new DefaultHttpContext()));
    }

    [Fact]
    public void A_device_link_is_drawn_as_an_SVG_QR_code()
    {
        string svg = InPersonHost.QrSvg("http://192.168.1.20:5200/?inperson=tablet&side=0#device=00FF");
        Assert.StartsWith("<svg", svg.TrimStart(), StringComparison.Ordinal);
        Assert.Contains("</svg>", svg, StringComparison.Ordinal);
    }

    [Fact]
    public void A_tablet_is_told_where_the_event_stands_in_a_line()
    {
        static StatusEventDetail Detail(string stage, IReadOnlyList<StatusPlayer> players, StatusDraft? draft = null, string detail = "") => new(
            new StatusEvent("e", EventKinds.GridDraft, "In person", "grid", "LSVCube", stage, true, 0, 0, DateTimeOffset.UnixEpoch,
                DateTimeOffset.UnixEpoch, null, detail, string.Empty, players, null, null),
            null, draft, [], []);
        StatusPlayer alice = new("Alice", 1, false, true, false), bob = new("Bob", 2, false, false, false);

        Assert.Equal("Waiting for Bob to scan their phone's code.", InPersonHost.PhaseLine(Detail(nameof(EventStage.Seating), [alice, bob])));
        Assert.Equal(TablePhase.Setup, InPersonHost.PhaseOf(Detail(nameof(EventStage.Seating), [alice, bob])));

        var draft = new StatusDraft("grid", 3, 18, 9, 1, 1, [new("Alice", 1, 4, 0, null, false), new("Bob", 2, 4, 0, null, true)], 0, [], 0, 0, 162);
        Assert.Equal("Grid 3 of 18: Bob picks.", InPersonHost.PhaseLine(Detail(nameof(EventStage.Drafting), [alice, bob], draft)));
        Assert.Equal(TablePhase.Drafting, InPersonHost.PhaseOf(Detail(nameof(EventStage.Drafting), [alice, bob], draft)));

        Assert.Equal("Building decks: Alice has submitted.",
            InPersonHost.PhaseLine(Detail(nameof(EventStage.DeckBuilding), [alice with { Submitted = true }, bob])));
        Assert.Equal("Game 1.", InPersonHost.PhaseLine(Detail(nameof(EventStage.Playing), [alice, bob], detail: "Game 1.")));
        Assert.Equal(TablePhase.Finished, InPersonHost.PhaseOf(Detail(nameof(EventStage.Complete), [alice, bob])));
        Assert.Equal(string.Empty, InPersonHost.PhaseLine(null));
    }
}

/// <summary>
/// The setup page's server for real: a table started from its form, its devices' links, and a phone and a tablet taking
/// their places by them over the relay's socket. On the wall clock because it is a real server.
/// </summary>
[Trait("Category", "WallClock")]
public sealed class InPersonSetupServerTests : IDisposable
{
    private readonly string _data = Path.Combine(Path.GetTempPath(), "cantrip-in-person-" + Guid.NewGuid().ToString("N"));

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

    [Fact]
    public async Task The_form_starts_a_grid_draft_whose_codes_seat_each_device()
    {
        await using InPersonHost host = await InPersonHost.StartAsync(new InPersonOptions
        {
            ContentRoot = RepoRoot.ContentRoot,
            Store = new EventStore(Path.Combine(_data, "events")),
            CubeDirectories = [Path.Combine(RepoRoot.Path, "docs", "cube-lists")],
            Port = 0,
            TablePort = 0,
            ListenAddress = "127.0.0.1",
            ArtCache = Path.Combine(_data, "art"),
            Rules = new CapturingRulesFactory(),
        });
        using var http = new HttpClient { BaseAddress = host.Site };

        Assert.Contains("Cantrip in person", await http.GetStringAsync("setup"), StringComparison.Ordinal);
        using (JsonDocument before = JsonDocument.Parse(await http.GetStringAsync("setup/api/state")))
        {
            Assert.False(before.RootElement.GetProperty("started").GetBoolean());
            Assert.Contains("LSVCube", before.RootElement.GetProperty("cubes").EnumerateArray().Select(cube => cube.GetString()));
        }

        HttpResponseMessage refused = await http.PostAsJsonAsync("setup/api/start", new SetupRequest(["Alice", "Bob"], "NoSuchCube", 1, 4, false, "127.0.0.1"));
        Assert.Equal(HttpStatusCode.BadRequest, refused.StatusCode);

        HttpResponseMessage started = await http.PostAsJsonAsync("setup/api/start", new SetupRequest(["Alice", "Bob"], "LSVCube", 1, 4, false, "127.0.0.1"));
        Assert.True(started.IsSuccessStatusCode, await started.Content.ReadAsStringAsync());
        using JsonDocument state = JsonDocument.Parse(await started.Content.ReadAsStringAsync());
        List<JsonElement> devices = [.. state.RootElement.GetProperty("devices").EnumerateArray()];
        Assert.Equal(["Alice's phone", "Bob's phone", "Alice's tablet", "Bob's tablet"], devices.Select(device => device.GetProperty("name").GetString()));
        Assert.All(devices, device => Assert.Contains("<svg", device.GetProperty("qr").GetString(), StringComparison.Ordinal));
        Assert.All(devices, device => Assert.False(device.GetProperty("connected").GetBoolean()));

        string phoneLink = devices[0].GetProperty("url").GetString()!;
        Assert.Contains("inperson=phone", phoneLink, StringComparison.Ordinal);
        Assert.Contains(Uri.EscapeDataString($"ws://127.0.0.1:{host.Server!.Port}/game"), phoneLink, StringComparison.Ordinal);
        string ticket = phoneLink[(phoneLink.IndexOf("#ticket=", StringComparison.Ordinal) + "#ticket=".Length)..];
        string tabletLink = devices[3].GetProperty("url").GetString()!;
        Assert.Contains("inperson=tablet&side=2", tabletLink, StringComparison.Ordinal);
        string token = tabletLink[(tabletLink.IndexOf("#device=", StringComparison.Ordinal) + "#device=".Length)..];

        Uri relay = DeviceConnection.AddressFor(new Uri(phoneLink.Replace("#ticket=" + ticket, string.Empty, StringComparison.Ordinal)));
        using DeviceConnection phone = await DeviceConnection.ConnectAsync(relay);
        phone.Send(DeviceConnection.Hello(ticket: ticket));
        using DeviceConnection tablet = await DeviceConnection.ConnectAsync(relay);
        tablet.Send(DeviceConnection.Hello(device: token));

        List<DeviceServerMessage> phoneSaw = [], tabletSaw = [];
        DateTime deadline = DateTime.UtcNow + TimeSpan.FromSeconds(20);
        while (!(phoneSaw.OfType<DeviceWelcomeMessage>().Any() && tabletSaw.OfType<DevicePhaseMessage>().Any(phase => phase.Line.Length > 0)))
        {
            Assert.True(DateTime.UtcNow < deadline, $"phone: {string.Join(" | ", phoneSaw.Select(m => m.Describe()))}; tablet: {string.Join(" | ", tabletSaw.Select(m => m.Describe()))}");
            phoneSaw.AddRange(phone.Receive());
            tabletSaw.AddRange(tablet.Receive());
            await Task.Delay(10);
        }

        Assert.Equal(1, phoneSaw.OfType<DeviceWelcomeMessage>().Single().Seat);
        Assert.Equal(2, tabletSaw.OfType<DeviceWelcomeMessage>().Single().Side);

        using JsonDocument after = JsonDocument.Parse(await http.GetStringAsync("setup/api/state"));
        Assert.Equal([true, false, false, true], after.RootElement.GetProperty("devices").EnumerateArray().Select(device => device.GetProperty("connected").GetBoolean()));
        Assert.Equal("4", host.Pod!.Kept("inperson.layout"));
        Assert.Equal("off", host.Pod.Kept("inperson.clocks"));
    }
}
