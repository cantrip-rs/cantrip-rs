using Cantrip.Protocol;
using Xunit;
using static Cantrip.Client.Llm.Tests.Board;

namespace Cantrip.Client.Llm.Tests;

/// <summary>
/// What the model is shown.
/// <para>
/// The same guarantee the web client's board has, and it matters more here: a projection leak that a renderer
/// merely fails to draw would be read out loud by this one. So the position is written out of
/// <see cref="PlayerView"/> and nothing else — what the server did not send, nobody can say.
/// </para>
/// </summary>
public sealed class GameTextTests
{
    [Fact]
    public void Says_what_is_on_the_board_and_what_each_card_does()
    {
        string text = GameText.Position(Seated(
            Creature(5, "Grizzly Bears", 2, 2),
            Creature(6, "Serra Angel", 4, 4, Them),
            Card(9, "Lightning Bolt", "{R}", "Lightning Bolt deals 3 damage to any target.")));

        Assert.Contains("Your battlefield", text, StringComparison.Ordinal);
        Assert.Contains("Grizzly Bears", text, StringComparison.Ordinal);
        Assert.Contains("Opponent's battlefield", text, StringComparison.Ordinal);
        Assert.Contains("Serra Angel", text, StringComparison.Ordinal);

        // A card in hand comes with its rules text: the model is never asked to remember what a card does.
        Assert.Contains("Your hand", text, StringComparison.Ordinal);
        Assert.Contains("3 damage to any target", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Names_cumulative_upkeep_mana_and_its_limit()
    {
        GameClient client = Seated();
        client.Me!.RestrictedMana = [new RestrictedManaView { Type = ManaTypeView.Blue, Amount = 1, Filter = ManaSpendFilterView.CumulativeUpkeep }];
        string text = GameText.Position(client);
        Assert.Contains("unspent mana 1 blue (only for cumulative upkeep costs)", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Never_names_a_card_the_server_did_not_send()
    {
        // The opponent's hand is a count in the view and has to be a count here. Anything else would be this seat
        // reading cards off a projection that does not carry them.
        string text = GameText.Position(Seated(Card(9, "Black Lotus", "{0}")));

        Assert.Contains("Opponent: 20 life, 4 card(s) in hand", text, StringComparison.Ordinal);
        Assert.DoesNotContain("Opponent's hand", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Says_what_stops_a_creature_attacking()
    {
        ObjectView sick = Creature(5, "Grizzly Bears", 2, 2);
        sick.SummoningSick = true;
        sick.Tapped = true;
        sick.DamageMarked = 1;
        sick.Counters = [new() { Name = "+1/+1", Count = 2 }];

        string text = GameText.Position(Seated(sick));

        Assert.Contains("tapped", text, StringComparison.Ordinal);
        Assert.Contains("summoning sick", text, StringComparison.Ordinal);
        Assert.Contains("1 damage marked", text, StringComparison.Ordinal);
        Assert.Contains("2 +1/+1 counter(s)", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Tells_two_copies_of_one_card_apart()
    {
        // Answers are letters, never these tags — but a question about one of two Grizzly Bears has to be able to
        // say which one, and without a tag it is the same words twice.
        string text = GameText.Position(Seated(
            Creature(5, "Grizzly Bears", 2, 2),
            Creature(6, "Grizzly Bears", 2, 2)));

        Assert.Contains("#5 Grizzly Bears", text, StringComparison.Ordinal);
        Assert.Contains("#6 Grizzly Bears", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Reads_the_stack_top_first()
    {
        // Zone order runs bottom to top; what resolves next is the top, so it is read the other way round. A model
        // told them in zone order would respond to the wrong spell.
        var bolt = Card(7, "Lightning Bolt", "{R}");
        bolt.Zone = ZoneKindView.Stack;
        bolt.ZoneOwner = 0;

        var counter = Card(8, "Counterspell", "{U}{U}");
        counter.Zone = ZoneKindView.Stack;
        counter.ZoneOwner = 0;

        var transport = new Held();
        var client = new GameClient(transport);
        PlayerView view = View(bolt, counter);
        view.Zones.Add(new ZoneView
        {
            Kind = ZoneKindView.Stack,
            Owner = 0,
            ContentsVisible = true,
            Contents = [Handle(7), Handle(8)],
            Count = 2,
        });

        transport.Inbound.Enqueue(new WelcomeMessage { Seat = Me, PlayerNames = ["Bot 1", "Bot 2"] });
        transport.Inbound.Enqueue(new StateMessage { View = view });
        client.Pump();

        string text = GameText.Position(client);
        Assert.True(
            text.IndexOf("Counterspell", StringComparison.Ordinal) < text.IndexOf("#7 Lightning Bolt", StringComparison.Ordinal),
            "the stack was not read top first");
    }

    [Fact]
    public void Writes_what_happened_as_a_sentence()
    {
        GameClient client = Seated(Creature(5, "Grizzly Bears", 2, 2), Creature(6, "Serra Angel", 4, 4, Them));

        Assert.Equal(
            "Opponent cast Serra Angel #6, targeting Grizzly Bears #5",
            GameText.Line(client, new SpellCastEvent
            {
                Spell = Handle(6),
                Controller = Them,
                Targets = [new() { Object = Handle(5) }],
            }));

        Assert.Equal(
            "you lost 3 life (now 17).",
            GameText.Line(client, new LifeChangedEvent { Player = Me, Delta = -3, NewTotal = 17 }));

        // Tapping shows up in the position itself; a line for it would bury the ones that do not.
        Assert.Null(GameText.Line(client, new TapChangedEvent { Object = Handle(5), Tapped = true }));
    }

    /// <summary>
    /// While this seat controls the other player (CR 723), the position says so and lists their hand, which it may see then
    /// (CR 723.4); a question of theirs is said to be theirs, answered for them.
    /// </summary>
    [Fact]
    public void A_controlled_players_turn_is_said_with_their_hand()
    {
        var wrath = Card(9, "Wrath of God", "{2}{W}{W}");
        wrath.Zone = ZoneKindView.Hand;
        wrath.ZoneOwner = Them;

        var transport = new Held();
        var client = new GameClient(transport);
        PlayerView view = View(wrath);
        view.ActivePlayer = Them;
        view.PriorityPlayer = Them;
        view.Players[1].ControlledBy = Me;
        ZoneView theirs = view.Zones.Single(zone => zone.Kind == ZoneKindView.Hand && zone.Owner == Them);
        (theirs.ContentsVisible, theirs.Contents, theirs.Count) = (true, [Handle(9)], 1);

        transport.Inbound.Enqueue(new WelcomeMessage { Seat = Me, PlayerNames = ["Bot 1", "Bot 2"] });
        transport.Inbound.Enqueue(new StateMessage { View = view });
        transport.Inbound.Enqueue(new PromptMessage { RequestId = 1, Prompt = new PriorityPromptMessage { Player = Them }, ForSeat = Them });
        client.Pump();

        Assert.Equal(Them, client.ActingSeat);
        Assert.Equal(Them, client.ControlledSeat);
        string text = GameText.Position(client);
        Assert.Contains("You control Opponent this turn", text, StringComparison.Ordinal);
        Assert.Contains("Opponent's hand", text, StringComparison.Ordinal);
        Assert.Contains("Wrath of God", text, StringComparison.Ordinal);
    }
}
