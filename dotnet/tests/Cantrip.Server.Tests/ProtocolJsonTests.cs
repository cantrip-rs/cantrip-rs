using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using Cantrip.Protocol;
using Cantrip.Server.Rules;
using MessagePack;
using Xunit;

namespace Cantrip.Server.Tests;

/// <summary>
/// The protocol as the native host writes it (<c>docs/host-protocol.md</c>): PascalCase members, enums by name,
/// handles as bare integers, unions tagged <c>$type</c> with the C# class name. Every prompt, answer and event kind has
/// a hand-written sample in the doc's shape, read into <c>Cantrip.Protocol</c> and written back, so a member the two
/// sides spell differently fails here rather than as a game the sidecar cannot start.
/// </summary>
public sealed class ProtocolJsonTests
{
    public static TheoryData<string> Prompts =>
    [
        """{"$type":"PriorityPromptMessage","Player":1,"Options":[{"Kind":"PlayLand","Source":3,"AbilityIndex":0,"TargetsRequired":0,"TargetCandidates":[],"Produces":["Green"],"Affordable":true,"ProducesOfAnyOneColor":0,"HasX":false,"MaxX":-1,"Description":"","LifeCost":0,"DamageToController":0,"ColorChoices":[],"CostText":"","TargetsOptional":false,"Filter":"None","Effect":"None","SpellName":"","ProducesOfAnyColorEach":0,"SacrificeCandidates":[],"Tap":null,"PaysUpTo":0,"Alternative":0,"LifeGained":0,"GrantedBy":""},{"Kind":"CastSpell","Source":4,"TargetsRequired":1,"TargetCandidates":[{"Player":2,"Object":0},{"Player":0,"Object":3}],"Tap":5}]}""",
        """{"$type":"DiscardPromptMessage","Player":1,"Count":2,"MaximumHandSize":7,"Hand":[5,6,7]}""",
        """{"$type":"DeclareAttackersPromptMessage","Player":1,"Eligible":[3,4],"AttackablePlayers":[2],"AttackablePlaneswalkers":[9],"AttackableBattles":[],"Requirements":[{"Attacker":3,"TargetPlayer":0,"TargetPermanent":0,"OtherThan":2}],"Forbidden":[{"Attacker":4,"TargetPlayer":0,"TargetPermanent":9}]}""",
        """{"$type":"DeclareBlockersPromptMessage","Player":2,"Eligible":[5],"Attackers":[3],"Allowed":[{"Blocker":5,"Attacker":3}],"MinimumBlockers":[1]}""",
        """{"$type":"AssignCombatDamagePromptMessage","Player":1,"Divisions":[{"Source":3,"Damage":4,"Creatures":[5,6],"Lethal":[1,2],"MayAssignToDefender":true,"Defender":2,"Suggested":[1,3],"SuggestedToDefender":0,"DefenderPermanent":0}]}""",
        """{"$type":"SearchLibraryPromptMessage","Player":1,"Count":1,"Library":[10,11,12],"Destination":"Hand","CountKind":"UpTo","Kind":"basic land","Words":[],"Verdict":{"Complete":true,"Addable":[11]}}""",
        """{"$type":"PayCostsPromptMessage","Player":1,"Spell":4,"Total":"{1}{R}","Remaining":"{R}","RemainingGeneric":0,"RemainingColored":["Red"],"Options":[{"Kind":"CancelCasting","Source":0}],"HasX":false,"X":0,"Committed":[{"Type":"Red","Amount":1,"Filter":"None","Effect":"None"}],"RemainingHybrid":[{"Colors":["Blue","Red"],"LifeCost":0}],"Kicker":"","Multikicker":false,"Buyback":false,"Squad":false,"AnnouncesXOrKicks":false,"MostAffordable":-1,"XPaidInLife":false,"RemainingColorless":0,"KickerPaid":false,"VariableKicker":false,"SpendsNoMana":false}""",
        """{"$type":"ChooseCardsPromptMessage","Player":1,"Owner":2,"Source":4,"Purpose":"Discard","Cards":[5,6],"Count":1,"CountKind":"Exactly","Words":["with different names"],"Verdict":{"Complete":false,"Addable":[6]},"Kind":"","MinimumPower":0,"Powers":[],"MinimumManaValue":0,"ManaValues":[]}""",
        """{"$type":"UnlessPaysPromptMessage","Player":2,"Source":4,"Subject":3,"Total":"{3}","Remaining":"{3}","RemainingGeneric":3,"RemainingColored":[],"Options":[{"Kind":"DeclineCost","Source":0}],"RemainingHybrid":[],"Verb":""}""",
        """{"$type":"DistributeCardsPromptMessage","Player":1,"Source":4,"Cards":[10,11],"Destinations":[{"Kind":"LibraryTop","Owner":1,"Min":0,"Max":2147483647,"Eligible":[10,11],"Ordered":true,"Randomized":false,"Groups":[],"GroupNames":[]},{"Kind":"LibraryBottom","Owner":1,"Min":0,"Max":2147483647,"Eligible":[10,11],"Ordered":true,"Randomized":false,"Groups":[[10],[11]],"GroupNames":["Creature","Land"]}],"MayShuffle":true}""",
        """{"$type":"ChooseTargetPromptMessage","Player":1,"Ability":8,"Source":3,"Text":"destroy target creature","Candidates":[{"Player":2,"Object":0},{"Player":0,"Object":5}],"Optional":false,"Count":1,"Targeted":true,"Minimum":0,"AnyNumber":false}""",
        """{"$type":"MayPromptMessage","Player":1,"Source":7,"Verb":"SearchLibrary","Amount":0}""",
        """{"$type":"ChooseModesPromptMessage","Player":1,"Source":4,"Modes":["a","b"],"Eligible":[0,1],"Count":1,"MayRepeat":false,"AdditionalCost":false,"Heading":"","AtMost":0,"Affordable":[0]}""",
        """{"$type":"ChooseReplacementPromptMessage","Player":1,"Subject":3,"Event":"enters the battlefield","Sources":[4,5],"Hints":["tapped","with a counter"],"SourceNames":["Hardened Scales","Corpsejack Menace"]}""",
        """{"$type":"OrderTriggersPromptMessage","Player":1,"Sources":[4,5],"Texts":["draw a card","gain 1 life"]}""",
        """{"$type":"NameCardPromptMessage","Player":1,"Source":4,"Owner":2,"Hand":[6,7],"Names":["Island","Lightning Bolt"],"Suggestion":"Lightning Bolt","Kind":"a card name other than a basic land card name"}""",
        """{"$type":"ChoosePlayerPromptMessage","Player":1,"Source":4,"Candidates":[2,3],"Verb":"Draw","Amount":1,"Subject":0}""",
        """{"$type":"ChooseColorsPromptMessage","Player":1,"Source":4,"Choices":["White","Blue","Black","Red","Green"],"Count":1,"MayRepeat":false,"Verb":"AddMana"}""",
        """{"$type":"ChooseDividedTargetsPromptMessage","Player":1,"Ability":4,"Source":4,"Text":"divided as you choose","Candidates":[{"Player":2,"Object":0},{"Player":0,"Object":5}],"Total":3,"MaxTargets":0,"Optional":false}""",
        """{"$type":"ChooseStartingPlayerPromptMessage","Player":2,"Candidates":[1,2]}""",
        """{"$type":"MulliganPromptMessage","Player":1,"Mulligans":1,"Keeps":6}""",
        """{"$type":"RemoveCountersPromptMessage","Player":1,"Source":4,"Total":3,"Pools":[{"Object":5,"Counter":"-1/-1","Count":2},{"Object":6,"Counter":"+1/+1","Count":1}],"UpTo":true}""",
    ];

    public static TheoryData<string> Answers =>
    [
        """{"$type":"PassAnswer"}""",
        """{"$type":"CardsChosenAnswer","Cards":[5,6]}""",
        """{"$type":"TakeActionAnswer","Kind":"ActivateAbility","Source":3,"AbilityIndex":1,"Targets":[{"Player":2,"Object":0}],"Color":"Red","Colors":["Red","Green"],"Sacrifice":null,"Tap":5,"Mana":"Green","Filter":"None","Effect":"None"}""",
        """{"$type":"AttackersDeclaredAnswer","Attacks":[{"Attacker":3,"TargetPlayer":2,"TargetPermanent":0}]}""",
        """{"$type":"BlockersDeclaredAnswer","Blocks":[{"Blocker":5,"Attacker":3}]}""",
        """{"$type":"CombatDamageAssignedAnswer","Shares":[{"Source":3,"Creature":5,"Amount":2},{"Source":3,"Creature":0,"Amount":2}]}""",
        """{"$type":"CardsDistributedAnswer","Placements":[[10],[11,12]],"Shuffle":false}""",
        """{"$type":"TargetsChosenAnswer","Targets":[{"Player":0,"Object":5}]}""",
        """{"$type":"MayChosenAnswer","Accept":true}""",
        """{"$type":"ModesChosenAnswer","Modes":[1]}""",
        """{"$type":"ReplacementChosenAnswer","Index":1}""",
        """{"$type":"TriggersOrderedAnswer","Order":[1,0]}""",
        """{"$type":"CancelAnnouncementAnswer"}""",
        """{"$type":"AnnounceAnswer","Value":3}""",
        """{"$type":"CardNamedAnswer","Name":"Lightning Bolt"}""",
        """{"$type":"PlayerChosenAnswer","Player":2}""",
        """{"$type":"ColorsChosenAnswer","Colors":["Blue"]}""",
        """{"$type":"TargetsDividedAnswer","Shares":[{"Target":{"Player":0,"Object":5},"Amount":2},{"Target":{"Player":2,"Object":0},"Amount":1}]}""",
        """{"$type":"CountersRemovedAnswer","Removals":[{"Object":5,"Counter":"-1/-1","Count":2},{"Object":6,"Counter":"+1/+1","Count":1}]}""",
    ];

    public static TheoryData<string> Events =>
    [
        """{"$type":"ObjectMovedEvent","Object":5,"From":"Hand","FromOwner":1,"To":"Battlefield","ToOwner":0,"Reason":"Play"}""",
        """{"$type":"ObjectMovedEvent","Object":0,"From":"Library","FromOwner":2,"To":"Hand","ToOwner":2,"Reason":"Draw"}""",
        """{"$type":"LifeChangedEvent","Player":2,"Delta":-3,"NewTotal":17}""",
        """{"$type":"DamageMarkedEvent","Object":5,"Amount":2}""",
        """{"$type":"TapChangedEvent","Object":3,"Tapped":true}""",
        """{"$type":"StepBeganEvent","Turn":2,"Step":"PrecombatMain","ActivePlayer":2}""",
        """{"$type":"SpellCastEvent","Spell":4,"Controller":1,"Targets":[{"Player":2,"Object":0}],"X":null,"Modes":[],"Kicked":0,"Buyback":false,"GiftPromisedTo":null}""",
        """{"$type":"SpellCastEvent","Spell":4,"Controller":1,"Targets":[],"X":3,"Modes":["a"],"Kicked":1,"Buyback":true,"GiftPromisedTo":2}""",
        """{"$type":"SpellFizzledEvent","Spell":4,"Controller":1}""",
        """{"$type":"LibraryShuffledEvent","Player":1}""",
        """{"$type":"CardsRevealedEvent","Player":2,"Cards":[6,7]}""",
        """{"$type":"CardsLookedAtEvent","Player":1,"Cards":[6,7]}""",
        """{"$type":"EventsRetractedEvent","Sequences":[12,13]}""",
        """{"$type":"LibraryArrangedEvent","Player":1,"OnTop":2,"OnBottom":1,"Shuffled":false}""",
        """{"$type":"AbilityActivatedEvent","Ability":8,"Source":3,"Controller":1,"Text":"{T}: Add {G}.","Targets":[],"Triggered":false,"Modes":[]}""",
        """{"$type":"TokenCreatedEvent","Object":9,"Controller":1,"Name":"Soldier"}""",
        """{"$type":"CountersPutEvent","Object":5,"Name":"+1/+1","Count":1}""",
        """{"$type":"ExtraTurnAddedEvent","Player":1,"Count":1}""",
        """{"$type":"PlayerLostEvent","Player":2,"Reason":"Life"}""",
        """{"$type":"GameEndedEvent","Outcome":"TeamWon","WinningTeam":1}""",
        """{"$type":"TransformedEvent","Object":5,"From":"Delver of Secrets","To":"Insectile Aberration"}""",
        """{"$type":"DieRolledEvent","Player":1,"Sides":20,"Result":17}""",
        """{"$type":"ExtraPhaseAddedEvent","Phase":"Combat"}""",
        """{"$type":"DayNightBecameEvent","Designation":"Night"}""",
        """{"$type":"MonarchBecameEvent","Player":2}""",
        """{"$type":"RandomPlayerChosenEvent","Controller":1,"SelectedPlayer":2,"Source":4}""",
        """{"$type":"StartingPlayerChosenEvent","Chooser":2,"Player":1}""",
        """{"$type":"MulliganTakenEvent","Player":1,"Count":1}""",
        """{"$type":"HandKeptEvent","Player":1,"Cards":6}""",
        """{"$type":"CoinFlippedEvent","Player":2,"Won":false,"Called":true,"Heads":false}""",
        """{"$type":"TurnEndedEvent","Player":1}""",
        """{"$type":"ScriedEvent","Player":1,"Count":2}""",
        """{"$type":"SurveilledEvent","Player":2,"Count":1}""",
        """{"$type":"AbilityResolvedEvent","Ability":8,"Source":3,"Controller":1,"First":40,"Targets":[{"Player":0,"Object":5}],"PowerToughness":[{"Object":5,"Power":2,"Toughness":2}]}""",
    ];

    [Theory]
    [MemberData(nameof(Prompts))]
    public void A_prompt_reads_and_writes_as_the_doc_shapes_it(string json) => RoundTrip<PromptPayload>(json);

    [Theory]
    [MemberData(nameof(Answers))]
    public void An_answer_reads_and_writes_as_the_doc_shapes_it(string json) => RoundTrip<AnswerPayload>(json);

    [Theory]
    [MemberData(nameof(Events))]
    public void An_event_reads_and_writes_as_the_doc_shapes_it(string json) => RoundTrip<GameEventView>(json);

    [Fact]
    public void Every_union_member_has_a_sample()
    {
        // A prompt added to the wire without a sample here is one the sidecar's JSON was never checked against.
        AssertSampled<PromptPayload>(Prompts);
        AssertSampled<AnswerPayload>(Answers);
        AssertSampled<GameEventView>(Events);
    }

    [Fact]
    public void A_view_reads_with_the_rust_mirrors_nulls_and_writes_back_whole()
    {
        // The Rust mirror sends null for an option it has none of: a step before the first turn, a zone a printed
        // card is not in. Each reads as the enum's zero.
        const string json = """
            {"Viewer":1,"Turn":0,"Step":null,"ActivePlayer":0,"PriorityPlayer":0,
             "Players":[{"Seat":1,"Name":"Alice","Team":1,"Life":20,"Status":"Active","PoisonCounters":0,"HandCount":7,"LibraryCount":53,"LandsPlayedThisTurn":0,"Mana":[0,0,0,1,0,0],"RestrictedMana":[{"Type":null,"Amount":1,"Filter":"CreatureSpells","Effect":"None"}],"IsMonarch":false,"HasInitiative":false,"Dungeon":"","DungeonRoom":"","DungeonsCompleted":0,"Energy":0,"HasCitysBlessing":false,"Counters":[],"HasEnduringStory":false,"Speed":null}],
             "Objects":[{"Handle":1,"Zone":null,"ZoneOwner":0,"Kind":"Card","Owner":0,"Controller":0,"Known":true,"CardId":"Lightning Bolt","Name":"Lightning Bolt","ManaCost":"{R}","ManaValue":1,"OracleText":"Lightning Bolt deals 3 damage to any target.","Supertypes":[],"CardTypes":["Instant"],"Subtypes":[],"Colors":["Red"],"Power":null,"Toughness":null,"Tapped":false,"FaceDown":false,"DamageMarked":0,"Counters":[],"SummoningSick":false,"KnownBy":[],"Targets":[],"ExiledWith":0,"ProducesMana":false,"FaceDownName":"","Monstrous":false,"AttachedTo":0,"Choices":[],"Exerted":false,"Saddled":false,"Goaded":false,"OtherFace":null,"Harnessed":false,"AttachedToPlayer":0,"GrantedAbilities":[],"SpentLines":[],"Fingerprint":0,"IsCopy":false,"CopyingCard":null,"PrepareSpell":null,"ChosenModes":[],"ClassLevel":0,"Protector":0,"Halves":[],"StartingCounter":null}],
             "Zones":[{"Kind":"Library","Owner":1,"Count":53,"Contents":[],"ContentsVisible":false,"Known":[{"FromTop":0,"Card":{"Handle":0,"Known":true,"CardId":"Island","Name":"Island"}}]}],
             "Combat":{"Attackers":[{"Attacker":3,"TargetKind":"Player","TargetPlayer":2,"TargetPermanent":0,"WasBlocked":false}],"Blocks":[{"Attacker":3,"Blockers":[5]}]},
             "Outcome":"InProgress","WinningTeam":0,"DayNight":"Neither"}
            """;

        PlayerView view = ProtocolJson.Deserialize<PlayerView>(json)!;
        Assert.Equal(TurnStepView.Untap, view.Step);
        Assert.Equal(ZoneKindView.Library, view.Objects[0].Zone);
        Assert.Equal(new ViewHandle(1), view.Objects[0].Handle);
        Assert.Null(view.Objects[0].Power);
        Assert.Equal([0, 0, 0, 1, 0, 0], view.Players[0].Mana);
        Assert.Equal(ManaSpendFilterView.CreatureSpells, view.Players[0].RestrictedMana[0].Filter);
        Assert.Equal("Island", view.Zones[0].Known[0].Card.Name);
        Assert.Equal(new ViewHandle(5), view.Combat.Blocks[0].Blockers[0]);

        JsonObject written = JsonNode.Parse(ProtocolJson.Serialize(view))!.AsObject();
        Assert.Equal("Untap", written["Step"]!.GetValue<string>());
        JsonObject bolt = written["Objects"]![0]!.AsObject();
        Assert.Equal(1, bolt["Handle"]!.GetValue<int>());
        Assert.True(bolt.ContainsKey("Power"));
        Assert.Null(bolt["Power"]);
    }

    [Fact]
    public void Seats_key_the_dictionaries_the_host_sends()
    {
        Dictionary<int, PlayerView> views = ProtocolJson.Deserialize<Dictionary<int, PlayerView>>(
            """{"1":{"Viewer":1},"2":{"Viewer":2}}""")!;

        Assert.Equal([1, 2], views.Keys.Order());
        Assert.Equal(2, views[2].Viewer);
    }

    [Fact]
    public void The_tag_comes_first_and_computed_members_stay_off_the_wire()
    {
        string json = ProtocolJson.Serialize<PromptPayload>(new ChooseModesPromptMessage { Player = 1, Modes = ["a"], Eligible = [0], Count = 1 });

        Assert.StartsWith("""{"$type":"ChooseModesPromptMessage","Player":1""", json, StringComparison.Ordinal);
        Assert.DoesNotContain("\"Preferred\"", json, StringComparison.Ordinal);
        Assert.DoesNotContain("\"Most\"", json, StringComparison.Ordinal);

        string target = ProtocolJson.Serialize(new TargetMessage { Player = 2 });
        Assert.Equal("""{"Player":2,"Object":0}""", target);
    }

    [Fact]
    public void A_handle_is_a_bare_integer_either_way_and_null_where_it_may_be()
    {
        var answer = (TakeActionAnswer)ProtocolJson.Deserialize<AnswerPayload>(
            """{"$type":"TakeActionAnswer","Kind":"PlayLand","Source":3,"Sacrifice":null,"Tap":5}""")!;

        Assert.Equal(new ViewHandle(3), answer.Source);
        Assert.Null(answer.Sacrifice);
        Assert.Equal(new ViewHandle(5), answer.Tap);

        Assert.Contains("\"Source\":3", ProtocolJson.Serialize<AnswerPayload>(answer), StringComparison.Ordinal);
        Assert.Contains("\"Sacrifice\":null", ProtocolJson.Serialize<AnswerPayload>(answer), StringComparison.Ordinal);
    }

    [Fact]
    public void An_enum_reads_by_name_or_number_and_writes_by_name()
    {
        Assert.Equal(ErrorCode.UnknownHandle, ProtocolJson.Deserialize<ErrorCode>("\"UnknownHandle\""));
        Assert.Equal(ErrorCode.UnknownHandle, ProtocolJson.Deserialize<ErrorCode>("2"));
        Assert.Equal("\"StaleReply\"", ProtocolJson.Serialize(ErrorCode.StaleReply));
        Assert.Throws<JsonException>(() => ProtocolJson.Deserialize<ErrorCode>("\"NoSuchCode\""));
    }

    [Fact]
    public void A_kind_nobody_declared_is_refused_by_name()
    {
        JsonException error = Assert.Throws<JsonException>(() => ProtocolJson.Deserialize<PromptPayload>("""{"$type":"WishPromptMessage","Player":1}"""));
        Assert.Contains("WishPromptMessage", error.Message, StringComparison.Ordinal);

        Assert.Throws<JsonException>(() => ProtocolJson.Deserialize<AnswerPayload>("""{"Player":1}"""));
    }

    /// <summary>
    /// Reads the sample into the base type, checks it came out as the kind the tag names, writes it back and checks
    /// that every member the sample spelled is there with the same value — the writer adds the members the sample
    /// left to default, which is fine — and that what was written reads and writes again to the same bytes.
    /// </summary>
    private static void RoundTrip<TBase>(string json)
        where TBase : class
    {
        TBase read = ProtocolJson.Deserialize<TBase>(json) ?? throw new Xunit.Sdk.XunitException("Read nothing.");
        JsonObject sample = JsonNode.Parse(json)!.AsObject();
        Assert.Equal(sample["$type"]!.GetValue<string>(), read.GetType().Name);

        string written = ProtocolJson.Serialize(read);
        JsonObject output = JsonNode.Parse(written)!.AsObject();
        Assert.Equal("$type", output.First().Key);

        AssertContains(sample, output, read.GetType().Name);

        TBase again = ProtocolJson.Deserialize<TBase>(written)!;
        Assert.Equal(written, ProtocolJson.Serialize(again));
    }

    /// <summary>
    /// Every member the sample spelled is in what was written, with the same value, at every depth: an object may have
    /// gained the members the sample left to default, an array may not have changed length.
    /// </summary>
    private static void AssertContains(JsonNode? expected, JsonNode? actual, string where)
    {
        switch (expected)
        {
            case JsonObject members:
                JsonObject? written = actual as JsonObject;
                Assert.True(written is not null, $"{where}: sample {members.ToJsonString()}, written {actual?.ToJsonString() ?? "null"}.");
                foreach ((string name, JsonNode? value) in members)
                {
                    Assert.True(written.ContainsKey(name), $"{where}.{name} was not written.");
                    AssertContains(value, written[name], $"{where}.{name}");
                }

                break;

            case JsonArray items:
                JsonArray? array = actual as JsonArray;
                Assert.True(array is not null && array.Count == items.Count, $"{where}: sample {items.ToJsonString()}, written {actual?.ToJsonString() ?? "null"}.");
                for (int index = 0; index < items.Count; index++)
                {
                    AssertContains(items[index], array[index], $"{where}[{index}]");
                }

                break;

            default:
                Assert.True(
                    JsonNode.DeepEquals(expected, actual),
                    $"{where}: sample {expected?.ToJsonString() ?? "null"}, written {actual?.ToJsonString() ?? "null"}.");
                break;
        }
    }

    private static void AssertSampled<TBase>(TheoryData<string> samples)
    {
        HashSet<string> sampled = [.. ((IEnumerable<object[]>)samples).Select(row => JsonNode.Parse((string)row[0])!["$type"]!.GetValue<string>())];
        foreach (UnionAttribute union in typeof(TBase).GetCustomAttributes<UnionAttribute>())
        {
            Assert.True(sampled.Contains(union.SubType.Name), $"{union.SubType.Name} has no JSON sample in {nameof(ProtocolJsonTests)}.");
        }
    }

    [Fact]
    public void A_declaration_of_blockers_carries_its_requirements_and_the_hosts_verdict()
    {
        // A creature that blocks this turn if able (CR 509.1c) is a requirement with no attacker; the verdict on declaring no
        // blockers says why that may not be declared, and which blocks may be declared first.
        var prompt = ProtocolJson.Deserialize<PromptPayload>(
            """{"$type":"DeclareBlockersPromptMessage","Player":2,"Eligible":[9],"Attackers":[3],"Allowed":[{"Blocker":9,"Attacker":3}],"MinimumBlockers":[1],"Requirements":[{"Blocker":9,"Attacker":0}],"MostRequirements":1,"Suggested":[{"Blocker":9,"Attacker":3}],"Verdict":{"Complete":false,"Addable":[{"Blocker":9,"Attacker":3}],"Issue":{"Creature":9,"Words":"must block","Other":0,"Rule":"509.1c"}}}""") as DeclareBlockersPromptMessage;
        Assert.NotNull(prompt);
        Assert.True(prompt.Requirements[0].Attacker.IsNone);
        Assert.False(prompt.Verdict.Complete);
        Assert.Equal(new ViewHandle(3), Assert.Single(prompt.Verdict.Addable).Attacker);
        Assert.Equal(("must block", "509.1c"), (prompt.Verdict.Issue!.Words, prompt.Verdict.Issue.Rule));
        Assert.True(prompt.Verdict.Issue.Other.IsNone);
    }
}
