using System.Net.WebSockets;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;

namespace Cantrip.Discord;

/// <summary>
/// Every table behind the bot's one web port, at <c>/table/{port}/game</c>: a player's WebSocket is accepted there and
/// passed through, frame for frame, to the table listening on that port on this machine. So one public address - an
/// HTTPS tunnel, or a TLS proxy - reaches the web client, its art and every table, and the tables themselves never
/// listen anywhere but here.
/// <para>
/// Only the bot's own table ports are passed through; any other port is not found, so the route cannot be used to reach
/// anything else on the machine. The table still checks every ticket and membership itself; behind sign-in
/// (<see cref="TableAccess"/>) this also checks that the account signed in has a seat there.
/// </para>
/// </summary>
internal static class TableProxy
{
    /// <summary>The public address of a table on a port, under the bot's public URL (https becomes wss).</summary>
    public static Uri Address(Uri publicUrl, int port)
    {
        var builder = new UriBuilder(publicUrl)
        {
            Scheme = publicUrl.Scheme == Uri.UriSchemeHttps ? "wss" : "ws",
            Path = publicUrl.AbsolutePath.TrimEnd('/') + $"/table/{port}/game",
            Query = string.Empty,
            Fragment = string.Empty,
        };
        if (publicUrl.IsDefaultPort) builder.Port = -1;
        return builder.Uri;
    }

    /// <summary>Passes WebSockets at <c>/table/{port}/game</c> through to the tables on <paramref name="host"/>, for the ports allowed.</summary>
    public static void MapTables(this IEndpointRouteBuilder app, string host, IReadOnlySet<int> ports, TableAccess? access = null)
    {
        app.Map("/table/{port:int}/game", async (HttpContext context, int port) =>
        {
            if (!ports.Contains(port))
            {
                context.Response.StatusCode = StatusCodes.Status404NotFound;
                return;
            }

            if (!context.WebSockets.IsWebSocketRequest)
            {
                context.Response.StatusCode = StatusCodes.Status400BadRequest;
                return;
            }

            // Behind sign-in, a table is for the Discord accounts it has seats for, ticket or not - but for an in-person
            // table, which its ticket alone opens (TableAccess.TicketOnly).
            if (access is not null && !access.OpensOnTicket(port))
            {
                if (SiteSignIn.Account(context) is not { } account)
                {
                    context.Response.StatusCode = StatusCodes.Status401Unauthorized;
                    return;
                }

                if (!access.Allows(port, account.Id))
                {
                    context.Response.StatusCode = StatusCodes.Status403Forbidden;
                    return;
                }
            }

            using var table = new ClientWebSocket();
            try
            {
                await table.ConnectAsync(new Uri($"ws://{host}:{port}/game"), context.RequestAborted).ConfigureAwait(false);
            }
            catch (Exception error) when (error is WebSocketException or HttpRequestException or OperationCanceledException)
            {
                // No table on that port now: one that finished, or never opened.
                context.Response.StatusCode = StatusCodes.Status502BadGateway;
                return;
            }

            using WebSocket player = await context.WebSockets.AcceptWebSocketAsync().ConfigureAwait(false);
            using var done = CancellationTokenSource.CreateLinkedTokenSource(context.RequestAborted);
            Task up = PumpAsync(player, table, done.Token);
            Task down = PumpAsync(table, player, done.Token);

            // Either side going ends both.
            await Task.WhenAny(up, down).ConfigureAwait(false);
            await done.CancelAsync().ConfigureAwait(false);
            await Close(player).ConfigureAwait(false);
            await Close(table).ConfigureAwait(false);
            await Task.WhenAll(Quietly(up), Quietly(down)).ConfigureAwait(false);
        });
    }

    /// <summary>Copies frames from one socket to the other until the first closes or fails.</summary>
    private static async Task PumpAsync(WebSocket from, WebSocket to, CancellationToken cancellation)
    {
        var buffer = new byte[16 * 1024];
        while (from.State == WebSocketState.Open && to.State == WebSocketState.Open && !cancellation.IsCancellationRequested)
        {
            WebSocketReceiveResult result = await from.ReceiveAsync(buffer, cancellation).ConfigureAwait(false);
            if (result.MessageType == WebSocketMessageType.Close)
            {
                return;
            }

            await to.SendAsync(buffer.AsMemory(0, result.Count), result.MessageType, result.EndOfMessage, cancellation).ConfigureAwait(false);
        }
    }

    private static async Task Close(WebSocket socket)
    {
        try
        {
            if (socket.State is WebSocketState.Open or WebSocketState.CloseReceived)
            {
                using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(2));
                await socket.CloseOutputAsync(WebSocketCloseStatus.NormalClosure, "Connection ended.", timeout.Token).ConfigureAwait(false);
            }
        }
        catch (Exception error) when (error is WebSocketException or OperationCanceledException or ObjectDisposedException)
        {
            // Already gone; nothing to tell it.
        }
    }

    private static async Task Quietly(Task pump)
    {
        try
        {
            await pump.ConfigureAwait(false);
        }
        catch (Exception error) when (error is WebSocketException or OperationCanceledException or ObjectDisposedException)
        {
        }
    }
}
