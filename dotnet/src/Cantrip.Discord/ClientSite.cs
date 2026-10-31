using Cantrip.Art;
using Cantrip.Server.Events;
using Cantrip.Server.Host;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Cantrip.Discord;

/// <summary>
/// The web client every ticket reply opens, and the link that opens a seat in it. A player clicks it in Discord and is
/// at the table in their browser: nothing to install, nothing to register.
/// <para>
/// The client is static and knows nothing of any table. The link names the table in its query and carries the ticket
/// after its <c>#</c>, as <c>/?server=ws%3A%2F%2Fhost%3A7782%2Fgame&amp;art=...#ticket=...</c>. A fragment never
/// leaves the browser, so the ticket is in no request and no log, and the client takes it out of the address bar as
/// the page loads (its Ticket.js). Because nothing about the site is per-game, one serves every table - and it could
/// as well be served from any static host or CDN: <see cref="DiscordBotOptions.Client"/> is just a URL.
/// </para>
/// </summary>
internal static class ClientSite
{
    private static readonly Lazy<string> NotFound = new(() => SiteChrome.Resource("notfound.html"));

    /// <summary>The link a player clicks: the client, with the table and art in its query and the ticket after the hash.</summary>
    /// <param name="art">Where card art is, as the client's <c>?art=</c> takes it; null leaves the client to its default.</param>
    public static Uri Link(Uri client, Uri address, string ticket, Uri? art) => new UriBuilder(client)
    {
        Query = $"server={Uri.EscapeDataString(address.AbsoluteUri)}" + (art is null ? string.Empty : $"&art={Uri.EscapeDataString(art.AbsoluteUri)}"),
        Fragment = $"ticket={Uri.EscapeDataString(ticket)}",
    }.Uri;

    /// <summary>
    /// Serves the built web client in <paramref name="root"/>, and card art beside it at <c>/art</c>, on one port until
    /// disposed; and, given <paramref name="tables"/>, every one of those table ports at <c>/table/{port}/game</c>
    /// (<see cref="TableProxy"/>), so that one public address reaches everything; and, given <paramref name="portraits"/>,
    /// players' pictures at <c>/portraits/{player}.png</c>, where the tables tell the client they are; and, given
    /// <paramref name="status"/>, the server's status page at <c>/status</c> (<see cref="StatusSite"/>); and, given
    /// <paramref name="queues"/>, the draft queue page at <c>/draft</c> (<see cref="QueueSite"/>); and, given
    /// <paramref name="play"/>, the constructed play page at <c>/constructed</c> (<see cref="PlaySite"/>); and, given
    /// <paramref name="home"/>, a player's own page at <c>/home</c> (<see cref="HomeSite"/>); and, given <paramref name="decks"/>,
    /// the deck editor's catalog and each player's saved decks at <c>/decks</c> (<see cref="DeckSite"/>); and, given
    /// <paramref name="leagues"/>, the leagues at <c>/leagues</c> (<see cref="LeagueSite"/>); and, given <paramref name="inPerson"/>,
    /// in-person tables at <c>/inperson</c>, open to anyone with no sign-in (docs/in-person.md, "Hosted in person") - each signing players in
    /// with <paramref name="signIn"/> (<see cref="SiteSignIn"/>). The bare address is the home page (<see cref="HomeSite.LandingPage"/>).
    /// </summary>
    /// <param name="before">For a test: run first on every request - to sign a test player in, say. Null for nothing.</param>
    public static async Task<WebApplication> StartAsync(string listen, int port, string root, ArtStore art, IReadOnlySet<int>? tables = null, PortraitStore? portraits = null,
        EventStatusBoard? status = null, SiteSignInOptions? signIn = null, QueueSiteOptions? queues = null, PlaySiteOptions? play = null, TableAccess? access = null, HomeSiteOptions? home = null, DeckSiteOptions? decks = null,
        Func<HttpContext, Func<Task>, Task>? before = null, LeagueSiteOptions? leagues = null, Cantrip.Server.InPerson.InPersonTables? inPerson = null)
    {
        WebApplicationBuilder builder = WebApplication.CreateBuilder();
        builder.WebHost.UseUrls($"http://{listen}:{port}");
        builder.Logging.SetMinimumLevel(LogLevel.Warning);
        builder.Services.AddRouting();
        if (signIn is not null) builder.Services.AddSiteSignIn(signIn);

        // One menu bar for every page: the sections this site serves, and whether anyone can sign in.
        var chrome = new SiteChrome
        {
            SignIn = signIn?.Enabled == true,
            Drafts = queues is not null,
            Leagues = leagues is not null,
            Constructed = play is not null,
            Decks = decks is not null,
            Events = status is not null,
        };
        builder.Services.AddSingleton(chrome);
        WebApplication app = builder.Build();
        if (before is not null) app.Use(before);

        // A ping every 30 seconds: a tunnel or proxy in front (Cloudflare closes a WebSocket silent for 100 seconds) would
        // otherwise drop a player who is only thinking - over a pile, or a deck.
        app.UseWebSockets(new WebSocketOptions { KeepAliveInterval = TimeSpan.FromSeconds(30) });

        // The site's front door is the bare address: for a player signed in, their own page. The client is at the same
        // address with a table in its query (see Link), so a request with a query goes on to it and a seat's link opens
        // as it always did.
        app.Use(async (context, next) =>
        {
            if (HttpMethods.IsGet(context.Request.Method) && context.Request.Path == "/" && !context.Request.QueryString.HasValue)
            {
                if (signIn?.Enabled == true && context.User.Identity?.IsAuthenticated != true)
                {
                    AuthenticateResult signedIn = await context.AuthenticateAsync().ConfigureAwait(false);
                    if (signedIn.Succeeded) context.User = signedIn.Principal;
                }

                IResult page = home is not null && SiteSignIn.Account(context) is not null
                    ? Results.Redirect(context.Request.PathBase + "/" + HomeSite.Route)
                    : HomeSite.LandingPage(context);
                await page.ExecuteAsync(context).ConfigureAwait(false);
                return;
            }

            await next(context).ConfigureAwait(false);
        });
        app.UseWebClient(root);
        app.UseRouting();
        if (signIn is not null) app.UseSiteSignIn(signIn);
        app.MapCardArt(art);
        if (portraits is not null)
        {
            // Asked for by a path that names when the picture was written, so a browser may keep each one for good.
            app.MapGet($"/{PortraitStore.Route}/{{name}}", (string name, HttpContext context) =>
            {
                if (portraits.Served(name) is not { } file) return Results.NotFound();
                context.Response.Headers.CacheControl = "public, max-age=31536000, immutable";
                return Results.File(file, "image/png");
            });
        }

        if (status is not null)
        {
            app.MapEventStatus(status, (context, page) => chrome.Page(context, page, SiteSection.Events));
        }

        if (signIn is not null)
        {
            app.MapSiteSignIn(signIn);
        }

        app.MapGet("/" + HomeSite.LogoRoute, HomeSite.Logo);
        app.MapGet("/" + SiteChrome.StyleRoute, (HttpContext context) => SiteChrome.Asset(context, SiteChrome.StyleRoute));
        app.MapGet("/" + SiteChrome.ScriptRoute, (HttpContext context) => SiteChrome.Asset(context, SiteChrome.ScriptRoute));
        if (home is null) app.MapGet("/" + HomeSite.AboutRoute, HomeSite.LandingPage);

        // Anything else asked for as a page is not there: said so in the site's own page, with the way on.
        app.MapFallback((HttpContext context) =>
            HttpMethods.IsGet(context.Request.Method) && !context.Request.Path.Value!.Contains("/api/", StringComparison.Ordinal)
                ? chrome.Page(context, NotFound.Value, SiteSection.None, StatusCodes.Status404NotFound)
                : Results.NotFound());

        // The pages' names before they were /draft and /constructed, which links already sent are still to.
        app.MapGet("/queue", () => Results.Redirect("/" + QueueSite.Route, permanent: true));
        app.MapGet("/play", () => Results.Redirect("/" + PlaySite.Route, permanent: true));

        if (home is not null)
        {
            app.MapHome(home);
        }

        if (decks is not null)
        {
            app.MapDecks(decks);
        }

        if (queues is not null)
        {
            app.MapDraftQueues(queues);
        }

        if (play is not null)
        {
            app.MapConstructedPlay(play);
        }

        if (leagues is not null)
        {
            app.MapLeagues(leagues);
        }

        if (access is not null)
        {
            TableAccess.MapEnter(app);
        }

        // In person: the setup page in the site's look, its API, and the devices' relay socket beside the client.
        // A signed-in creator reaches their table from any browser; anyone else, by the key their own browser keeps.
        inPerson?.Map(app, (context, page) => chrome.Page(context, page, SiteSection.None),
            owner: context => SiteSignIn.Account(context) is { } account ? account.Id.ToString(System.Globalization.CultureInfo.InvariantCulture) : null);

        if (tables is not null)
        {
            // The tables listen where the bot told them to; reached here from this machine.
            app.MapTables(listen is "0.0.0.0" or "*" or "+" ? "127.0.0.1" : listen, tables, access);
        }

        await app.StartAsync().ConfigureAwait(false);
        return app;
    }
}
