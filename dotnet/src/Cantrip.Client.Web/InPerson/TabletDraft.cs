using System.Text.Json;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Media;
using Windows.Foundation;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// The grid draft on a tablet (docs/in-person.md, "The grid draft"): the grid face up, with a button beside each row and
/// over each column, and beside it what both players are shown of the draft - which grid of how many, whose pick, how many
/// cards each has drafted, the last picks in words and the pick clock (<see cref="PublicGridMessage"/>). Nothing else: no
/// pool, nothing either seat alone sees.
/// <para>
/// A finger picks a line in two taps, so a stray touch takes nothing: the first lights the line and chooses it, the second
/// - on the same button, or on Take - takes it. Only the face in front of the player whose pick it is has its buttons
/// live. Each tap goes to the relay (<see cref="TableLineMessage"/>), which hands it to that player's phone to send as
/// its own pick; the table judges it, and the grid moves on when the table says so.
/// </para>
/// <para>
/// The shared board shows the draft twice, as its stack band does: one face upright for seat 1 at the tablet's bottom edge,
/// one turned to face seat 2 across the table, so each player reads the grid upright and picks on the face in front of
/// them. A grid cannot read upright for two people facing each other, and one turned sideways reads well for neither. A
/// player's own board shows one face, theirs, live only on their pick.
/// </para>
/// </summary>
public sealed class TabletDraft : Grid
{
    /// <summary>How long a pick just made is shown - its line lit on the grid it was taken from - before the grid moves on.</summary>
    private static readonly TimeSpan PickShown = TimeSpan.FromMilliseconds(1200);

    /// <summary>How long a take waits for the table to move the draft on before its buttons are live again.</summary>
    private static readonly TimeSpan TakeWait = TimeSpan.FromSeconds(4);

    /// <summary>A clock with more than this left is not shown: an in-person table's "off" is a clock too long to matter.</summary>
    private static readonly TimeSpan LongestClockShown = TimeSpan.FromDays(1);

    private readonly List<Face> _faces = [];
    private readonly Action<DeviceClientMessage> _send;
    private readonly Func<int, string> _nameOf;
    private readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromMilliseconds(250) };

    private PublicGridMessage? _grid;

    /// <summary>The grid shown a moment longer, the pick just made from it lit, and until when.</summary>
    private (PublicGridMessage Grid, int Line, int Seat, DateTime Until)? _held;

    /// <summary>The line chosen with a first tap, and the pick it was chosen for.</summary>
    private int? _chosen;
    private int _chosenFor = -1;

    /// <summary>The pick a take was sent for, and when, so a second take is not sent while the first is on its way.</summary>
    private int _sentFor = -1;
    private DateTime _sentAt;

    private DateTime? _clockBy;

    /// <param name="side">The seat whose own board this is, or zero for the shared board.</param>
    /// <param name="send">Sends the relay a line picked.</param>
    /// <param name="nameOf">A seat's player's name.</param>
    public TabletDraft(int side, Action<DeviceClientMessage> send, Func<int, string> nameOf)
    {
        _send = send;
        _nameOf = nameOf;
        Background = Theme.Brush(Theme.Window);
        Visibility = Visibility.Collapsed;

        if (side == 0)
        {
            RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            var turned = new Face(this, seat: 2, turned: true);
            var upright = new Face(this, seat: 1, turned: false);
            Grid.SetRow(upright, 1);
            Children.Add(turned);
            Children.Add(upright);
            _faces.Add(upright);
            _faces.Add(turned);
        }
        else
        {
            var face = new Face(this, side, turned: false);
            Children.Add(face);
            _faces.Add(face);
        }

        _timer.Tick += (_, _) => Tick();
        SizeChanged += (_, _) => Show();
    }

    /// <summary>Whether a grid is being drafted and shown.</summary>
    public bool Drafting => _grid is not null;

    /// <summary>The rows of cards, each with whether it is turned to face seat 2, for the page to make long-pressable.</summary>
    public IEnumerable<(CardBoardElement Board, bool Turned)> Boards => _faces.Select(face => (face.Board, face.Turned));

    /// <summary>Whether a row of cards is the face turned to face seat 2.</summary>
    public bool IsTurned(CardBoardElement board) => _faces.Any(face => face.Turned && face.Board == board);

    /// <summary>The draft as the relay last said: shown, or put away once it is over.</summary>
    public void Take(PublicGridMessage grid)
    {
        PublicGridMessage? before = _grid;
        if (grid.Grid == 0)
        {
            _grid = null;
            _held = null;
            Visibility = Visibility.Collapsed;
            _timer.Stop();
            return;
        }

        // Each place under a handle of its own grid, so the next grid's cards are new cards, not this one's moved.
        for (int place = 0; place < grid.Places.Count; place++)
        {
            grid.Places[place].Handle = new ViewHandle(5_000_000 + (grid.Grid * 16) + place);
        }

        // A pick just made: the grid it was taken from, its line lit, for a moment.
        if (before is not null && grid.Decision == before.Decision + 1 && grid.LastLine is >= 0 and < 6)
        {
            _held = (before, grid.LastLine, before.Active, DateTime.UtcNow + PickShown);
        }

        _grid = grid;
        _clockBy = grid.PickLeftMs > 0 ? DateTime.UtcNow + TimeSpan.FromMilliseconds(grid.PickLeftMs) : null;
        if (_chosenFor != grid.Decision)
        {
            _chosen = null;
        }

        Visibility = Visibility.Visible;
        _timer.Start();
        Show();
    }

    private void Tick()
    {
        if (_held is { } held && DateTime.UtcNow >= held.Until)
        {
            _held = null;
            Show();
        }
        else if (_grid is { } grid && _sentFor == grid.Decision && DateTime.UtcNow - _sentAt >= TakeWait)
        {
            // The take went nowhere - the picker's phone is away - so the line may be taken again.
            _sentFor = -1;
            Show();
        }
        else
        {
            ShowClock();
        }
    }

    /// <summary>A line's button tapped on a face: the line chosen, or - tapped again - taken.</summary>
    private void LineTapped(Face face, int line)
    {
        if (!Live(face) || _grid is not { } grid || !HasCards(grid, line))
        {
            return;
        }

        if (_chosen == line && _chosenFor == grid.Decision)
        {
            TakeChosen(face);
            return;
        }

        _chosen = line;
        _chosenFor = grid.Decision;
        _send(new TableLineMessage { Decision = grid.Decision, Line = line, Take = false });
        Show();
    }

    /// <summary>Takes the line chosen, through the picker's phone.</summary>
    private void TakeChosen(Face face)
    {
        if (!Live(face) || _grid is not { } grid || _chosen is not { } line || _chosenFor != grid.Decision)
        {
            return;
        }

        _sentFor = grid.Decision;
        _sentAt = DateTime.UtcNow;
        _send(new TableLineMessage { Decision = grid.Decision, Line = line, Take = true });
        Show();
    }

    /// <summary>Whether a face's buttons pick now: it faces the player whose pick it is, and nothing is on its way.</summary>
    private bool Live(Face face) => _grid is { } grid && _held is null && grid.Active == face.Seat && _sentFor != grid.Decision;

    private void Show()
    {
        if (_grid is not { } grid)
        {
            return;
        }

        foreach (Face face in _faces)
        {
            face.Show(_held is { } held ? held.Grid : grid, grid, _held is { } shown ? shown.Line : _chosenFor == grid.Decision ? _chosen : null);
        }

        ShowClock();
    }

    private void ShowClock()
    {
        string clock = string.Empty;
        if (_clockBy is { } by && by - DateTime.UtcNow <= LongestClockShown && _held is null)
        {
            TimeSpan left = by - DateTime.UtcNow;
            clock = left <= TimeSpan.Zero ? "Time is up." : $"{(int)left.TotalMinutes}:{left.Seconds:00} left to pick";
        }

        foreach (Face face in _faces)
        {
            face.Clock.Text = clock;
        }
    }

    private static bool HasCards(PublicGridMessage grid, int line) => Places(line).Any(place => place < grid.Places.Count && grid.Places[place].Known);

    /// <summary>The places in the grid of a line: a row, 0 to 2, or a column, 3 to 5.</summary>
    private static IEnumerable<int> Places(int line) => line < 3
        ? Enumerable.Range(0, 3).Select(column => (line * 3) + column)
        : Enumerable.Range(0, 3).Select(row => (row * 3) + (line - 3));

    private static string LineName(int line) => line < 3 ? $"Row {line + 1}" : $"Column {line - 2}";

    /// <summary>What a face says of the draft: the status line, in words for the player in front of it.</summary>
    private string Status(Face face, PublicGridMessage grid)
    {
        if (_held is { } held)
        {
            string who = held.Seat == face.Seat ? "You take" : $"{_nameOf(held.Seat)} takes";
            return $"{who} {LineName(held.Line).ToLowerInvariant()}.";
        }

        string picker = _nameOf(grid.Active);
        if (grid.Active != face.Seat)
        {
            return grid.Pick == 1 ? $"{picker} picks first from this grid." : $"{picker} picks from what is left.";
        }

        if (_sentFor == grid.Decision)
        {
            return "Taking...";
        }

        if (_chosen is { } line && _chosenFor == grid.Decision)
        {
            return $"{LineName(line)} chosen: tap it again, or Take, to take it.";
        }

        return grid.Pick == 1
            ? $"{picker}, your pick: tap a row or a column."
            : $"{picker}, your pick from what is left: tap a row or a column. The rest is discarded.";
    }

    // --- For a page driven by a test (TabletTestState.cs) ---

    /// <summary>Whether the faces are laid out and nothing is held up to be seen: what a test waits for.</summary>
    public bool Busy => Visibility == Visibility.Visible && (_held is not null || _faces.Any(face => face.Board.ActualWidth <= 0 || face.Board.Moving));

    /// <summary>
    /// The draft as a test reads it, under the tablet's <c>grid</c>: the pick being made, and each face's seat, whether it is
    /// turned, what it says and its buttons - each line's and Take's label, box, whether it is live and chosen.
    /// </summary>
    public void WriteTest(Utf8JsonWriter json, UIElement page)
    {
        json.WriteStartObject("grid");
        json.WriteBoolean("shown", Drafting);
        json.WriteNumber("grid", _grid?.Grid ?? 0);
        json.WriteNumber("grids", _grid?.Grids ?? 0);
        json.WriteNumber("decision", _grid?.Decision ?? -1);
        json.WriteNumber("active", _grid?.Active ?? 0);
        json.WriteNumber("pick", _grid?.Pick ?? 0);
        json.WriteBoolean("held", _held is not null);
        json.WriteStartArray("faces");
        foreach (Face face in Drafting ? _faces : [])
        {
            json.WriteStartObject();
            json.WriteNumber("seat", face.Seat);
            json.WriteBoolean("turned", face.Turned);
            json.WriteString("status", face.Status.Text);
            json.WriteString("seats", face.Seats.Text);
            json.WriteStartArray("recent");
            foreach (TextBlock said in face.Recent.Children.OfType<TextBlock>())
            {
                json.WriteStringValue(said.Text);
            }

            json.WriteEndArray();
            json.WriteStartArray("buttons");
            foreach ((DockButton button, string label, int line) in face.Buttons())
            {
                if (button.XamlRoot is null || button.ActualWidth <= 0 || button.Visibility != Visibility.Visible)
                {
                    continue;
                }

                Rect box = button.TransformToVisual(page).TransformBounds(new Rect(0, 0, button.ActualWidth, button.ActualHeight));
                json.WriteStartObject();
                json.WriteString("label", label);
                json.WriteNumber("line", line);
                json.WriteBoolean("enabled", button.Enabled);
                json.WriteBoolean("chosen", line >= 0 && button.On);
                json.WriteNumber("x", Math.Round(box.X + (box.Width / 2), 1));
                json.WriteNumber("y", Math.Round(box.Y + (box.Height / 2), 1));
                json.WriteNumber("w", Math.Round(box.Width, 1));
                json.WriteNumber("h", Math.Round(box.Height, 1));
                json.WriteEndObject();
            }

            json.WriteEndArray();
            json.WriteEndObject();
        }

        json.WriteEndArray();
        json.WriteEndObject();
    }

    /// <summary>The grid draft once, upright for one player: the grid with its buttons, and beside it what is said of it.</summary>
    private sealed class Face : Grid
    {
        /// <summary>A line button's thickness: a fingertip's, and room for the arrow.</summary>
        private const double ButtonSide = 48;
        private const double Gap = 5;
        private const double SideWidth = 330;
        private const double Spacing = 28;

        private readonly TabletDraft _draft;
        private readonly Canvas _picker = new() { HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
        private readonly DockButton[] _lines = new DockButton[6];
        private readonly DockButton _take = new("Take", primary: true, fontSize: 18) { MinHeight = 56, MinWidth = 200, HorizontalAlignment = HorizontalAlignment.Left };

        public Face(TabletDraft draft, int seat, bool turned)
        {
            _draft = draft;
            Seat = seat;
            Turned = turned;
            Padding = new Thickness(18, 8, 18, 8);
            BorderBrush = Theme.Brush(Theme.Line);
            BorderThickness = turned ? new Thickness(0, 0, 0, 1) : new Thickness(0);

            // The grid and what is said of it side by side, the pair in the middle of the face.
            ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(SideWidth + Spacing) });
            ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            if (turned)
            {
                RenderTransformOrigin = new Point(0.5, 0.5);
                RenderTransform = new RotateTransform { Angle = 180 };
            }

            Board = new CardBoardElement(BoardRows.StandardCardWidth, BoardRows.CardHeight(BoardRows.StandardCardWidth))
            {
                FliesAlong = false,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Top,
            };
            _picker.Children.Add(Board);
            for (int line = 0; line < 6; line++)
            {
                var button = new DockButton(line < 3 ? "▸" : "▾", primary: false, fontSize: 24).Filling();
                AutomationProperties.SetName(button, LineName(line));
                int chosen = line;
                button.Clicked += () => _draft.LineTapped(this, chosen);
                _lines[line] = button;
                _picker.Children.Add(button);
            }

            Grid.SetColumn(_picker, 1);
            Children.Add(_picker);
            SizeChanged += (_, _) => _draft.Show();

            AutomationProperties.SetName(_take, "Take");
            _take.Clicked += () => _draft.TakeChosen(this);
            var side = new StackPanel
            {
                Spacing = 10,
                Width = SideWidth,
                Margin = new Thickness(Spacing, 0, 0, 0),
                VerticalAlignment = VerticalAlignment.Center,
                Children = { Title, Seats, Status, Clock, _take, Recent },
            };
            Grid.SetColumn(side, 2);
            Children.Add(side);
        }

        /// <summary>The seat of the player this face is upright for.</summary>
        public int Seat { get; }

        public bool Turned { get; }

        public CardBoardElement Board { get; }

        public TextBlock Title { get; } = Text(22, Theme.Fg, semibold: true);

        public TextBlock Seats { get; } = Text(16, Theme.FgSoft);

        public TextBlock Status { get; } = Text(18, Theme.Accent, semibold: true);

        public TextBlock Clock { get; } = Text(16, Theme.Fg, semibold: true);

        public StackPanel Recent { get; } = new() { Spacing = 4, Margin = new Thickness(0, 6, 0, 0) };

        /// <summary>Every button on this face, with the label a test finds it by and its line (-1 for Take).</summary>
        public IEnumerable<(DockButton Button, string Label, int Line)> Buttons() =>
            [.. _lines.Select((button, line) => (button, LineName(line), line)), (_take, "Take", -1)];

        /// <param name="shown">The grid drawn: the one being picked from, or the one a pick was just taken from.</param>
        /// <param name="now">The draft as it stands.</param>
        /// <param name="lit">The line lit: the one chosen, or the pick just made.</param>
        public void Show(PublicGridMessage shown, PublicGridMessage now, int? lit)
        {
            Title.Text = $"Grid {now.Grid} of {now.Grids}";
            Seats.Text = string.Join("    ", now.Drafted.Select((count, i) => $"{_draft._nameOf(i + 1)}: {count} card{(count == 1 ? string.Empty : "s")}"));
            bool mine = now.Active == Seat;
            bool live = _draft.Live(this);
            Status.Text = _draft.Status(this, now);
            Status.Foreground = Theme.Brush(mine && _draft._held is null ? Theme.Accent : Theme.FgSoft);

            // The grid as large as the face allows, beside the buttons on its rows and over its columns.
            double room = ActualHeight - Padding.Top - Padding.Bottom;
            double across = ActualWidth - Padding.Left - Padding.Right - SideWidth - Spacing;
            if (room <= 0 || across <= 0)
            {
                return;
            }

            double byHeight = (room - ButtonSide - (3 * Gap)) / 3;
            double byWidth = (across - ButtonSide - (3 * Gap)) / 3 * BoardRows.AspectRatio;
            float height = (float)Math.Max(40, Math.Floor(Math.Min(byHeight, byWidth)));
            float width = MathF.Round(height / BoardRows.AspectRatio);
            height = BoardRows.CardHeight(width);
            double left = ButtonSide + Gap;
            double top = ButtonSide + Gap;

            HashSet<int> lighting = lit is { } line ? [.. Places(line)] : [];
            List<int> known = [.. Enumerable.Range(0, shown.Places.Count).Where(place => shown.Places[place].Known)];
            Board.SetCellSize(width, height);
            Board.Width = (3 * (width + Gap)) - Gap;
            Board.Height = (3 * (height + Gap)) - Gap;
            Board.SetCells([.. known.Select(place => new CardBoardElement.BoardCell(shown.Places[place],
                (place % 3) * (width + (float)Gap), (place / 3) * (height + (float)Gap), Tapped: false, Selected: lighting.Contains(place)))]);
            Canvas.SetLeft(Board, left);
            Canvas.SetTop(Board, top);

            for (int each = 0; each < 6; each++)
            {
                bool row = each < 3;
                DockButton button = _lines[each];
                int cards = Places(each).Count(place => place < shown.Places.Count && shown.Places[place].Known);
                button.Width = row ? ButtonSide : width;
                button.Height = row ? height : ButtonSide;
                button.Enabled = live && cards > 0;
                button.On = lit == each;
                button.Tooltip = cards == 0 ? $"{LineName(each)} has been taken." : $"{LineName(each)}: {cards} card{(cards == 1 ? string.Empty : "s")}";
                Canvas.SetLeft(button, row ? 0 : left + ((each - 3) * (width + Gap)));
                Canvas.SetTop(button, row ? top + (each * (height + Gap)) : 0);
            }

            _picker.Width = left + Board.Width;
            _picker.Height = top + Board.Height;

            int count = _draft._chosen is { } chosen && _draft._chosenFor == now.Decision ? Places(chosen).Count(place => now.Places[place].Known) : 0;
            _take.Visibility = mine && _draft._held is null ? Visibility.Visible : Visibility.Collapsed;
            _take.Caption = count > 0 && _draft._chosen is { } taking
                ? $"Take {LineName(taking).ToLowerInvariant()} ({count} card{(count == 1 ? string.Empty : "s")})"
                : "Take";
            _take.Enabled = live && count > 0;

            Recent.Children.Clear();
            foreach (string said in now.Recent.AsEnumerable().Reverse().Take(6))
            {
                TextBlock text = Text(14, Recent.Children.Count == 0 ? Theme.Fg : Theme.FgMuted, wrap: true);
                text.Text = said.Replace("[", string.Empty, StringComparison.Ordinal).Replace("]", string.Empty, StringComparison.Ordinal);
                Recent.Children.Add(text);
            }
        }

        private static TextBlock Text(double size, Windows.UI.Color color, bool semibold = false, bool wrap = true) => new()
        {
            FontSize = size,
            FontWeight = semibold ? FontWeights.SemiBold : FontWeights.Normal,
            Foreground = Theme.Brush(color),
            TextWrapping = wrap ? TextWrapping.WrapWholeWords : TextWrapping.NoWrap,
        };
    }
}
