using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Net.WebSockets;
using System.Security.Claims;
using Cantrip.Server;
using Cantrip.TestKit;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Xunit;

namespace Cantrip.Discord.Tests;

/// <summary>
/// Tables behind sign-in: a seat's link goes by the enter page, and the table proxy lets a WebSocket through only from a
/// browser signed in as an account the table has a seat for. Sign-in itself is Discord's, so here the account is a header
/// a test middleware turns into the claims the cookie would have carried.
/// </summary>
public sealed class TableAccessTests
{
    private const string AsHeader = "X-Test-Account";

    private sealed class Clock : TimeProvider
    {
        private DateTimeOffset _now = new(2026, 10, 1, 0, 0, 0, TimeSpan.Zero);
        public override DateTimeOffset GetUtcNow() => _now;
        public void Advance(TimeSpan duration) => _now += duration;
    }

    [Fact]
    public void A_seats_link_goes_by_the_enter_page_with_its_query_and_ticket_unchanged()
    {
        var access = new TableAccess();
        var site = new Uri("https://cantrip.example.com/");
        Uri table = TableProxy.Address(site, 7781);

        Uri link = TableAccess.Link(access, site, table, "t0k3n", new Uri("https://cantrip.example.com/art/{size}/{slug}.webp"), 42);
        Uri direct = TableAccess.Link(null, site, table, "t0k3n", null, 42);

        Assert.Equal("/enter", link.AbsolutePath);
        Assert.Equal("ticket=t0k3n", link.Fragment.TrimStart('#'));
        Assert.Contains("server=" + Uri.EscapeDataString(table.AbsoluteUri), link.Query, StringComparison.Ordinal);
        Assert.Equal("/", direct.AbsolutePath);
        Assert.True(access.Allows(7781, 42));
    }

    [Fact]
    public void A_seat_is_for_its_account_at_its_table_and_lapses()
    {
        var clock = new Clock();
        var access = new TableAccess(clock);
        access.Allow(TableProxy.Address(new Uri("https://cantrip.example.com/"), 7781), 42);

        Assert.True(access.Allows(7781, 42));
        Assert.False(access.Allows(7781, 43));
        Assert.False(access.Allows(7782, 42));

        clock.Advance(TimeSpan.FromHours(13));
        Assert.False(access.Allows(7781, 42));
    }

    [Theory]
    [InlineData("wss://cantrip.example.com/table/7790/game", 7790)]
    [InlineData("ws://localhost:7779/table/7781/game", 7781)]
    [InlineData("wss://example.com/cantrip/table/7790/game", 7790)]
    [InlineData("ws://localhost:7781/game", null)]
    public void The_port_is_read_from_a_tables_public_address(string address, int? port)
    {
        Assert.Equal(port, TableAccess.PortOf(new Uri(address)));
    }

    [Fact]
    public async Task A_table_opens_only_for_a_signed_in_account_with_a_seat_and_the_enter_page_says_who_is_signed_in()
    {
        List<int> free = FreePorts(2);
        (int web, int table) = (free[0], free[1]);
        var site = new Uri($"http://127.0.0.1:{web}/");
        var access = new TableAccess();

        DeckList deck = DeckList.Load(DeckList.Find(Path.Combine(RepoRoot.Path, "decks"), "rw-aggro"));
        await using var matchmaker = new Matchmaker(new MatchmakerOptions
        {
            ContentRoot = RepoRoot.ContentRoot,
            Deck = deck,
            Ports = [table],
            TableAddress = port => TableProxy.Address(site, port),
        });

        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.WebHost.UseUrls(site.AbsoluteUri);
        builder.Logging.SetMinimumLevel(LogLevel.Warning);
        builder.Services.AddRouting();
        WebApplication app = builder.Build();
        app.UseWebSockets();
        app.UseRouting();
        app.Use((context, next) =>
        {
            if (context.Request.Headers.TryGetValue(AsHeader, out var id))
            {
                var identity = new ClaimsIdentity([new Claim(ClaimTypes.NameIdentifier, id.ToString()), new Claim(ClaimTypes.Name, "Tester")], "test");
                context.User = new ClaimsPrincipal(identity);
            }

            return next(context);
        });
        TableAccess.MapEnter(app);
        app.MapTables("127.0.0.1", new HashSet<int> { table }, access);
        await app.StartAsync();
        try
        {
            Seating seating = await matchmaker.HostAsync(new DiscordPlayer(7, "Alice"));
            access.Allow(seating.Game!.Address, 7);
            Uri socket = seating.Game.Address;

            // Not signed in, or signed in without a seat there: refused before the table is reached.
            Assert.Equal(HttpStatusCode.Unauthorized, await Refusal(socket, null));
            Assert.Equal(HttpStatusCode.Forbidden, await Refusal(socket, 8));

            // The account the seat is for gets through.
            using var alice = new ClientWebSocket();
            alice.Options.SetRequestHeader(AsHeader, "7");
            await alice.ConnectAsync(socket, CancellationToken.None);
            Assert.Equal(WebSocketState.Open, alice.State);

            using var http = new HttpClient { BaseAddress = site };
            Assert.Equal(HttpStatusCode.Unauthorized, (await http.GetAsync("/whoami")).StatusCode);
            using var signedIn = new HttpRequestMessage(HttpMethod.Get, "/whoami");
            signedIn.Headers.Add(AsHeader, "7");
            HttpResponseMessage who = await http.SendAsync(signedIn);
            Assert.Equal("""{"id":"7","name":"Tester"}""", await who.Content.ReadAsStringAsync());

            HttpResponseMessage enter = await http.GetAsync("/enter");
            Assert.Equal(HttpStatusCode.OK, enter.StatusCode);
            Assert.Equal("no-store", enter.Headers.CacheControl?.ToString());
            Assert.Contains("signin?then=enter", await enter.Content.ReadAsStringAsync(), StringComparison.Ordinal);
        }
        finally
        {
            await app.StopAsync();
            await app.DisposeAsync();
        }
    }

    [Fact]
    public void Only_the_ports_it_is_told_of_open_on_a_ticket_alone()
    {
        Assert.False(new TableAccess().OpensOnTicket(7790));
        var access = new TableAccess { TicketOnly = port => port == 7790 };
        Assert.True(access.OpensOnTicket(7790));
        Assert.False(access.OpensOnTicket(7791));
    }

    [Fact]
    public async Task An_in_person_table_opens_on_its_ticket_alone_and_every_other_table_still_wants_a_seat()
    {
        List<int> free = FreePorts(3);
        (int web, int inPerson, int other) = (free[0], free[1], free[2]);
        var site = new Uri($"http://127.0.0.1:{web}/");
        bool held = true;
        var access = new TableAccess { TicketOnly = port => held && port == inPerson };

        DeckList deck = DeckList.Load(DeckList.Find(Path.Combine(RepoRoot.Path, "decks"), "rw-aggro"));
        await using var matchmaker = new Matchmaker(new MatchmakerOptions
        {
            ContentRoot = RepoRoot.ContentRoot,
            Deck = deck,
            Ports = [inPerson, other],
            TableAddress = port => TableProxy.Address(site, port),
        });

        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.WebHost.UseUrls(site.AbsoluteUri);
        builder.Logging.SetMinimumLevel(LogLevel.Warning);
        builder.Services.AddRouting();
        WebApplication app = builder.Build();
        app.UseWebSockets();
        app.UseRouting();
        app.Use((context, next) =>
        {
            if (context.Request.Headers.TryGetValue(AsHeader, out var id))
            {
                var identity = new ClaimsIdentity([new Claim(ClaimTypes.NameIdentifier, id.ToString()), new Claim(ClaimTypes.Name, "Tester")], "test");
                context.User = new ClaimsPrincipal(identity);
            }

            return next(context);
        });
        app.MapTables("127.0.0.1", new HashSet<int> { inPerson, other }, access);
        await app.StartAsync();
        try
        {
            Uri first = (await matchmaker.HostAsync(new DiscordPlayer(7, "Alice"))).Game!.Address;
            Uri second = (await matchmaker.HostAsync(new DiscordPlayer(9, "Carol"))).Game!.Address;
            Uri exempt = TableAccess.PortOf(first) == inPerson ? first : second;
            Uri guarded = exempt == first ? second : first;
            Assert.Equal(inPerson, TableAccess.PortOf(exempt));

            // The in-person table: through with nobody signed in, and with an account it has no seat for.
            using (var nobody = new ClientWebSocket())
            {
                await nobody.ConnectAsync(exempt, CancellationToken.None);
                Assert.Equal(WebSocketState.Open, nobody.State);
            }

            using (var stranger = new ClientWebSocket())
            {
                stranger.Options.SetRequestHeader(AsHeader, "8");
                await stranger.ConnectAsync(exempt, CancellationToken.None);
                Assert.Equal(WebSocketState.Open, stranger.State);
            }

            // Every other table, exactly as before.
            Assert.Equal(HttpStatusCode.Unauthorized, await Refusal(guarded, null));
            Assert.Equal(HttpStatusCode.Forbidden, await Refusal(guarded, 8));

            // Once the in-person table gives its port back, the port wants a seat like any other.
            held = false;
            Assert.Equal(HttpStatusCode.Unauthorized, await Refusal(exempt, null));
            Assert.Equal(HttpStatusCode.Forbidden, await Refusal(exempt, 8));
        }
        finally
        {
            await app.StopAsync();
            await app.DisposeAsync();
        }
    }

    private static async Task<HttpStatusCode?> Refusal(Uri socket, int? account)
    {
        using var probe = new ClientWebSocket();
        if (account is { } id) probe.Options.SetRequestHeader(AsHeader, id.ToString(CultureInfo.InvariantCulture));
        WebSocketException refused = await Assert.ThrowsAnyAsync<WebSocketException>(() => probe.ConnectAsync(socket, CancellationToken.None));

        // The upgrade's own answer is in the message: "The server returned status code '401' when status code '101' was expected."
        return refused.Message.Contains("401", StringComparison.Ordinal) ? HttpStatusCode.Unauthorized
            : refused.Message.Contains("403", StringComparison.Ordinal) ? HttpStatusCode.Forbidden
            : null;
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
