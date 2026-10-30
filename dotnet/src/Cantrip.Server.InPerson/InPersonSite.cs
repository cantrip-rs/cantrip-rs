using System.Net;
using System.Net.WebSockets;
using Cantrip.Protocol.InPerson;
using MessagePack;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;

namespace Cantrip.Server.InPerson;

/// <summary>
/// The relay's socket, <c>/inperson</c> (docs/in-person.md): every device's line to an <see cref="InPersonRelay"/>, open
/// to the local network. Each connection's first message is its hello; the relay says what is sent back.
/// </summary>
public static class InPersonSite
{
    public const string Route = "/inperson";

    /// <summary>The largest message a device is believed: a tap or a hello is a few bytes.</summary>
    private const int MaxMessage = 16 * 1024;

    /// <summary>How many devices may be connected at once: a table's phones and tablets, and room for reloads.</summary>
    private const int MostConnections = 16;

    /// <summary>Serves <paramref name="relay"/> at <see cref="Route"/>. The app must already use WebSockets.</summary>
    public static IEndpointRouteBuilder MapInPerson(this IEndpointRouteBuilder app, InPersonRelay relay)
    {
        ArgumentNullException.ThrowIfNull(relay);
        return app.MapInPerson(() => relay);
    }

    /// <summary>
    /// Serves whichever relay <paramref name="relay"/> says at <see cref="Route"/>: none - before a table is started - turns
    /// a device away to try again. The app must already use WebSockets.
    /// </summary>
    public static IEndpointRouteBuilder MapInPerson(this IEndpointRouteBuilder app, Func<InPersonRelay?> relay)
    {
        ArgumentNullException.ThrowIfNull(app);
        ArgumentNullException.ThrowIfNull(relay);
        var slots = new SemaphoreSlim(MostConnections, MostConnections);

        // A device's line ends as the server stops, rather than holding its shutdown until the device goes.
        CancellationToken stopping = app.ServiceProvider.GetRequiredService<IHostApplicationLifetime>().ApplicationStopping;
        app.Map(Route, async context =>
        {
            if (!context.WebSockets.IsWebSocketRequest)
            {
                context.Response.StatusCode = StatusCodes.Status400BadRequest;
                return;
            }

            if (relay() is not { } serving)
            {
                context.Response.StatusCode = StatusCodes.Status503ServiceUnavailable;
                return;
            }

            if (!slots.Wait(0))
            {
                context.Response.StatusCode = StatusCodes.Status409Conflict;
                return;
            }

            try
            {
                using WebSocket socket = await context.WebSockets.AcceptWebSocketAsync().ConfigureAwait(false);
                using var ended = CancellationTokenSource.CreateLinkedTokenSource(context.RequestAborted, stopping);
                await ServeAsync(serving, socket, ended.Token).ConfigureAwait(false);
            }
            finally
            {
                slots.Release();
            }
        });
        return app;
    }

    /// <summary>Whether a request came from this machine: what the setup page, whose QR codes hold credentials, is served to.</summary>
    public static bool IsLoopback(HttpContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        return context.Connection.RemoteIpAddress is { } address && IPAddress.IsLoopback(address);
    }

    /// <summary>Serves one accepted socket until either side ends it.</summary>
    internal static async Task ServeAsync(InPersonRelay relay, WebSocket socket, CancellationToken aborted)
    {
        RelayDevice device = relay.Attach();
        using var stop = CancellationTokenSource.CreateLinkedTokenSource(aborted);
        Task writer = WriteAsync(device, socket, stop.Token);
        try
        {
            await ReadAsync(relay, device, socket, stop.Token).ConfigureAwait(false);
        }
        finally
        {
            relay.Detach(device);
            try
            {
                await writer.ConfigureAwait(false);
            }
            finally
            {
                await stop.CancelAsync().ConfigureAwait(false);
            }
        }
    }

    private static async Task ReadAsync(InPersonRelay relay, RelayDevice device, WebSocket socket, CancellationToken stop)
    {
        var buffer = new byte[4 * 1024];
        try
        {
            while (socket.State == WebSocketState.Open && !stop.IsCancellationRequested)
            {
                using var frame = new MemoryStream();
                WebSocketReceiveResult result;
                do
                {
                    result = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), stop).ConfigureAwait(false);
                    if (result.MessageType == WebSocketMessageType.Close) return;
                    if (result.MessageType != WebSocketMessageType.Binary || frame.Length + result.Count > MaxMessage) return;
                    frame.Write(buffer, 0, result.Count);
                }
                while (!result.EndOfMessage);

                DeviceClientMessage message;
                try
                {
                    message = MessagePackSerializer.Deserialize<DeviceClientMessage>(frame.ToArray());
                }
                catch (MessagePackSerializationException)
                {
                    return;
                }

                relay.Receive(device, message);
            }
        }
        catch (Exception error) when (error is OperationCanceledException or WebSocketException)
        {
            // The device went, or the server is stopping.
        }
    }

    private static async Task WriteAsync(RelayDevice device, WebSocket socket, CancellationToken stop)
    {
        try
        {
            await foreach (DeviceServerMessage message in device.Outbox.ReadAllAsync(stop).ConfigureAwait(false))
            {
                if (socket.State != WebSocketState.Open) return;
                await socket.SendAsync(MessagePackSerializer.Serialize(message), WebSocketMessageType.Binary, endOfMessage: true, stop).ConfigureAwait(false);
            }

            // The relay is done with the device - refused, or its place taken: the socket is closed behind what it was told.
            if (socket.State == WebSocketState.Open)
            {
                using var patience = new CancellationTokenSource(TimeSpan.FromSeconds(2));
                await socket.CloseOutputAsync(WebSocketCloseStatus.NormalClosure, "closed by the relay", patience.Token).ConfigureAwait(false);
            }
        }
        catch (Exception error) when (error is OperationCanceledException or WebSocketException)
        {
        }
    }
}
