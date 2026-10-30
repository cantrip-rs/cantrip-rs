using Cantrip.Protocol;
using Microsoft.UI.Text;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// The stack across a tablet, between the halves on the shared board and over the seat's half on a player's own: a
/// band as tall as the page gives it, the turn and the stack's count on its left, and the spells and abilities as an
/// overlapping row, oldest at the left and the top of the stack whole at the right. Always there, empty or not, so the
/// halves never move as a spell is cast. Its cards are tapped as a battlefield's are: a spell can be a target.
/// </summary>
public sealed class TabletStack : Grid
{
    /// <summary>The label column on the band's left.</summary>
    private const double LabelWidth = PlayerRail.RailWidth;

    /// <summary>How far the band keeps its cards from its edges.</summary>
    private const float Inset = 8f;

    private readonly TextBlock _title;
    private readonly TextBlock _turn;
    private readonly TextBlock _empty;
    private readonly Grid _cards;

    public TabletStack()
    {
        Background = Theme.Brush(Theme.Bar);
        BorderBrush = Theme.Brush(Theme.Line);
        BorderThickness = new Thickness(0, 1, 0, 1);
        ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(LabelWidth) });
        ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });

        var label = new StackPanel { Spacing = 4, VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(16, 0, 12, 0) };
        _title = new TextBlock { Text = "Stack", FontSize = 20, FontWeight = FontWeights.SemiBold, Foreground = Theme.Brush(Theme.Fg) };
        _turn = new TextBlock { FontSize = 14, Foreground = Theme.Brush(Theme.FgSoft), TextWrapping = TextWrapping.WrapWholeWords };
        label.Children.Add(_title);
        label.Children.Add(_turn);
        Children.Add(label);

        _cards = new Grid();
        Grid.SetColumn(_cards, 1);
        Children.Add(_cards);

        _empty = new TextBlock
        {
            Text = "Nothing on the stack",
            FontSize = 15,
            Foreground = Theme.Brush(Theme.FgMuted),
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(Inset, 0, 0, 0),
        };
        _cards.Children.Add(_empty);

        Board = new CardBoardElement(60, 84)
        {
            Slides = true,
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(Inset, 0, 0, 0),
        };
        _cards.Children.Add(Board);
    }

    public CardBoardElement Board { get; }

    /// <summary>Lays the stack out, oldest first as the view lists it, and says whose turn and step it is.</summary>
    public void Show(IReadOnlyList<ObjectView> stack, string turn)
    {
        _title.Text = stack.Count == 0 ? "Stack" : $"Stack  ·  {stack.Count}";
        _turn.Text = turn;
        _empty.Visibility = stack.Count == 0 ? Visibility.Visible : Visibility.Collapsed;

        float height = MathF.Max(40f, (float)_cards.ActualHeight - (2f * Inset));
        float width = MathF.Round(height * 63f / 88f);
        Board.SetCellSize(width, height);
        Board.Height = height;

        // Each card whole while they fit; past that, closer together, the top of the stack still whole at the right.
        float room = MathF.Max(width, (float)_cards.ActualWidth - (2f * Inset));
        float step = stack.Count <= 1 ? width : MathF.Min(width + 10f, (room - width) / (stack.Count - 1));

        List<CardBoardElement.BoardCell> cells = [];
        float right = 0f;
        for (int i = 0; i < stack.Count; i++)
        {
            float x = i * step;
            cells.Add(new CardBoardElement.BoardCell(stack[i], x, 0, Tapped: false));
            right = x + width;
        }

        Board.Width = MathF.Max(1f, right);
        Board.SetCells(cells);
    }
}
