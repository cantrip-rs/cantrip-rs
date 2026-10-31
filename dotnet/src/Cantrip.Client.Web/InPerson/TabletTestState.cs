using System.Runtime.InteropServices.JavaScript;
using System.Text.Json;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using MessagePack;
using Microsoft.UI.Xaml.Media;
using Windows.Foundation;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// What a browser driven by Playwright reads from a tablet, and the feed a test drives one with. Not a player feature, and
/// none of it runs unless the page is asked for it with <c>?test</c>.
/// <para>
/// <c>globalThis.cantripTest</c> is the tablet's state as JSON, ten times a second, in the remote page's schema
/// (<c>MainPage.Test.cs</c>): <c>ready</c>, <c>busy</c> and <c>doing</c>, every card drawn by <c>name</c> with its box on
/// the page, and each player's box - so the same helpers find and tap a card. A card's <c>place</c> is <c>battlefield</c>
/// for the half at the tablet's bottom edge, <c>opponent-battlefield</c> for the turned one, or <c>stack</c>, and its
/// <c>seat</c> says whose half it is in. A tablet adds <c>layout</c>, <c>screen</c> (<c>phase</c> before the game,
/// <c>board</c> after), <c>phaseLine</c>, <c>held</c> (the faces of a card held up) and <c>taps</c>, the taps it has sent.
/// </para>
/// <para>
/// A tablet opened with <c>?test</c> and no device token reads its device messages from the test instead of a relay:
/// <c>globalThis.cantripTabletFeed(base64)</c> hands it one MessagePack <see cref="DeviceServerMessage"/>
/// (InPerson.js keeps them until the page takes them), and the taps it would send the relay are kept in <c>taps</c>.
/// </para>
/// </summary>
public sealed partial class TabletPage
{
    [JSImport("globalThis.cantripTakeTabletFeed")]
    private static partial string TakeTabletFeed();

    private DispatcherTimer? _testTimer;

    /// <summary>The taps sent, newest last, for a test to read.</summary>
    private readonly List<TableTapMessage> _sent = [];

    /// <summary>Whether this tablet is fed by a test rather than a relay.</summary>
    private static bool FedByTest => ClientConfig.Test && InPersonConfig.Device is null;

    /// <summary>The device messages a test has handed the page since it last looked.</summary>
    private static IReadOnlyList<DeviceServerMessage> TestFeed()
    {
        string taken = TakeTabletFeed();
        return taken.Length == 0
            ? []
            : [.. taken.Split('\n', StringSplitOptions.RemoveEmptyEntries).Select(line => MessagePackSerializer.Deserialize<DeviceServerMessage>(Convert.FromBase64String(line)))];
    }

    private void StartTestState()
    {
        if (!ClientConfig.Test || _testTimer is not null)
        {
            return;
        }

        _testTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(100) };
        _testTimer.Tick += (_, _) => JSHost.GlobalThis.SetProperty("cantripTest", TestState());
        _testTimer.Start();
    }

    private string TestState()
    {
        using var buffer = new MemoryStream();
        using (var json = new Utf8JsonWriter(buffer))
        {
            bool board = _client.View is not null;
            List<string> busy = [];
            if (CardArt.Arriving)
            {
                busy.Add("art");
            }

            if (DateTime.UtcNow - Faces.LastDrawn < TimeSpan.FromMilliseconds(400))
            {
                busy.Add("faces");
            }

            if (_flights.Flying.Count > 0)
            {
                busy.Add("flights");
            }

            if (Boards().Any(each => each.Moving))
            {
                busy.Add("slides");
            }

            if (_needsRedraw)
            {
                busy.Add("redraw");
            }

            json.WriteStartObject();
            json.WriteBoolean("ready", _phaseScreen.Visibility == Visibility.Visible || board);
            json.WriteBoolean("busy", busy.Count > 0);
            json.WriteString("doing", string.Join(" ", busy));
            json.WriteBoolean("over", _client.IsOver);
            json.WriteString("layout", _side == 0 ? "shared" : $"side {_side}");
            json.WriteString("screen", board ? "board" : "phase");
            json.WriteString("phase", _phase?.Phase.ToString() ?? string.Empty);
            json.WriteString("phaseLine", _phaseTexts[^1].Line.Text);
            json.WriteNumber("seat", _client.Seat);
            json.WriteNumber("held", _inspector.FacesHeld + _turnedInspector.FacesHeld);
            json.WriteString("tooltip", _tooltip.Showing.Length > 0 ? _tooltip.Showing : _turnedTooltip.Showing);
            json.WriteNumber("flown", _flights.Launched);
            json.WriteString("lives", _lives.Text);
            json.WriteString("art", ClientConfig.ArtTemplate);
            _fullScreen.WriteTest(json, _root);

            json.WriteStartArray("cards");
            if (board)
            {
                foreach (CardBoardElement row in Boards())
                {
                    if (row.XamlRoot is null || row.ActualWidth <= 0 || row.Visibility != Visibility.Visible)
                    {
                        continue;
                    }

                    TabletHalf? half = _halves.FirstOrDefault(each => each.Board == row || each.Pile.Board == row);
                    string place = half is null ? (row == _stack.Board ? "stack" : "opponent-stack")
                        : half.Pile.Board == row ? (half.Turned ? "opponent-" : string.Empty) + (half.Pile.Open?.Zone == ZoneKindView.Exile ? "exile" : "graveyard")
                        : half.Turned ? "opponent-battlefield" : "battlefield";
                    GeneralTransform toPage = row.TransformToVisual(_root);
                    foreach ((ObjectView card, Rect box, bool visible, _, _) in row.Targets())
                    {
                        json.WriteStartObject();
                        json.WriteString("name", card.Known || card.FaceDown ? card.Name : string.Empty);
                        json.WriteString("faceDownName", card.FaceDownName);
                        json.WriteBoolean("visible", visible);
                        json.WriteString("row", string.Empty);
                        json.WriteString("place", place);
                        json.WriteNumber("seat", half?.Seat ?? 0);
                        json.WriteNumber("handle", card.Handle.Value);
                        json.WriteBoolean("tapped", card.Tapped);
                        WriteBox(json, toPage.TransformBounds(box));
                        if (_flights.FlewFrom.TryGetValue(card.Handle, out CardFlights.Pose from))
                        {
                            json.WriteStartObject("flewFrom");
                            json.WriteNumber("x", Math.Round(from.X, 1));
                            json.WriteNumber("y", Math.Round(from.Y, 1));
                            json.WriteEndObject();
                        }

                        json.WriteEndObject();
                    }
                }
            }

            json.WriteEndArray();

            json.WriteStartArray("players");
            if (board)
            {
                foreach (TabletHalf half in _halves)
                {
                    FrameworkElement avatar = half.Rail.Avatar;
                    if (avatar.XamlRoot is null || avatar.ActualWidth <= 0)
                    {
                        continue;
                    }

                    json.WriteStartObject();
                    json.WriteNumber("seat", half.Seat);
                    json.WriteString("name", TabletLayout.NameOf(_client, half.Seat));
                    json.WriteNumber("life", _client.View!.Players.FirstOrDefault(player => player.Seat == half.Seat)?.Life ?? 0);
                    json.WriteBoolean("turned", half.Turned);
                    WriteBox(json, avatar.TransformToVisual(_root).TransformBounds(new Rect(0, 0, avatar.ActualWidth, avatar.ActualHeight)));

                    // The piles a finger opens: the graveyard, and the exile button while the seat has cards in exile.
                    json.WriteStartArray("piles");
                    FrameworkElement graveyard = half.Rail.GraveyardPile;
                    if (graveyard.ActualWidth > 0)
                    {
                        WritePile(json, "graveyard", graveyard, half.Pile.Open?.Zone == ZoneKindView.Graveyard);
                    }

                    foreach ((string text, _, _, DockButton button) in half.Rail.ZoneButtonsShown())
                    {
                        if (text.StartsWith("Exile", StringComparison.Ordinal) && button.ActualWidth > 0)
                        {
                            WritePile(json, "exile", button, half.Pile.Open?.Zone == ZoneKindView.Exile);
                        }
                    }

                    json.WriteEndArray();
                    json.WriteStartArray("mana");
                    json.WriteEndArray();
                    json.WriteEndObject();
                }
            }

            json.WriteEndArray();

            json.WriteStartArray("taps");
            foreach (TableTapMessage tap in _sent)
            {
                json.WriteStartObject();
                json.WriteNumber("object", tap.Object.Value);
                json.WriteNumber("player", tap.Player);
                json.WriteNumber("sequence", tap.Sequence);
                json.WriteEndObject();
            }

            json.WriteEndArray();
            json.WriteEndObject();
        }

        return System.Text.Encoding.UTF8.GetString(buffer.ToArray());
    }

    private void WritePile(Utf8JsonWriter json, string kind, FrameworkElement element, bool open)
    {
        json.WriteStartObject();
        json.WriteString("kind", kind);
        json.WriteBoolean("open", open);
        WriteBox(json, element.TransformToVisual(_root).TransformBounds(new Rect(0, 0, element.ActualWidth, element.ActualHeight)));
        json.WriteEndObject();
    }

    private static void WriteBox(Utf8JsonWriter json, Rect box)
    {
        json.WriteNumber("x", Math.Round(box.X + (box.Width / 2), 1));
        json.WriteNumber("y", Math.Round(box.Y + (box.Height / 2), 1));
        json.WriteNumber("w", Math.Round(box.Width, 1));
        json.WriteNumber("h", Math.Round(box.Height, 1));
    }
}
