using System.Runtime.InteropServices.JavaScript;
using Cantrip.Client.InPerson;
using Cantrip.Protocol.InPerson;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// A phone's line to the in-person relay (docs/in-person.md), beside its ordinary seat at the table: the taps made on the
/// tablets that were routed to this seat come in on it, and whether each was used goes back. The game itself - the view,
/// every prompt and answer - goes over the page's ordinary connection, as the remote client's does.
/// <para>
/// The line is the relay's <c>/inperson/socket</c> beside the page (<see cref="DeviceConnection"/>), opened with the
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

    /// <summary>Whether the relay has welcomed this phone on the line open now.</summary>
    private bool _welcomed;

    /// <summary>Whether another device has taken this phone's place: the line is not opened again to take it back.</summary>
    private bool _displaced;

    /// <param name="tapped">Hands the page a tap routed to this seat, with what to call once it is settled: used or not.</param>
    public PhoneLink(Action<SeatTapMessage, Action<bool>> tapped)
    {
        _tapped = tapped;
    }

    /// <summary>Why the relay last refused this phone, or null.</summary>
    public string? Refused { get; private set; }

    /// <summary>A re-entry code the relay sent (<see cref="AskReentry"/>), for the page to put in its address.</summary>
    public event Action<string>? ReentryCode;

    /// <summary>
    /// Asks the relay for a re-entry code, for the home-screen app this page is about to be added as (InPerson/FullScreen.cs):
    /// whether the ask went out, which it cannot before the relay has welcomed the phone.
    /// </summary>
    public bool AskReentry()
    {
        if (_connection is not { IsConnected: true } connection || !_welcomed)
        {
            return false;
        }

        connection.Send(new ReentryRequestMessage());
        return true;
    }

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
            _welcomed = false;
            _nextTry = DateTime.UtcNow + RetryEvery;
        }

        if (!_connecting && !_displaced && DateTime.UtcNow >= _nextTry)
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
            // A home-screen app's first launch has nothing but the re-entry code it was added with.
            connection.Send(DeviceConnection.Hello(ticket: ClientConfig.Ticket, resumeId: resume, proof: proof, reentry: FullScreenBridge.Reentry));
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
                _welcomed = true;
                PhoneFeedback.Keep(ResumeKey, welcome.ResumeId);
                PhoneFeedback.Keep(ProofKey, Convert.ToHexString(welcome.Proof));

                // Welcomed by a re-entry code, as a home-screen app opened for the first time: the ticket it was given
                // for the table is kept in the app's own storage, and the page loads again to take its seat with it.
                if (welcome.Ticket is { Length: > 0 } ticket)
                {
                    FullScreenBridge.KeepTicket(ticket);
                }

                break;

            case ReentryCodeMessage code:
                ReentryCode?.Invoke(code.Code);
                break;

            case SeatTapMessage tap:
                DeviceConnection? to = _connection;
                _tapped(tap, used => to?.Send(new TapResultMessage { TapId = tap.TapId, Used = used }));
                break;

            case DeviceErrorMessage error:
                Refused = error.Detail;

                // Refused once welcomed: another device - this phone's home-screen app, a code scanned again - has taken the
                // place, and opening the line again would only take it back from that one.
                _displaced |= _welcomed && error.Code == Cantrip.Protocol.ErrorCode.MembershipRefused;
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
