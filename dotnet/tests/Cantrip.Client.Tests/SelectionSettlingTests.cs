using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Tests;

/// <summary>
/// An answer to a choice of cards is settled with the host before it is sent (<see cref="GameClient.Answer"/>): checked
/// (<see cref="CheckSelectionMessage"/>), sent once the host calls it an answer, and mended a card at a time from what the
/// host says may be added when it does not - so a player that picks without knowing the question's rule still answers
/// what the rules take.
/// </summary>
public sealed class SelectionSettlingTests
{
    private static ViewHandle H(int value) => new(value);

    /// <summary>A transport whose host judges every check with <paramref name="rule"/>, as the native host does with the question's own rule.</summary>
    private sealed class Judging(Func<IReadOnlyList<ViewHandle>, SelectionVerdictView> rule) : IGameTransport
    {
        public Queue<ServerMessage> Inbound { get; } = [];

        public List<ClientMessage> Sent { get; } = [];

        public bool IsConnected => true;

        public void Send(ClientMessage message)
        {
            Sent.Add(message);
            if (message is CheckSelectionMessage check)
            {
                Inbound.Enqueue(new SelectionVerdictMessage { RequestId = check.RequestId, Selected = check.Selected, Verdict = rule(check.Selected) });
            }
        }

        public IReadOnlyList<ServerMessage> Receive()
        {
            List<ServerMessage> messages = [.. Inbound];
            Inbound.Clear();
            return messages;
        }

        public void Dispose()
        {
        }
    }

    /// <summary>
    /// "Exactly two cards with different names" over cards 1 to 4, where 1 is a split card sharing a name with both 2 and 3,
    /// and 2 and 4 share theirs (CR 201.2a-b).
    /// </summary>
    private static SelectionVerdictView TwoWithDifferentNames(IReadOnlyList<ViewHandle> picked)
    {
        HashSet<(int, int)> same = [(1, 2), (2, 1), (1, 3), (3, 1), (2, 4), (4, 2)];
        bool apart(ViewHandle a, ViewHandle b) => !same.Contains((a.Value, b.Value)) && a != b;
        bool valid = picked.Select((card, i) => picked.Take(i).All(other => apart(card, other))).All(ok => ok);
        List<ViewHandle> addable = picked.Count >= 2 ? [] : [.. new[] { H(1), H(2), H(3), H(4) }.Where(card => !picked.Contains(card) && picked.All(other => apart(card, other)))];
        return new SelectionVerdictView { Complete = valid && picked.Count == 2, Addable = addable };
    }

    private static (GameClient Client, Judging Transport) Asked(Func<IReadOnlyList<ViewHandle>, SelectionVerdictView> rule, bool noneIsAnswer = false)
    {
        var transport = new Judging(rule);
        var client = new GameClient(transport);
        transport.Inbound.Enqueue(new PromptMessage
        {
            RequestId = 7,
            Prompt = new SearchLibraryPromptMessage
            {
                Player = 1,
                Count = 2,
                CountKind = noneIsAnswer ? CountKindView.UpTo : CountKindView.Exactly,
                Library = [H(1), H(2), H(3), H(4)],
                Words = ["with different names"],
                Verdict = rule([]) is var first ? new SelectionVerdictView { Complete = noneIsAnswer || first.Complete, Addable = first.Addable } : new(),
            },
        });
        client.Pump();
        return (client, transport);
    }

    private static List<ViewHandle>? Answered(Judging transport) =>
        transport.Sent.OfType<AnswerMessage>().LastOrDefault()?.Answer is CardsChosenAnswer chosen ? chosen.Cards : null;

    [Fact]
    public void An_answer_the_host_takes_is_sent_once_checked()
    {
        (GameClient client, Judging transport) = Asked(TwoWithDifferentNames);
        client.Answer(new CardsChosenAnswer { Cards = [H(2), H(3)] });

        Assert.False(client.IsWaitingOnMe);
        Assert.True(client.IsSettling);
        Assert.Null(Answered(transport));
        Assert.Single(transport.Sent.OfType<CheckSelectionMessage>());

        client.Pump();
        Assert.False(client.IsSettling);
        Assert.Equal([H(2), H(3)], Answered(transport));
    }

    [Fact]
    public void An_answer_the_host_refuses_is_mended_from_what_may_be_added_keeping_the_cards_meant_first()
    {
        // 2 and 4 share a name: 2 is kept, 4 is not addable beside it, and 3 - the first card of the question's own order
        // that is - takes its place.
        (GameClient client, Judging transport) = Asked(TwoWithDifferentNames);
        client.Answer(new CardsChosenAnswer { Cards = [H(2), H(4)] });
        for (int i = 0; i < 10 && client.IsSettling; i++) client.Pump();

        Assert.Equal([H(2), H(3)], Answered(transport));
    }

    [Fact]
    public void Mending_keeps_a_card_meant_and_takes_the_first_that_may_join_it()
    {
        // The split card 1 shares a name with 2: 1 is kept, and 4, the first card that may join it, takes 2's place.
        (GameClient client, Judging transport) = Asked(TwoWithDifferentNames);
        client.Answer(new CardsChosenAnswer { Cards = [H(1), H(2)] });
        for (int i = 0; i < 20 && client.IsSettling; i++) client.Pump();

        Assert.Equal([H(1), H(4)], Answered(transport));
    }

    [Fact]
    public void Mending_goes_back_a_card_where_nothing_more_may_be_added()
    {
        // Here the split card 1 shares a name with every other card: kept, it reaches no answer, so mending gives it up and
        // takes the next card meant (2) and the first that may join it (3).
        static SelectionVerdictView Rule(IReadOnlyList<ViewHandle> picked) =>
            picked.Contains(H(1)) && picked.Count > 1 ? new SelectionVerdictView()
            : picked.Contains(H(1)) ? new SelectionVerdictView { Addable = [] }
            : TwoWithDifferentNames(picked);
        (GameClient client, Judging transport) = Asked(Rule);
        client.Answer(new CardsChosenAnswer { Cards = [H(1), H(2)] });
        for (int i = 0; i < 20 && client.IsSettling; i++) client.Pump();

        Assert.Equal([H(2), H(3)], Answered(transport));
    }

    [Fact]
    public void None_where_none_is_an_answer_goes_at_once_without_a_check()
    {
        (GameClient client, Judging transport) = Asked(TwoWithDifferentNames, noneIsAnswer: true);
        client.Answer(new CardsChosenAnswer());

        Assert.Empty(transport.Sent.OfType<CheckSelectionMessage>());
        Assert.Equal([], Answered(transport));
    }

    [Fact]
    public void A_selection_judged_while_picking_is_answered_without_asking_again()
    {
        (GameClient client, Judging transport) = Asked(TwoWithDifferentNames);
        client.Check([H(3), H(4)]);
        client.Check([H(4), H(3)]);
        client.Pump();
        Assert.Single(transport.Sent.OfType<CheckSelectionMessage>());
        Assert.True(client.Verdict([H(4), H(3)])!.Complete);

        client.Answer(new CardsChosenAnswer { Cards = [H(3), H(4)] });
        Assert.Equal([H(3), H(4)], Answered(transport));
    }

    private static ErrorMessage Refusal(int requestId) => new() { Code = ErrorCode.StaleReply, Detail = "refused", RequestId = requestId };

    [Fact]
    public void An_error_that_is_not_about_the_check_does_not_send_the_proposal()
    {
        (GameClient client, Judging transport) = Asked(TwoWithDifferentNames);
        client.Answer(new CardsChosenAnswer { Cards = [H(2), H(4)] });
        Assert.True(client.IsChecking);

        // An error about nothing in particular, and one about another request: neither is this check's.
        transport.Inbound.Clear();
        transport.Inbound.Enqueue(Refusal(0));
        transport.Inbound.Enqueue(Refusal(99));
        client.Pump();

        Assert.True(client.IsSettling);
        Assert.Null(Answered(transport));
    }

    /// <summary>
    /// The check of its own request refused never lets the unsettled proposal go: nothing is sent. A question no longer
    /// outstanding (or not this seat's) is dropped; any other refusal puts the question back to be answered anew.
    /// </summary>
    [Theory]
    [InlineData(ErrorCode.StaleReply, false)]
    [InlineData(ErrorCode.NotYourTurnToAnswer, false)]
    [InlineData(ErrorCode.UnknownHandle, true)]
    [InlineData(ErrorCode.IllegalAnswer, true)]
    public void The_check_of_its_own_request_refused_sends_nothing(ErrorCode code, bool askedAgain)
    {
        (GameClient client, Judging transport) = Asked(TwoWithDifferentNames);
        client.Answer(new CardsChosenAnswer { Cards = [H(2), H(4)] });
        transport.Inbound.Clear();
        transport.Inbound.Enqueue(new ErrorMessage { Code = code, Detail = "refused", RequestId = 7 });
        client.Pump();

        Assert.False(client.IsSettling);
        Assert.Null(Answered(transport));
        Assert.Single(client.Mends);
        Assert.Equal(askedAgain, client.Prompt?.RequestId == 7);
    }

    [Fact]
    public void A_mend_is_logged_with_the_request_the_proposal_and_what_was_sent()
    {
        (GameClient client, Judging transport) = Asked(TwoWithDifferentNames);
        List<string> told = [];
        client.MendedSelection += told.Add;
        client.Answer(new CardsChosenAnswer { Cards = [H(2), H(4)] });
        for (int i = 0; i < 10 && client.IsSettling; i++) client.Pump();

        Assert.Equal(2, told.Count);
        Assert.Contains("request 7", told[0]);
        Assert.Contains("#2", told[0]);
        Assert.Contains("#4", told[0]);
        Assert.Contains("proposed [#2, #4], sent [#2, #3]", told[1]);
        Assert.Equal(told, client.Mends);

        // An answer the host takes as it is is no mend.
        (GameClient clean, Judging cleanTransport) = Asked(TwoWithDifferentNames);
        clean.Answer(new CardsChosenAnswer { Cards = [H(2), H(3)] });
        clean.Pump();
        Assert.Empty(clean.Mends);
    }

    [Fact]
    public void Mending_keeps_the_cards_meant_and_adds_none_the_player_never_picked_once_they_are_an_answer()
    {
        // Up to two of 1 to 4, but 1 and 2 may not be together. Meant: 1 and 2. 1 alone is an answer, and 3 and 4 could join it:
        // mending sends 1, the card meant, and does not grow it into 1 and 3.
        static SelectionVerdictView Rule(IReadOnlyList<ViewHandle> picked)
        {
            bool clash = picked.Contains(H(1)) && picked.Contains(H(2));
            List<ViewHandle> addable = clash || picked.Count >= 2 ? [] : [.. new[] { H(1), H(2), H(3), H(4) }
                .Where(card => !picked.Contains(card) && !(card == H(2) && picked.Contains(H(1))) && !(card == H(1) && picked.Contains(H(2))))];
            return new SelectionVerdictView { Complete = !clash, Addable = addable };
        }

        (GameClient client, Judging transport) = Asked(Rule, noneIsAnswer: true);
        client.Answer(new CardsChosenAnswer { Cards = [H(1), H(2)] });
        for (int i = 0; i < 20 && client.IsSettling; i++) client.Pump();

        Assert.Equal([H(1)], Answered(transport));
    }

    /// <summary>
    /// Teamwork announced, then the creature that could have paid it tapped for the spell's mana: asked for total power 2,
    /// nothing reaches it - no creature at all, or one of power 1. Mending finds no answer, and a question that may be
    /// cancelled is: the announcement is given up, rather than sending a choice the rules refuse.
    /// </summary>
    [Theory]
    [InlineData(new int[0])]
    [InlineData(new[] { 1 })]
    public void A_total_power_out_of_reach_is_cancelled_where_the_question_may_be(int[] powers)
    {
        List<ViewHandle> cards = [.. powers.Select((_, i) => H(i + 1))];
        SelectionVerdictView Rule(IReadOnlyList<ViewHandle> picked) =>
            new() { Complete = false, Addable = [.. cards.Where(card => !picked.Contains(card))] };
        var transport = new Judging(Rule);
        var client = new GameClient(transport);
        transport.Inbound.Enqueue(new PromptMessage
        {
            RequestId = 11,
            MayCancel = true,
            Prompt = new ChooseCardsPromptMessage
            {
                Player = 1,
                Purpose = CardChoicePurposeView.TapToPay,
                CountKind = CountKindView.AnyNumber,
                Count = cards.Count,
                Cards = cards,
                MinimumPower = 2,
                Powers = [.. powers],
                Verdict = Rule([]),
            },
        });
        client.Pump();
        client.Answer(new CardsChosenAnswer { Cards = [] });
        for (int i = 0; i < 10 && client.IsSettling; i++) client.Pump();

        Assert.False(client.IsSettling);
        Assert.IsType<CancelAnnouncementAnswer>(Assert.Single(transport.Sent.OfType<AnswerMessage>()).Answer);
        Assert.Contains("the announcement given up", client.Mends[^1]);
    }

    [Fact]
    public void A_one_card_answer_the_prompt_calls_complete_alone_is_sent_without_a_check()
    {
        var transport = new Judging(TwoWithDifferentNames);
        var client = new GameClient(transport);
        transport.Inbound.Enqueue(new PromptMessage
        {
            RequestId = 9,
            Prompt = new ChooseCardsPromptMessage
            {
                Player = 1,
                Count = 1,
                CountKind = CountKindView.Exactly,
                Cards = [H(1), H(2), H(3)],
                Verdict = new SelectionVerdictView { Addable = [H(1), H(2), H(3)], CompleteAlone = [H(1), H(2)] },
            },
        });
        client.Pump();

        Assert.True(client.Verdict([H(2)])!.Complete);
        Assert.Empty(transport.Sent.OfType<CheckSelectionMessage>());
        client.Answer(new CardsChosenAnswer { Cards = [H(2)] });
        Assert.Empty(transport.Sent.OfType<CheckSelectionMessage>());
        Assert.Equal([H(2)], Answered(transport));

        // A card the prompt does not mark waits for the host's word, as does any other selection.
        transport.Inbound.Enqueue(new PromptMessage
        {
            RequestId = 10,
            Prompt = new ChooseCardsPromptMessage
            {
                Player = 1, Count = 1, CountKind = CountKindView.Exactly, Cards = [H(1), H(2), H(3)],
                Verdict = new SelectionVerdictView { Addable = [H(1), H(2), H(3)], CompleteAlone = [H(1)] },
            },
        });
        client.Pump();
        Assert.Null(client.Verdict([H(3)]));
        client.Answer(new CardsChosenAnswer { Cards = [H(3)] });
        Assert.Single(transport.Sent.OfType<CheckSelectionMessage>());
    }

    [Fact]
    public void The_default_answer_of_a_total_mana_value_choice_fits_within_the_limit()
    {
        // Lively Dirge: up to two, total mana value 4 or less, a 3 and a 2 offered first - together 5, which the rules refuse.
        var prompt = new ChooseCardsPromptMessage
        {
            Player = 1,
            Cards = [H(1), H(2), H(3)],
            Count = 2,
            CountKind = CountKindView.UpTo,
            ManaValues = [3, 2, 1],
            MaximumTotalManaValue = 4,
            Verdict = new SelectionVerdictView { Complete = true, Addable = [H(1), H(2), H(3)] },
        };

        var chosen = Assert.IsType<CardsChosenAnswer>(prompt.Default());
        Assert.Equal([H(1), H(3)], chosen.Cards);
    }

    /// <summary>"Any number of target creature cards that each have a different mana value" over objects 1 to 3, 1 and 2 sharing one.</summary>
    private static SelectionVerdictView DifferentManaValues(IReadOnlyList<ViewHandle> picked)
    {
        static bool Clash(ViewHandle a, ViewHandle b) => (a.Value, b.Value) is (1, 2) or (2, 1);
        bool valid = picked.Select((card, i) => picked.Take(i).All(other => !Clash(card, other))).All(ok => ok);
        List<ViewHandle> addable = [.. new[] { H(1), H(2), H(3) }.Where(card => !picked.Contains(card) && picked.All(other => !Clash(card, other)))];
        return new SelectionVerdictView { Complete = valid, Addable = addable };
    }

    private static TargetMessage Seat(int seat) => new() { Player = seat };

    private static TargetMessage Object(int handle) => new() { Object = H(handle) };

    [Fact]
    public void A_choice_of_targets_judged_together_is_settled_with_the_host_and_keeps_the_players_chosen()
    {
        var transport = new Judging(DifferentManaValues);
        var client = new GameClient(transport);
        transport.Inbound.Enqueue(new PromptMessage
        {
            RequestId = 9,
            Prompt = new ChooseTargetPromptMessage
            {
                Player = 1,
                Candidates = [Seat(2), Object(1), Object(2), Object(3)],
                Optional = true,
                Count = 4,
                AnyNumber = true,
                Words = ["with different mana values"],
                Verdict = DifferentManaValues([]),
            },
        });
        client.Pump();

        // 1 and 2 share a mana value: the host refuses them together, 1 is kept and nothing else joins it.
        client.Answer(new TargetsChosenAnswer { Targets = [Seat(2), Object(1), Object(2)] });
        Assert.True(client.IsSettling);
        for (int i = 0; i < 10 && client.IsSettling; i++) client.Pump();

        var sent = Assert.IsType<TargetsChosenAnswer>(transport.Sent.OfType<AnswerMessage>().Last().Answer);
        Assert.Equal([(2, 0), (0, 1)], sent.Targets.Select(target => (target.Player, target.Object.Value)));
        Assert.Contains(transport.Sent.OfType<CheckSelectionMessage>(), check => check.Selected.SequenceEqual([H(1), H(2)]));
    }

    [Fact]
    public void A_choice_of_targets_the_slot_does_not_judge_together_is_sent_as_it_is()
    {
        var transport = new Judging(_ => throw new InvalidOperationException("nothing is checked"));
        var client = new GameClient(transport);
        transport.Inbound.Enqueue(new PromptMessage
        {
            RequestId = 9,
            Prompt = new ChooseTargetPromptMessage { Player = 1, Candidates = [Object(1), Object(2)], Count = 2 },
        });
        client.Pump();
        client.Answer(new TargetsChosenAnswer { Targets = [Object(1), Object(2)] });

        Assert.False(client.IsSettling);
        Assert.Empty(transport.Sent.OfType<CheckSelectionMessage>());
        Assert.IsType<TargetsChosenAnswer>(transport.Sent.OfType<AnswerMessage>().Single().Answer);
    }

    [Fact]
    public void The_default_answer_of_targets_judged_together_takes_only_what_may_be_picked()
    {
        var prompt = new ChooseTargetPromptMessage
        {
            Player = 1,
            Candidates = [Object(1), Object(2), Object(3)],
            Count = 2,
            Verdict = new SelectionVerdictView { Complete = false, Addable = [H(2), H(3)] },
        };

        var chosen = Assert.IsType<TargetsChosenAnswer>(prompt.Default());
        Assert.Equal([2, 3], chosen.Targets.Select(target => target.Object.Value));
    }
}
