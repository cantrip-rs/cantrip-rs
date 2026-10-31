using Cantrip.Protocol.InPerson;

namespace Cantrip.Client.Web.InPerson;

/// <summary>
/// The tablet's grid draft (<see cref="TabletDraft"/>): over the board, under the layers a card is held up in, from the
/// first <see cref="PublicGridMessage"/> until the draft is over. Its cards are held up to read with a long press as the
/// board's are; a tap on a card picks nothing - a row or a column is picked with its button.
/// </summary>
public sealed partial class TabletPage
{
    private TabletDraft _draft = null!;

    private void AttachDraft()
    {
        _draft = new TabletDraft(_side, message =>
        {
            if (ClientConfig.Test && message is TableLineMessage line)
            {
                _lines.Add(line);
            }

            Send(message);
        }, SeatName);

        // Over the board and under the arrows, flights and the cards held up, which come next.
        _root.Children.Insert(1, _draft);
        foreach ((CardBoardElement board, bool turned) in _draft.Boards)
        {
            MakeTappable(board, turned);
        }
    }

    /// <summary>Whether the grid draft is up, in place of the phase screen before the game.</summary>
    private bool ShowingDraft()
    {
        if (!_draft.Drafting)
        {
            return false;
        }

        _phaseScreen.Visibility = Visibility.Collapsed;
        return true;
    }
}
