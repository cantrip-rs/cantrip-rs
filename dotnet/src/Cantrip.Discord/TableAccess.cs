using System.Collections.Concurrent;
using System.Globalization;
using System.Text.RegularExpressions;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;

namespace Cantrip.Discord;

/// <summary>
/// Who may open which table. A seat's link names the Discord account it was made for (<see cref="Link"/>), and the table
/// proxy (<see cref="TableProxy"/>) lets a WebSocket through to a table only from a browser signed in with Discord
/// (<see cref="SiteSignIn"/>) as an account the table has a seat for: the ticket alone no longer opens it.
/// <para>
/// A link opens at <c>/enter</c>, which signs the player in first if they are not and then goes on to the web client with
/// the link's query and ticket as they were. The ticket stays after the <c>#</c> throughout, so it is in no request: the
/// page keeps it in the tab for the round trip to Discord and back.
/// </para>
/// </summary>
public sealed partial class TableAccess(TimeProvider? clock = null)
{
    /// <summary>The page a seat's link opens, which signs the player in and passes them on to the client.</summary>
    public const string EnterRoute = "enter";

    /// <summary>Who is asking, for the page to tell whether the browser is signed in.</summary>
    public const string WhoAmIRoute = "whoami";

    /// <summary>How long a seat is kept for its account: well past a game, short of forever for a port that is used again.</summary>
    private static readonly TimeSpan Keep = TimeSpan.FromHours(12);

    private readonly TimeProvider _clock = clock ?? TimeProvider.System;
    private readonly ConcurrentDictionary<(int Port, ulong Player), DateTimeOffset> _seats = new();

    /// <summary>
    /// The one exemption: the ports of tables that open on their ticket alone, with no sign-in - the in-person tables the
    /// bot's site hosts (<c>InPersonTables.Holds</c>), whose phones belong to no Discord account. True for a port only while
    /// such a table holds it; the table still checks every ticket itself. Null exempts nothing.
    /// </summary>
    public Func<int, bool>? TicketOnly { get; init; }

    /// <summary>Whether the table on <paramref name="port"/> opens on its ticket alone (<see cref="TicketOnly"/>).</summary>
    public bool OpensOnTicket(int port) => TicketOnly?.Invoke(port) == true;

    /// <summary>Lets <paramref name="player"/> open the table at <paramref name="address"/>.</summary>
    public void Allow(Uri address, ulong player)
    {
        ArgumentNullException.ThrowIfNull(address);
        if (PortOf(address) is not { } port) return;

        DateTimeOffset now = _clock.GetUtcNow();
        foreach (((int, ulong) seat, DateTimeOffset until) in _seats)
        {
            if (until < now) _seats.TryRemove(seat, out _);
        }

        _seats[(port, player)] = now + Keep;
    }

    /// <summary>Whether <paramref name="player"/> has a seat at the table on <paramref name="port"/>.</summary>
    public bool Allows(int port, ulong player) =>
        _seats.TryGetValue((port, player), out DateTimeOffset until) && until >= _clock.GetUtcNow();

    /// <summary>
    /// The link for <paramref name="player"/>'s seat: <see cref="ClientSite.Link"/> by way of the enter page, and the seat
    /// recorded for them. Without an access list - no sign-in to ask for - the client directly.
    /// </summary>
    public static Uri Link(TableAccess? access, Uri client, Uri address, string ticket, Uri? art, ulong player)
    {
        ArgumentNullException.ThrowIfNull(client);
        if (access is null) return ClientSite.Link(client, address, ticket, art);

        access.Allow(address, player);
        Uri direct = ClientSite.Link(client, address, ticket, art);
        return new UriBuilder(direct) { Path = client.AbsolutePath.TrimEnd('/') + "/" + EnterRoute }.Uri;
    }

    /// <summary>The port of a table's public address, <c>/table/{port}/game</c> (<see cref="TableProxy.Address"/>).</summary>
    public static int? PortOf(Uri address)
    {
        ArgumentNullException.ThrowIfNull(address);
        Match match = TablePath().Match(address.AbsolutePath);
        return match.Success && int.TryParse(match.Groups[1].Value, NumberStyles.None, CultureInfo.InvariantCulture, out int port) ? port : null;
    }

    /// <summary>Serves the enter page and the answer to who is signed in.</summary>
    public static IEndpointRouteBuilder MapEnter(IEndpointRouteBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);

        app.MapGet("/" + WhoAmIRoute, (HttpContext context) =>
        {
            context.Response.Headers.CacheControl = "no-store";
            return SiteSignIn.Account(context) is { } account
                ? Results.Json(new { id = account.Id.ToString(CultureInfo.InvariantCulture), name = account.Name })
                : Results.Unauthorized();
        });

        app.MapGet("/" + EnterRoute, (HttpContext context) =>
        {
            IResult page = SiteChrome.For(context).Page(context, EnterPage, SiteSection.None);
            context.Response.Headers.CacheControl = "no-store";
            return page;
        });

        return app;
    }

    [GeneratedRegex(@"/table/(\d+)/game$", RegexOptions.CultureInvariant)]
    private static partial Regex TablePath();

    private const string EnterPage = """
        <!doctype html>
        <html lang="en">
        <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <title>Cantrip - Taking you to your seat</title>
        <!--site:head-->
        </head>
        <body data-page="">
        <!--site:header-->
        <main id="main">
          <div class="signin-card">
            <img src="logo.png" alt="" width="72" height="72">
            <h2>Taking you to your seat</h2>
            <p id="say" role="status">Checking your Discord sign-in&hellip;</p>
            <div class="actions" id="ways" hidden><a class="button primary" href="home">Go to my page</a><a class="button" href="./">Home</a></div>
          </div>
        </main>
        <!--site:footer-->
        <script>
        (async () => {
          const say = text => { document.getElementById('say').textContent = text; };
          const stuck = text => { say(text); document.getElementById('ways').hidden = false; };
          const kept = 'cantrip.enter', tried = 'cantrip.enter.signin';
          try {
            // The link's query and ticket, held in this tab across the trip to Discord: the ticket is after the # so no
            // request carries it, and it stays that way.
            let target = location.search + location.hash;
            if (!target) target = sessionStorage.getItem(kept) || '';
            if (!target) { stuck('This page opens a seat from a link to one: there is none here.'); return; }

            const answer = await fetch('whoami', { credentials: 'same-origin', cache: 'no-store' });
            if (answer.ok) {
              sessionStorage.removeItem(kept);
              sessionStorage.removeItem(tried);
              location.replace(new URL('./' + target, location.href));
              return;
            }

            if (sessionStorage.getItem(tried)) {
              sessionStorage.removeItem(tried);
              stuck('Signing in with Discord did not work. Open the link from Discord again to retry.');
              return;
            }

            sessionStorage.setItem(kept, target);
            sessionStorage.setItem(tried, '1');
            say('Taking you to Discord to sign in…');
            location.replace(new URL('signin?then=enter', location.href));
          } catch (error) {
            stuck('Could not check your sign-in. Reload to try again.');
          }
        })();
        </script>
        </body>
        </html>
        """;
}
