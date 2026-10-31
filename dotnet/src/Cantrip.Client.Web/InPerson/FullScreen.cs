using System.Globalization;
using System.Runtime.InteropServices.JavaScript;
using System.Text.Json;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace Cantrip.Client.Web.InPerson;

/// <summary>What an in-person page's Fullscreen button does where the page is.</summary>
public enum FullScreenState
{
    /// <summary>Nothing to do: no Fullscreen API, or an app opened from the Home Screen, which has the screen already.</summary>
    None,

    /// <summary>The Fullscreen API is here and the page is not full screen.</summary>
    Enter,

    /// <summary>The page is full screen.</summary>
    Exit,

    /// <summary>An iPhone's browser, with no Fullscreen API: the button explains Add to Home Screen.</summary>
    HomeScreen,
}

/// <summary>
/// Full screen as the browser has it (FullScreen.js, docs/in-person.md "Full screen"): the Fullscreen API, an orientation
/// lock, the safe-area insets, and the re-entry code a phone's home-screen app comes back with. Every call is safe on a
/// page without the script's support: it reads as none.
/// </summary>
internal static partial class FullScreenBridge
{
    /// <summary>The button's state, and how many times full screen has changed since the page loaded.</summary>
    public static (FullScreenState State, int Changes) Read()
    {
        try
        {
            string[] parts = State().Split(',');
            FullScreenState state = parts[0] switch
            {
                "enter" => FullScreenState.Enter,
                "exit" => FullScreenState.Exit,
                "homescreen" => FullScreenState.HomeScreen,
                _ => FullScreenState.None,
            };
            return (state, parts.Length > 1 && int.TryParse(parts[1], NumberStyles.Integer, CultureInfo.InvariantCulture, out int changes) ? changes : 0);
        }
        catch (JSException)
        {
            return (FullScreenState.None, 0);
        }
    }

    /// <summary>
    /// How far the notch, the status bar and the home indicator cover each edge of the page, in the page's pixels: zero
    /// on a screen without them.
    /// </summary>
    public static Thickness SafeArea()
    {
        try
        {
            double[] edges = [.. SafeAreaInsets().Split(',').Select(each => double.TryParse(each, NumberStyles.Float, CultureInfo.InvariantCulture, out double value) ? value : 0)];
            return edges.Length == 4 ? new Thickness(edges[3], edges[0], edges[1], edges[2]) : default;
        }
        catch (JSException)
        {
            return default;
        }
    }

    /// <summary>
    /// The re-entry code this page was opened with as a phone's home-screen app on its first launch, or null: what its
    /// hello to the relay brings, to be traded for a ticket (<see cref="KeepTicket"/>).
    /// </summary>
    public static string? Reentry =>
        JSHost.GlobalThis.GetPropertyAsString("cantripReentry") is { Length: > 0 } code ? code : null;

    /// <summary>Asks for the whole screen, from a tap: a tablet then locked to landscape, a phone upright, where allowed.</summary>
    public static void Enter(bool landscape) => Call(() => EnterFullScreen(landscape));

    public static void Exit() => Call(ExitFullScreen);

    /// <summary>Puts a phone's re-entry code in the address the icon about to be added to the Home Screen opens.</summary>
    public static void OfferHomeScreen(string code) => Call(() => OfferCode(code));

    /// <summary>A home-screen app's phone keeps the ticket its re-entry code was traded for, and loads again with it.</summary>
    public static void KeepTicket(string ticket) => Call(() => Keep(ticket));

    private static void Call(Action call)
    {
        try
        {
            call();
        }
        catch (JSException)
        {
            // The browser said no: the button stays as it was.
        }
    }

    [JSImport("globalThis.cantripFullScreenState")]
    private static partial string State();

    [JSImport("globalThis.cantripSafeArea")]
    private static partial string SafeAreaInsets();

    [JSImport("globalThis.cantripFullScreenEnter")]
    private static partial void EnterFullScreen(bool landscape);

    [JSImport("globalThis.cantripFullScreenExit")]
    private static partial void ExitFullScreen();

    [JSImport("globalThis.cantripOfferHomeScreen")]
    private static partial void OfferCode(string code);

    [JSImport("globalThis.cantripKeepTicket")]
    private static partial void Keep(string ticket);
}

/// <summary>
/// An in-person page's Fullscreen button (docs/in-person.md, "Full screen"), a small square over the page at an edge of
/// it, with the page padded clear of the notch and the home indicator. Its corners say what a tap does - turned out for
/// <b>Full screen</b>, in for <b>Exit full screen</b> while it is (its name, read aloud and in the tooltip) - following
/// the browser however full screen was left; it is not there where the browser has no
/// Fullscreen API, nor in an app opened from the Home Screen. On an iPhone's browser, which has no Fullscreen API, it
/// explains Add to Home Screen instead, having first asked the relay for the re-entry code the icon will come back with.
/// <para>
/// Lies over the whole page without taking a tap anywhere but on its button and its explanation.
/// </para>
/// </summary>
public sealed class FullScreenButton : Grid
{
    private readonly Border _button;
    private readonly Canvas _icon = new() { Width = 20, Height = 20 };
    private readonly Border _help;
    private readonly TextBlock _helpStatus;
    private readonly bool _landscape;
    private readonly Action<Thickness> _pad;
    private readonly Func<bool>? _askReentry;
    private readonly DispatcherTimer _follow = new() { Interval = TimeSpan.FromMilliseconds(250) };
    private FullScreenState _state = (FullScreenState)(-1);
    private Thickness _insets = new(-1);
    private string? _code;

    /// <param name="landscape">Whether the page is a tablet's, locked to landscape in full screen; a phone's is upright.</param>
    /// <param name="corner">Where the button sits on the page, and how far it is kept from the edges.</param>
    /// <param name="pad">Pads the page by the safe-area insets, whenever they change.</param>
    /// <param name="askReentry">A phone's: asks the relay for a re-entry code, saying whether the ask went out.</param>
    public FullScreenButton(bool landscape, (HorizontalAlignment Horizontal, VerticalAlignment Vertical, Thickness Margin) corner, Action<Thickness> pad, Func<bool>? askReentry = null)
    {
        _landscape = landscape;
        _pad = pad;
        _askReentry = askReentry;
        Grid.SetRowSpan(this, 8);
        Grid.SetColumnSpan(this, 8);
        Canvas.SetZIndex(this, 1000);
        _button = new Border
        {
            Background = Theme.Brush(Theme.WithAlpha(Theme.ControlFill, 0.85)),
            BorderBrush = Theme.Brush(Theme.ControlBorder),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(8),
            Padding = new Thickness(10),
            Margin = corner.Margin,
            HorizontalAlignment = corner.Horizontal,
            VerticalAlignment = corner.Vertical,
            Visibility = Visibility.Collapsed,
            Child = _icon,
        };
        _button.PointerPressed += (_, _) => _button.Background = Theme.Brush(Theme.ControlPressed);
        _button.PointerReleased += (_, _) => _button.Background = Theme.Brush(Theme.WithAlpha(Theme.ControlFill, 0.85));
        _button.PointerExited += (_, _) => _button.Background = Theme.Brush(Theme.WithAlpha(Theme.ControlFill, 0.85));
        _button.Tapped += (_, e) =>
        {
            e.Handled = true;
            Tap();
        };
        Children.Add(_button);

        _helpStatus = new TextBlock { FontSize = 13, Foreground = Theme.Brush(Theme.FgMuted), TextWrapping = TextWrapping.Wrap };
        var close = new DockButton("Close", primary: true, fontSize: 14) { HorizontalAlignment = HorizontalAlignment.Right };
        close.Clicked += () => _help!.Visibility = Visibility.Collapsed;
        _help = new Border
        {
            Background = Theme.Brush(Theme.Panel),
            BorderBrush = Theme.Brush(Theme.ControlBorder),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(10),
            Padding = new Thickness(18, 16, 18, 16),
            Margin = new Thickness(16),
            MaxWidth = 420,
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            Visibility = Visibility.Collapsed,
            Child = new StackPanel
            {
                Spacing = 10,
                Children =
                {
                    new TextBlock { Text = "Full screen on an iPhone", FontSize = 18, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg) },
                    new TextBlock
                    {
                        Text = "An iPhone's browser can't give a page the whole screen. Add this page to your Home Screen instead: "
                            + "tap Share (the square with the arrow), then Add to Home Screen, then Add. Open Cantrip from the Home Screen "
                            + "and it comes back to this seat, with the whole screen.",
                        FontSize = 14,
                        Foreground = Theme.Brush(Theme.FgSoft),
                        TextWrapping = TextWrapping.Wrap,
                    },
                    _helpStatus,
                    close,
                },
            },
        };
        Children.Add(_help);

        _follow.Tick += (_, _) => Follow();
        Loaded += (_, _) =>
        {
            Follow();
            _follow.Start();
        };
        Unloaded += (_, _) => _follow.Stop();
    }

    /// <summary>What the button does here now.</summary>
    public FullScreenState State => _state;

    /// <summary>Whether the explanation of Add to Home Screen is up.</summary>
    public bool Explaining => _help.Visibility == Visibility.Visible;

    /// <summary>
    /// The re-entry code the relay sent for this phone (<see cref="Cantrip.Protocol.InPerson.ReentryCodeMessage"/>): put in
    /// the page's address for the icon the player is about to add.
    /// </summary>
    public void Reentry(string code)
    {
        _code = code;
        FullScreenBridge.OfferHomeScreen(code);
        _helpStatus.Text = "This page is ready to add.";
    }

    private void Tap()
    {
        switch (_state)
        {
            case FullScreenState.Enter:
                FullScreenBridge.Enter(_landscape);
                break;
            case FullScreenState.Exit:
                FullScreenBridge.Exit();
                break;
            case FullScreenState.HomeScreen:
                _help.Visibility = Visibility.Visible;
                if (_code is null)
                {
                    bool asked = _askReentry?.Invoke() ?? false;
                    _helpStatus.Text = asked ? "Getting this page ready to add…" : "Not connected to the table yet: try again in a moment.";
                }

                break;
            default:
                break;
        }
    }

    /// <summary>Follows the browser: full screen entered or left (Escape, a swipe, the system), and the insets with it.</summary>
    private void Follow()
    {
        (FullScreenState state, _) = FullScreenBridge.Read();
        if (state != _state)
        {
            _state = state;
            _button.Visibility = state == FullScreenState.None ? Visibility.Collapsed : Visibility.Visible;
            Caption = state == FullScreenState.Exit ? "Exit full screen" : "Full screen";
            Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(_button, Caption);
            ToolTipService.SetToolTip(_button, state switch
            {
                FullScreenState.Exit => "Exit full screen",
                FullScreenState.HomeScreen => "Full screen: how, on an iPhone",
                _ => "Full screen",
            });
            DrawIcon(outward: state != FullScreenState.Exit);
            if (state != FullScreenState.HomeScreen)
            {
                _help.Visibility = Visibility.Collapsed;
            }
        }

        Thickness insets = FullScreenBridge.SafeArea();
        if (insets != _insets)
        {
            _insets = insets;
            _pad(insets);
        }
    }

    /// <summary>What the button is called now: what a tap on it does.</summary>
    public string Caption { get; private set; } = "Full screen";

    /// <summary>Four corner marks, turned out (full screen) or in (exit full screen).</summary>
    private void DrawIcon(bool outward)
    {
        const double Edge = 20;
        const double Arm = 6;
        double near = outward ? 0 : Arm;
        _icon.Children.Clear();
        foreach ((double x, double y) in ((double, double)[])[(0, 0), (Edge, 0), (Edge, Edge), (0, Edge)])
        {
            // The corner's point, and its two arms along the edges from it.
            double cx = x == 0 ? near : Edge - near;
            double cy = y == 0 ? near : Edge - near;
            double ax = x == 0 ? (outward ? Arm : 0) : (outward ? Edge - Arm : Edge);
            double ay = y == 0 ? (outward ? Arm : 0) : (outward ? Edge - Arm : Edge);
            _icon.Children.Add(new Polyline
            {
                Points = new PointCollection { new Point(cx, ay), new Point(cx, cy), new Point(ax, cy) },
                Stroke = Theme.Brush(Theme.Fg),
                StrokeThickness = 2,
            });
        }
    }

    /// <summary>The button for the page's test state (<c>fullScreen</c>): what it does, its box on the page, and the explanation.</summary>
    public void WriteTest(Utf8JsonWriter json, UIElement page)
    {
        json.WriteStartObject("fullScreen");
        json.WriteString("state", _state switch
        {
            FullScreenState.Enter => "enter",
            FullScreenState.Exit => "exit",
            FullScreenState.HomeScreen => "homescreen",
            _ => "none",
        });
        json.WriteString("caption", Caption);
        json.WriteBoolean("shown", _button.Visibility == Visibility.Visible && _button.ActualWidth > 0);
        json.WriteBoolean("explaining", Explaining);
        json.WriteBoolean("reentry", _code is not null);
        if (_button.ActualWidth > 0)
        {
            Rect box = _button.TransformToVisual(page).TransformBounds(new Rect(0, 0, _button.ActualWidth, _button.ActualHeight));
            json.WriteNumber("x", Math.Round(box.X + (box.Width / 2), 1));
            json.WriteNumber("y", Math.Round(box.Y + (box.Height / 2), 1));
            json.WriteNumber("w", Math.Round(box.Width, 1));
            json.WriteNumber("h", Math.Round(box.Height, 1));
        }

        json.WriteEndObject();
    }
}
