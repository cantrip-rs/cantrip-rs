using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Media;
using ManaRow = (Cantrip.Protocol.ActionOptionMessage Option, Cantrip.Protocol.ColorView? Color, System.Collections.Generic.List<Cantrip.Protocol.ColorView> Combination, Cantrip.Protocol.ViewHandle? Sacrifice);

namespace Cantrip.Client.Web;

/// <summary>
/// A phone's hand at an in-person table (docs/web-client.md, "In-person play"): looked through, read and played without a
/// card ever being played by a slip of the thumb.
/// <list type="bullet">
/// <item>The whole hand is in view at once, across the phone, each card showing at least its name's start.</item>
/// <item>A touch on a card, or a thumb drawn along the hand, brings the card under the thumb out whole, the others moved
/// aside, standing proud of them; it stays so once the thumb lifts. Nothing is played and no box of words follows the
/// thumb.</item>
/// <item>A card swiped up out of the hand is played: what it can do, when that is one thing; its sheet, when it can do
/// several (cast it or cycle it) or nothing now.</item>
/// <item>A finger resting on a card opens its sheet: the card as large as the phone allows, beside what it can do now,
/// each a button worded as its menu row is ("Cast Lightning Bolt", "Cycling {2}") - or that it can do nothing now.
/// Nothing is done until one is pressed. The sheet pages through the hand by a swipe across the card or its arrows, and
/// shuts with a tap off it or Close.</item>
/// </list>
/// A tap that answers a question about the hand - a card to discard, a card chosen for an effect, a target - answers it at
/// once, as on every table: those are picked and confirmed, not played.
/// </summary>
public sealed partial class MainPage
{
    /// <summary>The narrowest the sheet's side with the card's name and buttons is made.</summary>
    private const double SheetPanelWidth = 230;

    private Grid _sheet = null!;
    private CardBoardElement _sheetCard = null!;
    private Border _sheetCardBox = null!;
    private TextBlock _sheetCount = null!;
    private TextBlock _sheetName = null!;
    private SymbolText _sheetNote = null!;
    private StackPanel _sheetActions = null!;
    private DockButton _sheetPrevious = null!;
    private DockButton _sheetNext = null!;

    /// <summary>The card the sheet is open on, or null while it is shut.</summary>
    private ViewHandle? _sheetShows;

    /// <summary>What the sheet's buttons do, as the prompt now stands: pressed by index, so none acts on a prompt gone by.</summary>
    private List<SheetItem> _sheetItems = [];

    /// <summary>What the sheet last showed, to make its buttons again only when that changes.</summary>
    private string _sheetSaid = string.Empty;

    /// <summary>Where a finger went down on the sheet's card, to page the hand when it is swiped across.</summary>
    private Windows.Foundation.Point? _sheetSwipeFrom;

    /// <summary>How far a finger goes across the sheet's card to page to the next card or the one before.</summary>
    private const double SheetSwipe = 48;

    /// <summary>One thing a card can do now: its menu row's words, and its ability and option, as a menu row is known by.</summary>
    private sealed record SheetItem(string Text, Action OnChosen, int Ability, int OptionId);

    /// <summary>The hand thumbed through, and the sheet over the whole phone, shut. Called as the phone's views are built.</summary>
    private void BuildPhoneHand()
    {
        _handBoard.TouchScrubs = true;
        _handBoard.FocusChanged += () =>
        {
            if (_gameClient is { } client)
            {
                LayOutHand(client);
            }
        };

        _handBoard.CardFlicked += PlayFromHand;

        // The hand is all in view at once and taken by the thumb along it: it never scrolls.
        _handScroll.HorizontalScrollMode = ScrollMode.Disabled;
        _handScroll.HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled;

        _sheetCard = new CardBoardElement(BoardRows.StandardCardWidth, BoardRows.CardHeight(BoardRows.StandardCardWidth)) { IsHitTestVisible = false };
        _sheetCardBox = new Border
        {
            Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent),
            VerticalAlignment = VerticalAlignment.Center,
            Child = _sheetCard,
        };
        _sheetCardBox.PointerPressed += (_, e) =>
        {
            e.Handled = true;
            _sheetSwipeFrom = e.GetCurrentPoint(_sheet).Position;
            _sheetCardBox.CapturePointer(e.Pointer);
        };
        _sheetCardBox.PointerReleased += (_, e) =>
        {
            e.Handled = true;
            _sheetCardBox.ReleasePointerCapture(e.Pointer);
            if (_sheetSwipeFrom is { } from)
            {
                _sheetSwipeFrom = null;
                double across = e.GetCurrentPoint(_sheet).Position.X - from.X;
                if (Math.Abs(across) >= SheetSwipe)
                {
                    PageCardSheet(across < 0 ? 1 : -1);
                }
            }
        };
        _sheetCardBox.PointerCaptureLost += (_, _) => _sheetSwipeFrom = null;
        _sheetCardBox.Tapped += (_, e) => e.Handled = true;

        _sheetPrevious = SheetArrow("‹", "Previous card", -1);
        _sheetNext = SheetArrow("›", "Next card", 1);

        _sheetCount = new TextBlock { FontSize = 13, Foreground = Theme.Brush(Theme.FgMuted) };
        _sheetName = new TextBlock
        {
            FontSize = 20,
            FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
            Foreground = Theme.Brush(Theme.Fg),
            TextWrapping = TextWrapping.WrapWholeWords,
        };
        _sheetNote = new SymbolText(14, Theme.FgSoft, wrap: true);
        _sheetActions = new StackPanel { Spacing = 10, Margin = new Thickness(0, 8, 0, 0) };

        var close = new DockButton("Close", primary: false, fontSize: 15) { MinHeight = 44 }.Filling();
        AutomationProperties.SetName(close, "Close the card");
        close.Clicked += CloseCardSheet;

        var panel = new Grid { MinWidth = SheetPanelWidth, Margin = new Thickness(12, 4, 0, 4) };
        panel.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        panel.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        panel.Children.Add(new ScrollViewer
        {
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
            Content = new StackPanel { Spacing = 4, Children = { _sheetCount, _sheetName, _sheetNote, _sheetActions } },
        });
        Grid.SetRow(close, 1);
        close.Margin = new Thickness(0, 8, 0, 0);
        panel.Children.Add(close);
        panel.Tapped += (_, e) => e.Handled = true;

        // The arrows either side of the card, the thumbs' reach; the buttons down the right, the right thumb's.
        var sheet = _sheet = new Grid
        {
            Visibility = Visibility.Collapsed,
            Background = Theme.Brush(Theme.Window),
            Padding = new Thickness(8, 8, 16, 8),
            ColumnSpacing = 4,
        };
        sheet.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        sheet.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        sheet.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        sheet.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        sheet.Children.Add(_sheetPrevious);
        Grid.SetColumn(_sheetCardBox, 1);
        sheet.Children.Add(_sheetCardBox);
        Grid.SetColumn(_sheetNext, 2);
        sheet.Children.Add(_sheetNext);
        Grid.SetColumn(panel, 3);
        sheet.Children.Add(panel);
        sheet.Tapped += (_, e) =>
        {
            e.Handled = true;
            CloseCardSheet();
        };
        Grid.SetRowSpan(sheet, 3);
        Grid.SetColumnSpan(sheet, 2);
        int under = Root.Children.IndexOf(_inspector);
        Root.Children.Insert(under >= 0 ? under : Root.Children.Count, sheet);
        sheet.SizeChanged += (_, _) => SizeCardSheet();
    }

    private DockButton SheetArrow(string arrow, string name, int by)
    {
        var button = new DockButton(arrow, primary: false, fontSize: 34)
        {
            Width = 44,
            MinHeight = 88,
            VerticalAlignment = VerticalAlignment.Center,
        }.Quiet();
        AutomationProperties.SetName(button, name);
        button.Clicked += () => PageCardSheet(by);
        return button;
    }

    /// <summary>
    /// Opens the sheet on a hand card held or swiped up, unless the hand is answering a question - a card to discard, a
    /// target - where a held card is held up to be read as any other is: whether the sheet opened.
    /// </summary>
    private bool OpenCardSheet(ViewHandle handle)
    {
        if (!IsPhone || _gameClient is not { } client || !_hand.Exists(card => card.Handle == handle) || SheetItems(client, handle) is null)
        {
            return false;
        }

        _sheetShows = handle;
        _sheetSaid = string.Empty;
        _handBoard.Unfocus();
        _sheet.Visibility = Visibility.Visible;
        FollowCardSheet(client);
        return true;
    }

    /// <summary>
    /// Whether a tap on a hand card only brings it forward (<see cref="CardBoardElement.Focused"/>, done as the finger went
    /// down): on a phone, unless the tap answers a question about the hand.
    /// </summary>
    private bool TapOnlyLooks(ViewHandle handle) =>
        IsPhone && _gameClient is { } client && _hand.Exists(card => card.Handle == handle) && SheetItems(client, handle) is not null;

    /// <summary>
    /// A hand card swiped up: what it can do, when that is one thing; its sheet, to choose, when it can do several or
    /// nothing now; and where the hand answers a question - a card to discard, a target - the card as a tap would pick it.
    /// </summary>
    private void PlayFromHand(ViewHandle handle)
    {
        if (!IsPhone || _gameClient is not { } client || !_hand.Exists(card => card.Handle == handle))
        {
            return;
        }

        switch (SheetItems(client, handle))
        {
            case null:
                OnCardClicked(handle);
                break;
            case [SheetItem only]:
                _handBoard.Unfocus();
                SendOffTheTop(handle);
                only.OnChosen();
                break;
            default:
                OpenCardSheet(handle);
                break;
        }
    }

    /// <summary>
    /// A card played from the hand flies up out of the top of the phone as it leaves the hand, toward the table, where the
    /// tablet flies it in from the player's side (TabletPage). A spell waiting on its targets or costs stays until then.
    /// </summary>
    private void SendOffTheTop(ViewHandle handle) =>
        _flights.SendOff(handle, last => last with { Y = -(last.Width * BoardRows.AspectRatio) });

    private void CloseCardSheet()
    {
        _sheetShows = null;
        _sheetItems = [];
        _sheetSaid = string.Empty;
        _sheet.Visibility = Visibility.Collapsed;
    }

    /// <summary>The sheet on the hand's next card, or the one before, round from the last to the first.</summary>
    private void PageCardSheet(int by)
    {
        if (_gameClient is not { } client || _sheetShows is not { } shown || _hand.Count == 0)
        {
            return;
        }

        int at = _hand.FindIndex(card => card.Handle == shown);
        _sheetShows = _hand[(((at < 0 ? 0 : at) + by) % _hand.Count + _hand.Count) % _hand.Count].Handle;
        FollowCardSheet(client);
    }

    /// <summary>
    /// What a hand card can do now, each as the card's menu would word it; empty for a card that can do nothing now; null
    /// where a tap on it answers a question - a card to discard or choose, a target, a cost's permanent - rather than plays it.
    /// </summary>
    private List<SheetItem>? SheetItems(GameClient client, ViewHandle handle)
    {
        bool plays = AimCandidates(client) is null && _selection.CostChoice is null
            && client.Prompt?.Prompt is PriorityPromptMessage or PayCostsPromptMessage or UnlessPaysPromptMessage or ActivateManaAbilityPromptMessage;
        if (plays && _actions.For(handle) is { Count: > 0 } offered)
        {
            List<List<ManaRow>> entries = CardActions.MenuEntries(CardActions.ManaRows(offered));
            return [.. MenuItems(client, entries).Select((item, i) => new SheetItem(item.Text, item.OnChosen, entries[i][0].Option.AbilityIndex, entries[i][0].Option.OptionId))];
        }

        return _looks.TryGetValue(handle, out CardLook? look) && look.Clicked is not null ? null : [];
    }

    /// <summary>
    /// Keeps the sheet with the game, on every redraw: shut once its card has left the hand, a choice fills the phone or the
    /// hand is no longer the view; otherwise its card, its place in the hand, its note and what it can do as things stand.
    /// </summary>
    private void FollowCardSheet(GameClient client)
    {
        if (_sheetShows is not { } handle)
        {
            return;
        }

        int at = _hand.FindIndex(each => each.Handle == handle);
        if (at < 0 || _choicesOpen || _phoneView.View != PhoneView.Hand)
        {
            CloseCardSheet();
            return;
        }

        ObjectView card = _hand[at];
        List<SheetItem> items = SheetItems(client, handle) ?? [];
        _sheetItems = items;
        CardLook look = LookOf(client, card);
        string said = $"{handle.Value}|{at}|{_hand.Count}|{card.Name}|{look.Note}|{string.Join("|", items.Select(item => $"{item.Text}:{item.Ability}:{item.OptionId}"))}";
        _sheetCard.SetCells([Cell(client, card, 0, 0, tapped: false)]);
        if (said == _sheetSaid)
        {
            return;
        }

        _sheetSaid = said;
        SizeCardSheet();
        _sheetCount.Text = _hand.Count > 1 ? $"{at + 1} of {_hand.Count} in your hand" : "Your hand";
        _sheetName.Text = card.Name;
        _sheetNote.Text = look.Note ?? (items.Count == 0 ? "Nothing to do with it right now." : string.Empty);
        _sheetNote.Visibility = _sheetNote.Text.Length > 0 ? Visibility.Visible : Visibility.Collapsed;
        _sheetPrevious.Visibility = _hand.Count > 1 ? Visibility.Visible : Visibility.Collapsed;
        _sheetNext.Visibility = _sheetPrevious.Visibility;

        _sheetActions.Children.Clear();
        for (int i = 0; i < items.Count; i++)
        {
            SheetItem item = items[i];
            var button = new DockButton(item.Text, primary: i == 0, fontSize: 16)
            {
                MinHeight = 48,
                Tag = new CardMenu.RowIdentity(item.Ability, item.OptionId),
            }.Filling().Wrapping(Math.Max(SheetPanelWidth, _sheetActions.ActualWidth) - 8);
            AutomationProperties.SetName(button, item.Text);
            int index = i;
            button.Clicked += () => PressSheetItem(index);
            _sheetActions.Children.Add(button);
        }
    }

    /// <summary>One of the sheet's buttons: the sheet shuts, and the card does what the button says.</summary>
    private void PressSheetItem(int index)
    {
        if (index < 0 || index >= _sheetItems.Count)
        {
            return;
        }

        Action chosen = _sheetItems[index].OnChosen;
        if (_sheetShows is { } played)
        {
            SendOffTheTop(played);
        }

        CloseCardSheet();
        chosen();
    }

    /// <summary>The sheet's card as tall as the phone allows, keeping room for its buttons beside it.</summary>
    private void SizeCardSheet()
    {
        if (_sheetShows is null || _sheet.ActualHeight <= 0)
        {
            return;
        }

        double tall = _sheet.ActualHeight - _sheet.Padding.Top - _sheet.Padding.Bottom;
        double room = _sheet.ActualWidth - _sheet.Padding.Left - _sheet.Padding.Right - (2 * 48) - SheetPanelWidth - 12;
        float width = (float)Math.Floor(Math.Max(80, Math.Min(room, tall / BoardRows.AspectRatio)));
        float height = BoardRows.CardHeight(width);
        _sheetCard.SetCellSize(width, height);
        _sheetCard.Width = width;
        _sheetCard.Height = height;
    }
}
