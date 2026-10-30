using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>The cards a viewer has been sent as printed, as a session keeps them for each seat and the relay for the public.</summary>
public sealed class PrintedCardsTests
{
    private static ObjectView Card(string id) => new() { CardId = id, Name = id };

    [Fact]
    public void Each_card_is_sent_the_first_time_it_is_seen_and_kept_for_a_catch_up()
    {
        var printed = new PrintedCards([Card("Island")]);
        var view = new PlayerView { Objects = [Card("Island"), Card("Bolt"), Card("Bolt"), new ObjectView(), Card("Unknown")] };

        List<ObjectView> fresh = printed.NewlySeen(view, id => id == "Unknown" ? null : Card(id));

        Assert.Equal(["Bolt"], fresh.Select(card => card.CardId));
        Assert.Equal(["Island", "Bolt"], printed.Cards.Select(card => card.CardId));
        Assert.Empty(printed.NewlySeen(view, Card));
    }
}
