using Cantrip.Protocol;
using Xunit;
using static Cantrip.Client.Tests.TacticsBoard;

namespace Cantrip.Client.Tests;

/// <summary>
/// What the dock says of a choice of cards: the verb is the card's own (exile is exile, a free cast is a free cast), the kind of
/// card is named where the asking card named one, and "any number" is said where the rules text says it.
/// </summary>
public sealed class CardChoiceCaptionTests
{
    private static ObjectView Card(int handle, string name, ZoneKindView zone, int owner, CardTypeView type = CardTypeView.Creature) => new()
    {
        Handle = Handle(handle),
        Zone = zone,
        ZoneOwner = zone == ZoneKindView.Battlefield ? 0 : owner,
        Kind = ObjectKindView.Card,
        Owner = owner,
        Controller = owner,
        Name = name,
        CardTypes = [type],
    };

    private static ChooseCardsPromptMessage Choice(CardChoicePurposeView purpose, int count, bool upTo, params ObjectView[] cards) => new()
    {
        Player = Me,
        Owner = Me,
        Source = Handle(90),
        Purpose = purpose,
        Cards = [.. cards.Select(card => card.Handle)],
        Verdict = new() { Addable = [.. cards.Select(card => card.Handle)], Complete = upTo },
        Count = count,
        CountKind = upTo ? CountKindView.UpTo : CountKindView.Exactly,
    };

    private static ObjectView Spell() => new()
    {
        Handle = Handle(90),
        Zone = ZoneKindView.Stack,
        Kind = ObjectKindView.Card,
        Owner = Me,
        Controller = Me,
        Name = "Strategic Betrayal",
        CardTypes = [CardTypeView.Sorcery],
    };

    [Fact]
    public void An_exile_of_creatures_on_the_battlefield_says_creature_and_exile()
    {
        ObjectView elves = Card(3, "Llanowar Elves", ZoneKindView.Battlefield, Them);
        GameClient client = Seated([Spell(), elves]);
        ChooseCardsPromptMessage prompt = Choice(CardChoicePurposeView.Exile, 1, false, elves);
        prompt.Kind = "creature";

        CardChoicePresentation shown = CardChoicePresentation.Describe(client, prompt, []);

        Assert.Equal("Strategic Betrayal: choose a creature to exile", shown.Title);
        Assert.Equal("Exile nothing", shown.Confirm);
    }

    [Fact]
    public void An_exile_from_a_hand_names_the_card_kind_and_says_card()
    {
        ObjectView elves = Card(3, "Llanowar Elves", ZoneKindView.Hand, Them);
        GameClient client = Seated([Spell(), elves]);
        ChooseCardsPromptMessage prompt = Choice(CardChoicePurposeView.Exile, 1, false, elves);
        prompt.Kind = "artifact or creature";

        Assert.Equal("Strategic Betrayal: choose an artifact or creature card to exile", CardChoicePresentation.Describe(client, prompt, []).Title);
    }

    [Fact]
    public void Any_number_is_said_where_the_rules_text_names_no_number()
    {
        ObjectView one = Card(3, "Ornithopter", ZoneKindView.Graveyard, Them, CardTypeView.Artifact);
        ObjectView two = Card(4, "Ornithopter", ZoneKindView.Graveyard, Them, CardTypeView.Artifact);
        GameClient client = Seated([Spell(), one, two]);

        // "Any number of" is said so by the host (`AnyNumber`), its count every eligible card.
        ChooseCardsPromptMessage anyNumber = Choice(CardChoicePurposeView.Exile, 2, true, one, two);
        anyNumber.CountKind = CountKindView.AnyNumber;
        Assert.Equal(
            "Strategic Betrayal: choose any number of cards to exile",
            CardChoicePresentation.Describe(client, anyNumber, []).Title);

        // A count the rules text gave is "up to" that many, even when every card shown could be chosen.
        Assert.Equal(
            "Strategic Betrayal: choose up to 2 cards to exile",
            CardChoicePresentation.Describe(client, Choice(CardChoicePurposeView.Exile, 2, true, one, two), []).Title);
    }

    [Fact]
    public void A_cast_without_paying_says_so_and_a_cast_of_copies_says_copies_and_what_is_left_to_spend()
    {
        ObjectView giant = Card(3, "Bonecrusher Giant", ZoneKindView.Exile, Me);
        GameClient client = Seated([Spell(), giant]);

        Assert.Equal(
            "Strategic Betrayal: choose up to 1 card to cast without paying its mana cost",
            CardChoicePresentation.Describe(client, Choice(CardChoicePurposeView.CastFree, 1, true, giant), []).Title);

        ChooseCardsPromptMessage copies = Choice(CardChoicePurposeView.CastCopyFree, 1, true, giant);
        copies.MaximumManaValue = 4;
        CardChoicePresentation shown = CardChoicePresentation.Describe(client, copies, []);
        Assert.Equal(
            "Strategic Betrayal: choose up to 1 copy to cast without paying its mana cost (total mana value 4 or less left to spend)",
            shown.Title);
        Assert.Equal("Cast nothing", shown.Confirm);

        Assert.Equal(
            "Strategic Betrayal: choose up to 1 card to cast, paying its costs",
            CardChoicePresentation.Describe(client, Choice(CardChoicePurposeView.Cast, 1, true, giant), []).Title);
    }

    [Fact]
    public void A_choice_of_the_permanent_to_put_counters_on_says_which_counters_and_never_keep()
    {
        ObjectView token = Card(3, "Jace", ZoneKindView.Battlefield, Me, CardTypeView.Planeswalker);
        ObjectView copy = Card(4, "Jace, the Mind Sculptor", ZoneKindView.Battlefield, Me, CardTypeView.Planeswalker);
        GameClient client = Seated([Spell(), token, copy]);
        ChooseCardsPromptMessage prompt = Choice(CardChoicePurposeView.PutCounters, 1, false, token, copy);
        prompt.Kind = "Jace planeswalker token";
        prompt.Counter = "loyalty";
        prompt.CounterCount = 2;

        CardChoicePresentation shown = CardChoicePresentation.Describe(client, prompt, []);

        Assert.Equal("Strategic Betrayal: choose a Jace planeswalker token to put 2 loyalty counters on", shown.Title);
        Assert.Equal("Put 2 loyalty counters on it", shown.Confirm);
        // One permanent on the battlefield, chosen as a target is: the click on it answers.
        Assert.True(shown.AnswersOnLastPick);

        // Amass's Army, one +1/+1 counter.
        ChooseCardsPromptMessage army = Choice(CardChoicePurposeView.PutCounters, 1, false, token, copy);
        army.Kind = "Army";
        army.Counter = "+1/+1";
        army.CounterCount = 1;
        CardChoicePresentation amass = CardChoicePresentation.Describe(client, army, []);
        Assert.Equal("Strategic Betrayal: choose an Army to put a +1/+1 counter on", amass.Title);
        Assert.Equal("Put a +1/+1 counter on it", amass.Confirm);
    }

    /// <summary>
    /// An effect's own choice as it resolves (CR 608.2d), with nothing yet done to what is chosen: Spry and Mighty's "choose
    /// exactly two creatures you control". The caption names what is chosen and no verb, and the button says how many.
    /// </summary>
    [Fact]
    public void A_choice_that_does_nothing_yet_names_what_is_chosen_and_no_verb()
    {
        ObjectView elves = Card(3, "Llanowar Elves", ZoneKindView.Battlefield, Me);
        ObjectView bear = Card(4, "Grizzly Bears", ZoneKindView.Battlefield, Me);
        GameClient client = Seated([Spell(), elves, bear]);
        ChooseCardsPromptMessage prompt = Choice(CardChoicePurposeView.Choose, 2, false, elves, bear);
        prompt.Kind = "creature";

        Assert.Equal("Strategic Betrayal: choose 2 creatures", CardChoicePresentation.Describe(client, prompt, []).Title);
        Assert.Equal("Choose it", CardChoicePresentation.Describe(client, prompt, [3]).Confirm);
        Assert.Equal("Choose 2", CardChoicePresentation.Describe(client, prompt, [3, 4]).Confirm);
    }

    /// <summary>
    /// Craft's materials (CR 702.167a) are permanents you control and cards in your graveyard, exiled with the crafting
    /// permanent: the caption says both zones and the quality, "one or more" is said as such, and fewer than that is no answer.
    /// </summary>
    [Fact]
    public void Craft_materials_say_both_zones_and_one_or_more_needs_one()
    {
        ObjectView bears = Card(3, "Grizzly Bears", ZoneKindView.Battlefield, Me);
        ObjectView elves = Card(4, "Llanowar Elves", ZoneKindView.Graveyard, Me);
        GameClient client = Seated([Spell(), bears, elves]);

        ChooseCardsPromptMessage one = Choice(CardChoicePurposeView.Craft, 1, false, bears, elves);
        one.Kind = "creature";
        CardChoicePresentation exactly = Judged(client, one, [3]);
        Assert.Equal("Strategic Betrayal: craft - choose a creature you control or a creature card from your graveyard to exile", exactly.Title);
        Assert.Equal("Exile it", exactly.Confirm);
        Assert.True(exactly.CanSubmit);

        ChooseCardsPromptMessage more = Choice(CardChoicePurposeView.Craft, 2, true, bears, elves);
        more.Kind = "creature";
        more.Minimum = 1;
        more.Verdict.Complete = false;
        Assert.Equal(
            "Strategic Betrayal: craft - choose 1 or more from among creatures you control and/or creature cards in your graveyard to exile",
            CardChoicePresentation.Describe(client, more, []).Title);
        Assert.False(CardChoicePresentation.Describe(client, more, []).CanSubmit);
        Assert.True(Judged(client, more, [3, 4]).CanSubmit);
        Assert.Equal("Exile 2", CardChoicePresentation.Describe(client, more, [3, 4]).Confirm);

        ChooseCardsPromptMessage two = Choice(CardChoicePurposeView.Craft, 2, false, bears, elves);
        two.Kind = "creature";
        Assert.Equal(
            "Strategic Betrayal: craft - choose 2 from among creatures you control and/or creature cards in your graveyard to exile",
            CardChoicePresentation.Describe(client, two, []).Title);
        Assert.False(Judged(client, two, [3]).CanSubmit);

        // Named as cards, the materials are graveyard cards alone (CR 702.167b): Ore-Rich Stalactite's four or more.
        ChooseCardsPromptMessage cards = Choice(CardChoicePurposeView.Craft, 2, true, elves);
        cards.Kind = "red instant and/or sorcery card";
        cards.Minimum = 1;
        Assert.Equal(
            "Strategic Betrayal: craft - choose 1 or more from among red instant and/or sorcery cards in your graveyard to exile",
            CardChoicePresentation.Describe(client, cards, []).Title);
    }
    /// <summary>
    /// A card revealed from among those looked at (Break Out) may then go into the hand or onto the battlefield, so the caption
    /// says only what is true of the choice: it is revealed.
    /// </summary>
    [Fact]
    public void Reveal_says_reveal_and_not_where_the_card_goes()
    {
        ObjectView mystic = Card(3, "Elvish Mystic", ZoneKindView.Library, Me);
        GameClient client = Seated([Spell(), mystic]);

        ChooseCardsPromptMessage reveal = Choice(CardChoicePurposeView.Reveal, 1, true, mystic);
        reveal.Kind = "creature";
        CardChoicePresentation shown = CardChoicePresentation.Describe(client, reveal, [3]);
        Assert.Contains("reveal", shown.Title);
        Assert.DoesNotContain("hand", shown.Title);
        Assert.Equal("Reveal it", shown.Confirm);
        Assert.Equal("Reveal nothing", CardChoicePresentation.Describe(client, reveal, []).Confirm);
    }

    /// <summary>
    /// A card put into the hand from outside the game (North Wind Avatar, CR 400.11b) comes from the chooser's sideboard,
    /// which nothing on the table shows: the caption says so, and the tray is named for it.
    /// </summary>
    [Fact]
    public void A_card_from_outside_the_game_says_so_and_names_the_sideboard()
    {
        ObjectView bolt = Card(3, "Lightning Bolt", ZoneKindView.Sideboard, Me, CardTypeView.Instant);
        ObjectView counterspell = Card(4, "Counterspell", ZoneKindView.Sideboard, Me, CardTypeView.Instant);
        GameClient client = Seated([Spell(), bolt, counterspell]);

        ChooseCardsPromptMessage fetch = Choice(CardChoicePurposeView.PutIntoHand, 1, true, bolt, counterspell);
        CardChoicePresentation shown = CardChoicePresentation.Describe(client, fetch, []);
        Assert.Equal("Strategic Betrayal: choose up to 1 card you own from outside the game to put into your hand", shown.Title);
        Assert.Equal("0 of 1 chosen  ·  from your sideboard, which only you can see", shown.Detail);
        Assert.StartsWith("Your sideboard", shown.TrayTitle, StringComparison.Ordinal);
        Assert.Equal("Take nothing", shown.Confirm);
        Assert.Equal("Put into your hand", CardChoicePresentation.Describe(client, fetch, [3]).Confirm);
    }
}
