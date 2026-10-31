using System.Net;
using System.Net.Http.Json;
using System.Net.Sockets;
using System.Net.WebSockets;
using System.Text.Json;
using Cantrip.Art;
using Cantrip.Client;
using Cantrip.Client.InPerson;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.Events;
using Cantrip.Server.InPerson;
using Cantrip.TestKit;
using Xunit;

namespace Cantrip.Discord.Tests;

/// <summary>
/// In-person play on the bot's own site (docs/in-person.md, "Hosted in person"), over real sockets: a table made at
/// <c>/inperson</c> with nobody signed in, its phone seated through the bot's table proxy on its ticket alone - behind
/// sign-in, where every other table wants a Discord account - and its tablet welcomed on the relay's socket beside the
/// client.
/// </summary>
[Trait("Category", "WallClock")]
public sealed class InPersonSiteTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), "cantrip-in-person-site-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        try
        {
            Directory.Delete(_root, recursive: true);
        }
        catch (IOException)
        {
        }
    }

    [Fact]
    public async Task A_table_made_with_no_sign_in_seats_its_phone_on_its_ticket_and_welcomes_its_tablet_through_the_site()
    {
        Directory.CreateDirectory(_root);
        await File.WriteAllTextAsync(Path.Combine(_root, "index.html"), "<!doctype html><title>Cantrip</title>");
        List<int> free = FreePorts(3);
        (int web, int table, int spare) = (free[0], free[1], free[2]);
        var site = new Uri($"http://127.0.0.1:{web}/");

        await using var tables = new InPersonTables(new InPersonTablesOptions
        {
            ContentRoot = RepoRoot.ContentRoot,
            Store = new EventStore(Path.Combine(_root, "data")),
            CubeDirectories = [Path.Combine(RepoRoot.Path, "docs", "cube-lists")],
            Ports = [table, spare],
            Site = site,
            Art = site,
            TableAddress = port => TableProxy.Address(site, port),
            MostGames = 1,
        });
        var access = new TableAccess { TicketOnly = tables.Holds };
        var art = new ArtStore(new ArtStoreOptions { ContentRoot = _root, CacheDirectory = Path.Combine(_root, "art") });
        await using var server = await ClientSite.StartAsync("127.0.0.1", web, _root, art, new HashSet<int> { table, spare }, access: access, inPerson: tables);
        using var http = new HttpClient { BaseAddress = site };

        string page = await http.GetStringAsync("inperson");
        Assert.Contains("In person", page, StringComparison.Ordinal);
        Assert.Contains("site-header", page, StringComparison.Ordinal);

        HttpResponseMessage started = await http.PostAsJsonAsync("inperson/api/start", new SetupRequest(["Alice", "Bob"], "LSVCube", 2, 3, false, null));
        Assert.True(started.IsSuccessStatusCode, await started.Content.ReadAsStringAsync());
        using JsonDocument made = JsonDocument.Parse(await started.Content.ReadAsStringAsync());
        HttpResponseMessage shown = await http.PostAsJsonAsync("inperson/api/table",
            new { id = made.RootElement.GetProperty("id").GetString(), key = made.RootElement.GetProperty("key").GetString() });
        using JsonDocument state = JsonDocument.Parse(await shown.Content.ReadAsStringAsync());
        List<string> links = [.. state.RootElement.GetProperty("devices").EnumerateArray().Select(device => device.GetProperty("url").GetString()!)];

        // The phone's link names the table behind the site's proxy; its ticket alone seats it, with nobody signed in.
        Uri phone = new(links[0]);
        string ticket = phone.Fragment[(phone.Fragment.IndexOf("ticket=", StringComparison.Ordinal) + "ticket=".Length)..];
        Uri address = new(Uri.UnescapeDataString(phone.Query.Split('&').Single(part => part.StartsWith("server=", StringComparison.Ordinal) || part.StartsWith("?server=", StringComparison.Ordinal)).Split('=', 2)[1]));
        Assert.Equal(TableProxy.Address(site, table), address);

        GameClient seated = await SitAsync(address, ticket);
        Assert.Equal(1, seated.Membership!.Seat);

        // The table still checks the ticket.
        GameClient refused = await SitAsync(address, "not-a-ticket");
        Assert.Null(refused.Membership);
        Assert.Contains(refused.Errors, error => error.Code == ErrorCode.MembershipRefused);

        // A port no in-person table holds wants a signed-in account, as every table behind sign-in does.
        using (var probe = new ClientWebSocket())
        {
            WebSocketException wanted = await Assert.ThrowsAnyAsync<WebSocketException>(() => probe.ConnectAsync(TableProxy.Address(site, spare), CancellationToken.None));
            Assert.Contains("401", wanted.Message, StringComparison.Ordinal);
        }

        // The tablet, on the relay's socket beside the client.
        Uri relay = DeviceConnection.AddressFor(new Uri(links[2]));
        Assert.Equal($"ws://127.0.0.1:{web}/inperson/socket", relay.AbsoluteUri);
        string token = links[2][(links[2].IndexOf("#device=", StringComparison.Ordinal) + "#device=".Length)..];
        using DeviceConnection tablet = await DeviceConnection.ConnectAsync(relay);
        tablet.Send(DeviceConnection.Hello(device: token));
        List<DeviceServerMessage> told = [];
        DateTime deadline = DateTime.UtcNow + TimeSpan.FromSeconds(20);
        while (!told.OfType<DeviceWelcomeMessage>().Any())
        {
            Assert.True(DateTime.UtcNow < deadline, string.Join(" | ", told.Select(message => message.Describe())) + tablet.LastError);
            told.AddRange(tablet.Receive());
            await Task.Delay(10);
        }

        Assert.Equal(0, told.OfType<DeviceWelcomeMessage>().Single().Side);

        // A code for no table here is turned away.
        using DeviceConnection stray = await DeviceConnection.ConnectAsync(relay);
        stray.Send(DeviceConnection.Hello(device: new string('0', 64)));
        List<DeviceServerMessage> strayTold = [];
        deadline = DateTime.UtcNow + TimeSpan.FromSeconds(20);
        while (!strayTold.OfType<DeviceErrorMessage>().Any())
        {
            Assert.True(DateTime.UtcNow < deadline, stray.LastError);
            strayTold.AddRange(stray.Receive());
            await Task.Delay(10);
        }

        Assert.Equal(ErrorCode.MembershipRefused, strayTold.OfType<DeviceErrorMessage>().Single().Code);

        // The creator calls the table off - with JSON and its key only - and the site has room again.
        object mine = new { id = made.RootElement.GetProperty("id").GetString(), key = made.RootElement.GetProperty("key").GetString() };
        using (var plain = new StringContent(JsonSerializer.Serialize(mine), System.Text.Encoding.UTF8, "text/plain"))
        {
            Assert.Equal(HttpStatusCode.UnsupportedMediaType, (await http.PostAsync("inperson/api/cancel", plain)).StatusCode);
        }

        Assert.Equal(HttpStatusCode.NotFound, (await http.PostAsJsonAsync("inperson/api/cancel", new { id = made.RootElement.GetProperty("id").GetString(), key = "wrong" })).StatusCode);
        Assert.Equal(1, tables.Count);
        Assert.Equal(HttpStatusCode.OK, (await http.PostAsJsonAsync("inperson/api/cancel", mine)).StatusCode);
        Assert.Equal(0, tables.Count);
        Assert.Equal(HttpStatusCode.NotFound, (await http.PostAsJsonAsync("inperson/api/table", mine)).StatusCode);
    }

    private static async Task<GameClient> SitAsync(Uri address, string ticket)
    {
        WebSocketTransport transport = await WebSocketTransport.ConnectAsync(address);
        var client = new GameClient(transport);
        client.Join("Anyone", 0, ticket);
        DateTime deadline = DateTime.UtcNow + TimeSpan.FromSeconds(30);
        while (client.Membership is null && client.Errors.Count == 0)
        {
            Assert.True(DateTime.UtcNow < deadline, $"Not seated: {transport.LastError}");
            client.Pump();
            await Task.Delay(5);
        }

        return client;
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
