using System.Net;
using System.Net.WebSockets;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using MessagePack;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;

namespace Cantrip.Server.InPerson;

/// <summary>
/// The relay's socket, <c>/inperson/socket</c> (docs/in-person.md): every device's line to an <see cref="InPersonRelay"/>,
/// open to the network. Each connection's first message is its hello, which says which table's relay it is for - a site
/// may host several (<see cref="InPersonTables"/>); the relay says what is sent back.
/// </summary>
public static class InPersonSite
{
    /// <summary>Where the relay's socket is: beside the page, as <c>DeviceConnection.AddressFor</c> finds it.</summary>
    public const string SocketRoute = "/inperson/socket";

    /// <summary>The largest message a device is believed: a tap or a hello is a few bytes.</summary>
    private const int MaxMessage = 16 * 1024;

    /// <summary>How long a new connection has to say who it is.</summary>
    private static readonly TimeSpan HelloPatience = TimeSpan.FromSeconds(30);

    /// <summary>Serves <paramref name="relay"/> at <see cref="SocketRoute"/>. The app must already use WebSockets.</summary>
    public static IEndpointRouteBuilder MapInPerson(this IEndpointRouteBuilder app, InPersonRelay relay)
    {
        ArgumentNullException.ThrowIfNull(relay);
        return app.MapInPerson(_ => relay);
    }

    /// <summary>
    /// Serves whichever relay <paramref name="relay"/> says at <see cref="SocketRoute"/>: none - before a table is started -
    /// turns a device away to try again. The app must already use WebSockets.
    /// </summary>
    public static IEndpointRouteBuilder MapInPerson(this IEndpointRouteBuilder app, Func<InPersonRelay?> relay)
    {
        ArgumentNullException.ThrowIfNull(relay);
        return app.MapInPerson(_ => relay());
    }

    /// <summary>
    /// Serves the relay each connection's hello names (<paramref name="relayFor"/>, null for none) at
    /// <see cref="SocketRoute"/>, to at most <paramref name="mostConnections"/> connections at once - the tables' phones and
    /// tablets, and room for reloads. The app must already use WebSockets.
    /// </summary>
    public static IEndpointRouteBuilder MapInPerson(this IEndpointRouteBuilder app, Func<DeviceHelloMessage, InPersonRelay?> relayFor, int mostConnections = 16)
    {
        ArgumentNullException.ThrowIfNull(app);
        ArgumentNullException.ThrowIfNull(relayFor);
        var slots = new SemaphoreSlim(mostConnections, mostConnections);

        // A device's line ends as the server stops, rather than holding its shutdown until the device goes.
        CancellationToken stopping = app.ServiceProvider.GetRequiredService<IHostApplicationLifetime>().ApplicationStopping;
        app.Map(SocketRoute, async context =>
        {
            if (!context.WebSockets.IsWebSocketRequest)
            {
                context.Response.StatusCode = StatusCodes.Status400BadRequest;
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
                await ServeAsync(relayFor, socket, ended.Token).ConfigureAwait(false);
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

    /// <summary>Serves one accepted socket until either side ends it, for the relay its hello names.</summary>
    internal static async Task ServeAsync(Func<DeviceHelloMessage, InPersonRelay?> relayFor, WebSocket socket, CancellationToken aborted)
    {
        DeviceClientMessage? first;
        using (var patience = CancellationTokenSource.CreateLinkedTokenSource(aborted))
        {
            patience.CancelAfter(HelloPatience);
            first = await ReceiveAsync(socket, new byte[4 * 1024], patience.Token).ConfigureAwait(false);
        }

        if (first is null) return;
        InPersonRelay? relay = first is DeviceHelloMessage hello ? relayFor(hello) : null;
        if (relay is null)
        {
            await RefuseAsync(socket, first is DeviceHelloMessage
                ? new DeviceErrorMessage { Code = ErrorCode.MembershipRefused, Detail = "That code is not for a table here, or no longer works: scan the setup page's again." }
                : new DeviceErrorMessage { Code = ErrorCode.MembershipRequired, Detail = "A device says who it is first." }).ConfigureAwait(false);
            return;
        }

        await ServeAsync(relay, socket, aborted, first).ConfigureAwait(false);
    }

    /// <summary>Serves one accepted socket until either side ends it; <paramref name="first"/> is a message already read.</summary>
    internal static async Task ServeAsync(InPersonRelay relay, WebSocket socket, CancellationToken aborted, DeviceClientMessage? first = null)
    {
        RelayDevice device = relay.Attach();
        using var stop = CancellationTokenSource.CreateLinkedTokenSource(aborted);
        Task writer = WriteAsync(device, socket, stop.Token);
        try
        {
            if (first is not null) relay.Receive(device, first);
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

    /// <summary>Tells a connection no relay here is for it, and closes it.</summary>
    private static async Task RefuseAsync(WebSocket socket, DeviceErrorMessage error)
    {
        try
        {
            using var patience = new CancellationTokenSource(TimeSpan.FromSeconds(2));
            await socket.SendAsync(MessagePackSerializer.Serialize<DeviceServerMessage>(error), WebSocketMessageType.Binary, endOfMessage: true, patience.Token).ConfigureAwait(false);
            await socket.CloseOutputAsync(WebSocketCloseStatus.NormalClosure, "refused", patience.Token).ConfigureAwait(false);
        }
        catch (Exception failure) when (failure is OperationCanceledException or WebSocketException)
        {
        }
    }

    /// <summary>The next message on the socket, or null once it closes, fails or sends what is no message.</summary>
    private static async Task<DeviceClientMessage?> ReceiveAsync(WebSocket socket, byte[] buffer, CancellationToken stop)
    {
        try
        {
            using var frame = new MemoryStream();
            WebSocketReceiveResult result;
            do
            {
                result = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), stop).ConfigureAwait(false);
                if (result.MessageType == WebSocketMessageType.Close) return null;
                if (result.MessageType != WebSocketMessageType.Binary || frame.Length + result.Count > MaxMessage) return null;
                frame.Write(buffer, 0, result.Count);
            }
            while (!result.EndOfMessage);

            return MessagePackSerializer.Deserialize<DeviceClientMessage>(frame.ToArray());
        }
        catch (Exception error) when (error is OperationCanceledException or WebSocketException or MessagePackSerializationException)
        {
            return null;
        }
    }

    private static async Task ReadAsync(InPersonRelay relay, RelayDevice device, WebSocket socket, CancellationToken stop)
    {
        var buffer = new byte[4 * 1024];
        while (socket.State == WebSocketState.Open && !stop.IsCancellationRequested)
        {
            if (await ReceiveAsync(socket, buffer, stop).ConfigureAwait(false) is not { } message) return;
            relay.Receive(device, message);
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
