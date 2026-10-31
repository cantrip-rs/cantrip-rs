using Cantrip.Protocol;
using Microsoft.UI.Text;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// A graveyard or the cards in exile, opened on a tablet from a seat's rail: every card in it laid out to read and to tap,
/// over that seat's battlefield and turned with its half. What a graveyard holds and what is in exile face up are public
/// (CR 404.2, 406.3): the list shows the public view's cards and nothing else, a card exiled face down as a back. A card
/// in it is tapped as a permanent is - a creature card in a graveyard tapped as a spell's target - and held up to read
/// with a long press.
/// </summary>
public sealed class TabletPile : Border
{
    /// <summary>The widest a card is drawn in the list, and the narrowest before the rows run off the bottom.</summary>
    private const float WidestCard = 120f;
    private const float NarrowestCard = 36f;
    private const float Gap = 8f;
    private const float Inset = 12f;

    private readonly TextBlock _title;
    private readonly Grid _area;

    public TabletPile()
    {
        Background = Theme.Brush(Theme.WithAlpha(Theme.Panel, 0.97));
        BorderBrush = Theme.Brush(Theme.ControlBorder);
        BorderThickness = new Thickness(1);
        CornerRadius = new CornerRadius(10);
        Margin = new Thickness(10);
        Visibility = Visibility.Collapsed;

        var layout = new Grid();
        layout.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        layout.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

        var header = new Grid { Padding = new Thickness(Inset, 8, 8, 4) };
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        _title = new TextBlock { FontSize = 18, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg), VerticalAlignment = VerticalAlignment.Center };
        header.Children.Add(_title);

        // A finger's size: the list is shut from here, or from the pile it was opened from.
        Close = new DockButton("Close", primary: false, fontSize: 15) { MinWidth = 88, MinHeight = 40 };
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(Close, "Close pile");
        Grid.SetColumn(Close, 1);
        header.Children.Add(Close);
        layout.Children.Add(header);

        _area = new Grid();
        Grid.SetRow(_area, 1);
        Board = new CardBoardElement(WidestCard, BoardRows.CardHeight(WidestCard))
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Top,
            Margin = new Thickness(Inset, 4, Inset, Inset),
        };
        _area.Children.Add(Board);
        layout.Children.Add(_area);
        Child = layout;
    }

    /// <summary>The cards of the pile, tapped and held as a battlefield's are.</summary>
    public CardBoardElement Board { get; }

    /// <summary>The button that shuts the list.</summary>
    public DockButton Close { get; }

    /// <summary>The pile open, or null: its zone and its owner.</summary>
    public (ZoneKindView Zone, int Owner)? Open { get; private set; }

    /// <summary>Opens the list on a pile, or shuts it.</summary>
    public void Show((ZoneKindView Zone, int Owner)? pile)
    {
        Open = pile;
        Visibility = pile is null ? Visibility.Collapsed : Visibility.Visible;
        if (pile is null)
        {
            Board.SetCells([]);
        }
    }

    /// <summary>
    /// Lays the open pile's cards out in rows, the most recent first, at the largest size at which they all fit; and says
    /// whose pile it is and how many cards it holds.
    /// </summary>
    public void Lay(string title, IReadOnlyList<ObjectView> cards)
    {
        _title.Text = $"{title}  ·  {cards.Count}";
        float width = (float)Math.Max(0, _area.ActualWidth - (2 * Inset));
        float height = (float)Math.Max(0, _area.ActualHeight - Inset - 4);
        if (width <= 0 || height <= 0)
        {
            return;
        }

        float card = WidestCard;
        int perRow = 1;
        for (; card > NarrowestCard; card -= 2f)
        {
            perRow = Math.Max(1, (int)((width + Gap) / (card + Gap)));
            int rows = (cards.Count + perRow - 1) / perRow;
            if ((rows * (BoardRows.CardHeight(card) + Gap)) - Gap <= height)
            {
                break;
            }
        }

        perRow = Math.Max(1, (int)((width + Gap) / (card + Gap)));
        float cardHeight = BoardRows.CardHeight(card);
        Board.SetCellSize(card, cardHeight);
        List<CardBoardElement.BoardCell> cells = [];
        for (int i = 0; i < cards.Count; i++)
        {
            int index = cards.Count - 1 - i;
            cells.Add(new CardBoardElement.BoardCell(cards[index], (i % perRow) * (card + Gap), (i / perRow) * (cardHeight + Gap), Tapped: false));
        }

        int shownRows = Math.Max(1, (cards.Count + perRow - 1) / perRow);
        Board.Width = Math.Max(1, (Math.Min(cards.Count, perRow) * (card + Gap)) - Gap);
        Board.Height = Math.Max(1, (shownRows * (cardHeight + Gap)) - Gap);
        Board.SetCells(cells);
    }
}
