using System.Runtime.InteropServices.JavaScript;
using Cantrip.Client.InPerson;
using Cantrip.Protocol.InPerson;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// A phone's line to the in-person relay (docs/in-person.md), beside its ordinary seat at the table: the taps made on the
/// tablets that were routed to this seat come in on it, and whether each was used goes back. The game itself - the view,
/// every prompt and answer - goes over the page's ordinary connection, as the remote client's does.
/// <para>
/// The line is the relay's <c>/inperson</c> socket on the page's own host (<see cref="DeviceConnection"/>), opened with the
/// seat's ticket, or with the resume id and proof of the welcome this tab had before, kept in the tab's session storage
/// so a reload takes the phone's place back. A dropped line is opened again every few seconds.
/// </para>
/// </summary>
public sealed class PhoneLink
{
    private static readonly TimeSpan RetryEvery = TimeSpan.FromSeconds(3);
    private const string ResumeKey = "cantrip-inperson-resume";
    private const string ProofKey = "cantrip-inperson-proof";

    private readonly Action<SeatTapMessage, Action<bool>> _tapped;
    private DeviceConnection? _connection;
    private bool _connecting;
    private DateTime _nextTry = DateTime.MinValue;

    /// <param name="tapped">Hands the page a tap routed to this seat, with what to call once it is settled: used or not.</param>
    public PhoneLink(Action<SeatTapMessage, Action<bool>> tapped)
    {
        _tapped = tapped;
    }

    /// <summary>Why the relay last refused this phone, or null.</summary>
    public string? Refused { get; private set; }

    /// <summary>
    /// Takes in whatever the relay has sent since the last pump, and opens the line again if it is down. Called on the
    /// page's thread, on its timer.
    /// </summary>
    public void Pump()
    {
        if (_connection is { } connection)
        {
            foreach (DeviceServerMessage message in connection.Receive())
            {
                Take(message);
            }

            if (connection.IsConnected)
            {
                return;
            }

            connection.Dispose();
            _connection = null;
            _nextTry = DateTime.UtcNow + RetryEvery;
        }

        if (!_connecting && DateTime.UtcNow >= _nextTry)
        {
            _ = ConnectAsync();
        }
    }

    private async Task ConnectAsync()
    {
        _connecting = true;
        try
        {
            string page = JSHost.GlobalThis.GetPropertyAsJSObject("location")?.GetPropertyAsString("href") ?? string.Empty;
            if (!Uri.TryCreate(page, UriKind.Absolute, out Uri? address) || address.Scheme is not ("http" or "https"))
            {
                _nextTry = DateTime.MaxValue;
                return;
            }

            DeviceConnection connection = await DeviceConnection.ConnectAsync(DeviceConnection.AddressFor(address));
            string? resume = PhoneFeedback.Recall(ResumeKey);
            byte[]? proof = PhoneFeedback.Recall(ProofKey) is { Length: > 0 } hex ? Convert.FromHexString(hex) : null;
            connection.Send(DeviceConnection.Hello(ticket: ClientConfig.Ticket, resumeId: resume, proof: proof));
            _connection = connection;
        }
        catch (Exception error) when (error is System.Net.WebSockets.WebSocketException or InvalidOperationException or FormatException or JSException)
        {
            // No relay here yet, or not reachable: tried again in a moment.
            _nextTry = DateTime.UtcNow + RetryEvery;
        }
        finally
        {
            _connecting = false;
        }
    }

    /// <summary>One message from the relay.</summary>
    public void Take(DeviceServerMessage message)
    {
        switch (message)
        {
            case DeviceWelcomeMessage welcome:
                Refused = null;
                PhoneFeedback.Keep(ResumeKey, welcome.ResumeId);
                PhoneFeedback.Keep(ProofKey, Convert.ToHexString(welcome.Proof));
                break;

            case SeatTapMessage tap:
                DeviceConnection? to = _connection;
                _tapped(tap, used => to?.Send(new TapResultMessage { TapId = tap.TapId, Used = used }));
                break;

            case DeviceErrorMessage error:
                Refused = error.Detail;
                break;

            default:
                break;
        }
    }
}

/// <summary>What a phone keeps and does besides drawing: its welcome for the tab, and a short buzz where the browser allows one.</summary>
internal static partial class PhoneFeedback
{
    /// <summary>A short buzz, for a tap on the table that did nothing. Nothing where the browser has no vibration (iOS).</summary>
    public static void Buzz()
    {
        try
        {
            Vibrate(80);
        }
        catch (JSException)
        {
            // No vibration here, or not allowed yet: the line on screen says it alone.
        }
    }

    /// <summary>Keeps a value for this tab alone (session storage), so a reload finds it and no other tab does.</summary>
    public static void Keep(string key, string value)
    {
        try
        {
            SessionSet(key, value);
        }
        catch (JSException)
        {
            // Storage refused (a private window): the phone's ticket takes its place back instead.
        }
    }

    /// <summary>A value kept for this tab, or null.</summary>
    public static string? Recall(string key)
    {
        try
        {
            return SessionGet(key) is { Length: > 0 } value ? value : null;
        }
        catch (JSException)
        {
            return null;
        }
    }

    [JSImport("globalThis.cantripVibrate")]
    private static partial void Vibrate(int milliseconds);

    [JSImport("globalThis.cantripSessionSet")]
    private static partial void SessionSet(string key, string value);

    [JSImport("globalThis.cantripSessionGet")]
    private static partial string? SessionGet(string key);
}
