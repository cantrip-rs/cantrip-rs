using System.Collections.Concurrent;
using System.Net.WebSockets;
using System.Threading.Channels;
using Cantrip.Protocol.InPerson;
using MessagePack;

namespace Cantrip.Client.InPerson;

/// <summary>
/// An in-person device's line to the relay (docs/in-person.md): the <c>/inperson/socket</c> WebSocket, which a tablet takes its
/// public feed from and sends its taps down, and which a phone takes the taps routed to it from. The same shape as
/// <see cref="WebSocketTransport"/>, and for the same reason: a reader task fills a queue that <see cref="Receive"/>
/// drains and a writer task empties a channel that <see cref="Send"/> fills, so the page above it polls on a timer and
/// stays single-threaded. It works in the browser, where <see cref="ClientWebSocket"/> is the browser's own WebSocket.
/// <para>
/// It reconnects nothing. A dropped line shows as <see cref="IsConnected"/> going false with <see cref="LastError"/> set;
/// a new connection's <see cref="DeviceHelloMessage"/> brings the resume id and proof the device was welcomed with
/// (<see cref="DeviceWelcomeMessage"/>) to take its place back.
/// </para>
/// </summary>
public sealed class DeviceConnection : IDisposable
{
    /// <summary>The largest message the relay is believed: a public feed's state is a whole board.</summary>
    private const int MaxMessage = 4 * 1024 * 1024;

    private readonly ClientWebSocket _socket = new();
    private readonly ConcurrentQueue<DeviceServerMessage> _inbound = new();
    private readonly Channel<byte[]> _outbound = Channel.CreateUnbounded<byte[]>();
    private readonly CancellationTokenSource _shutdown = new();

    private volatile bool _connected;
    private volatile string? _lastError;

    private DeviceConnection()
    {
    }

    public bool IsConnected => _connected;

    /// <summary>Why the connection ended, or null while it is healthy.</summary>
    public string? LastError => _lastError;

    /// <summary>
    /// The relay's address for a page served by an in-person server - the standalone one, or the bot's site behind its public
    /// URL: the <c>inperson/socket</c> beside the page, on its own host and port (<c>ws://</c> for an <c>http://</c> page,
    /// <c>wss://</c> for an <c>https://</c> one).
    /// </summary>
    public static Uri AddressFor(Uri page)
    {
        ArgumentNullException.ThrowIfNull(page);
        string path = page.AbsolutePath;
        var address = new UriBuilder(page)
        {
            Scheme = page.Scheme == Uri.UriSchemeHttps ? "wss" : "ws",
            Path = path[..(path.LastIndexOf('/') + 1)] + "inperson/socket",
            Query = string.Empty,
            Fragment = string.Empty,
        };
        return address.Uri;
    }

    /// <summary>Connects to the relay and starts pumping. The device's first message is its hello (<see cref="Hello"/>).</summary>
    public static async Task<DeviceConnection> ConnectAsync(Uri address, CancellationToken cancellation = default)
    {
        ArgumentNullException.ThrowIfNull(address);

        var connection = new DeviceConnection();
        try
        {
            await connection._socket.ConnectAsync(address, cancellation).ConfigureAwait(false);
        }
        catch
        {
            connection.Dispose();
            throw;
        }

        connection._connected = true;

        // Deliberately not awaited: these run for the life of the connection.
        _ = Task.Run(connection.ReadLoopAsync, CancellationToken.None);
        _ = Task.Run(connection.WriteLoopAsync, CancellationToken.None);
        return connection;
    }

    /// <summary>
    /// The hello a device opens with: a phone's seat <paramref name="ticket"/>, a tablet's <paramref name="device"/>
    /// token, or the resume id and proof of an earlier welcome, which the relay prefers when it still knows them; or a
    /// home-screen app's first-launch <paramref name="reentry"/> code.
    /// </summary>
    public static DeviceHelloMessage Hello(string? ticket = null, string? device = null, string? resumeId = null, byte[]? proof = null, string? reentry = null) => new()
    {
        Ticket = ticket,
        Device = device,
        ResumeId = resumeId,
        Proof = proof,
        Reentry = reentry,
    };

    public void Send(DeviceClientMessage message)
    {
        ArgumentNullException.ThrowIfNull(message);
        if (!_connected)
        {
            return;
        }

        _outbound.Writer.TryWrite(MessagePackSerializer.Serialize(message));
    }

    /// <summary>Everything the relay has sent since the last call, in order.</summary>
    public IReadOnlyList<DeviceServerMessage> Receive()
    {
        if (_inbound.IsEmpty)
        {
            return [];
        }

        List<DeviceServerMessage> messages = [];
        while (_inbound.TryDequeue(out DeviceServerMessage? message))
        {
            messages.Add(message);
        }

        return messages;
    }

    private async Task ReadLoopAsync()
    {
        var buffer = new byte[16 * 1024];
        try
        {
            // Until the close is read, not while the socket says it is open: in the browser it says closed as soon as the
            // close arrives, with the last messages before it - why a device was refused - still to be read.
            while (!_shutdown.IsCancellationRequested && _socket.State is not (WebSocketState.Aborted or WebSocketState.None))
            {
                using var frame = new MemoryStream();
                WebSocketReceiveResult result;
                do
                {
                    result = await _socket.ReceiveAsync(new ArraySegment<byte>(buffer), _shutdown.Token).ConfigureAwait(false);
                    if (result.MessageType == WebSocketMessageType.Close)
                    {
                        Fault("the relay closed the connection.");
                        return;
                    }

                    if (result.MessageType != WebSocketMessageType.Binary || frame.Length + result.Count > MaxMessage)
                    {
                        Fault("the relay sent an invalid or oversized frame.");
                        return;
                    }

                    frame.Write(buffer, 0, result.Count);
                }
                while (!result.EndOfMessage);

                if (frame.Length == 0)
                {
                    continue;
                }

                _inbound.Enqueue(MessagePackSerializer.Deserialize<DeviceServerMessage>(frame.ToArray()));
            }
        }
        catch (OperationCanceledException)
        {
            // Expected on disposal.
        }
        catch (Exception error)
        {
            Fault(error.Message);
        }
        finally
        {
            _connected = false;
            _outbound.Writer.TryComplete();
        }
    }

    private async Task WriteLoopAsync()
    {
        try
        {
            await foreach (byte[] payload in _outbound.Reader.ReadAllAsync(_shutdown.Token).ConfigureAwait(false))
            {
                if (_socket.State != WebSocketState.Open)
                {
                    return;
                }

                await _socket.SendAsync(payload, WebSocketMessageType.Binary, endOfMessage: true, _shutdown.Token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception error)
        {
            Fault(error.Message);
        }
    }

    private void Fault(string reason)
    {
        _lastError ??= reason;
        _connected = false;
        _outbound.Writer.TryComplete();
        _socket.Abort();
    }

    public void Dispose()
    {
        _connected = false;
        _outbound.Writer.TryComplete();
        try
        {
            _shutdown.Cancel();
        }
        catch (ObjectDisposedException)
        {
        }

        _socket.Dispose();
        _shutdown.Dispose();
    }
}
