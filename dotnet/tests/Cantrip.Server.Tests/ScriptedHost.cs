using System.Text.Json;
using Cantrip.Server.Rules;

namespace Cantrip.Server.Tests;

/// <summary>
/// A host that answers from a script: a two-seat game whose opening asks seat one for priority (request 7), whose
/// answer asks seat two (request 8), and which refuses a card choice as illegal.
/// </summary>
internal sealed class ScriptedHost : IHostTransport
{
    public List<string> Requests { get; } = [];

    public (string Code, string Detail)? RefuseDeal { get; init; }

    /// <summary>Whether the events carry the host's own sequence numbers.</summary>
    public bool Numbered { get; init; }

    /// <summary>
    /// Whether each priority step carries a public part (docs/host-protocol.md, "The public view"): Alice's Island on the
    /// battlefield under public handle 5, which seat two names 1 and seat one does not see.
    /// </summary>
    public bool Public { get; init; }

    /// <summary>The step the game opens with: a priority prompt for seat one unless a test wants another question.</summary>
    public string? Opening { get; init; }

    /// <summary>Codes to answer the next answer with instead of a step, by code.</summary>
    public Dictionary<string, bool> Rejections { get; } = [];

    public string Exchange(string request)
    {
        Requests.Add(request);
        using JsonDocument document = JsonDocument.Parse(request);
        JsonElement root = document.RootElement;
        string command = root.GetProperty("Cmd").GetString()!;

        switch (command)
        {
            case "NewGame":
                if (RefuseDeal is { } refuse)
                {
                    return $$"""{"Rejected":{"Code":"{{refuse.Code}}","Detail":"{{refuse.Detail}}"} }""";
                }

                return Ok($$"""{"GameId":1,"Decklists":{"1":[{{Printed("Island")}},{{Printed("Island")}},{{Printed("Lightning Bolt")}}],"2":[{{Printed("Swamp")}}] } }""");

            case "Start":
                return Ok(Opening ?? PriorityStep(7, 1, finished: false));

            case "Answer":
                if (Rejections.Remove("StaleReply"))
                {
                    return """{"Rejected":{"Code":"StaleReply","Detail":"request 7 is not outstanding."}}""";
                }

                string kind = root.GetProperty("Answer").GetProperty("$type").GetString()!;
                return kind == "PassAnswer"
                    ? Ok(PriorityStep(8, 2, finished: false))
                    : """{"Rejected":{"Code":"IllegalAnswer","Detail":"a priority prompt takes a pass or an action."}}""";

            case "Concede":
                return Ok(PriorityStep(0, 0, finished: true));

            case "View":
                return Ok($$"""{"View":{{View(root.GetProperty("Seat").GetInt32())}}}""");

            case "Oracle":
                string name = root.GetProperty("Name").GetString()!;
                return Ok(name == "Wish" ? """{"Card":null}""" : $$"""{"Card":{{Printed(name)}}}""");

            case "Close":
                return Ok("{}");

            default:
                throw new InvalidOperationException($"Unexpected command {command}.");
        }
    }

    public static string DiscardStep =>
        $$"""
        {"Requests":[{"RequestId":7,"Seat":1,"Prompt":{"$type":"ChooseCardsPromptMessage","Player":1,"Owner":1,"Source":0,"Purpose":"Discard","Cards":[1],"Eligible":[1],"Count":1,"UpTo":false},"MayCancel":false}],
         "Views":{"1":{{View(1)}},"2":{{View(2)}}},
         "Events":{"1":[],"2":[]},
         "Finished":false,"Outcome":"InProgress","WinningTeam":0}
        """;

    private static string Ok(string body) => $$"""{"Ok":{{body}}}""";

    private string PriorityStep(int requestId, int seat, bool finished)
    {
        string requests = requestId == 0
            ? "[]"
            : $$"""[{"RequestId":{{requestId}},"Seat":{{seat}},"Prompt":{"$type":"PriorityPromptMessage","Player":{{seat}},"Options":[]},"MayCancel":false}]""";
        string sequence = Numbered ? "\"Sequence\":40," : string.Empty;
        string outcome = finished ? "\"Finished\":true,\"Outcome\":\"TeamWon\",\"WinningTeam\":1" : "\"Finished\":false,\"Outcome\":\"InProgress\",\"WinningTeam\":0";
        string shown = Public
            ? $$"""
                "Public":{"View":{{View(0)}},
                 "Events":[{"$type":"StepBeganEvent",{{sequence}}"Turn":1,"Step":"Upkeep","ActivePlayer":1}],
                 "Handles":{"1":[],"2":[[5,1]]} },
                """
            : string.Empty;

        return $$"""
            {"Requests":{{requests}},
             "Views":{"1":{{View(1)}},"2":{{View(2)}}},
             "Events":{"1":[{"$type":"StepBeganEvent",{{sequence}}"Turn":1,"Step":"Upkeep","ActivePlayer":1}],"2":[{"$type":"StepBeganEvent",{{sequence}}"Turn":1,"Step":"Upkeep","ActivePlayer":1}]},
             {{shown}}
             {{outcome}}}
            """;
    }

    /// <summary>
    /// Seat one sees its Lightning Bolt in hand; seat two sees an Island of Alice's on the battlefield, and so does the public
    /// (seat zero) under a handle of its own.
    /// </summary>
    private static string View(int seat)
    {
        string objects = seat switch
        {
            1 => """[{"Handle":1,"Zone":"Hand","ZoneOwner":1,"Kind":"Card","Owner":1,"Controller":1,"Known":true,"CardId":"Lightning Bolt","Name":"Lightning Bolt"}]""",
            2 => """[{"Handle":1,"Zone":"Battlefield","ZoneOwner":0,"Kind":"Card","Owner":1,"Controller":1,"Known":true,"CardId":"Island","Name":"Island"}]""",
            _ => """[{"Handle":5,"Zone":"Battlefield","ZoneOwner":0,"Kind":"Card","Owner":1,"Controller":1,"Known":true,"CardId":"Island","Name":"Island"}]""",
        };

        return $$"""
            {"Viewer":{{seat}},"Turn":1,"Step":"Upkeep","ActivePlayer":1,"PriorityPlayer":1,
             "Players":[{"Seat":1,"Name":"Alice","Team":1,"Life":20,"Status":"Active"},{"Seat":2,"Name":"Bob","Team":2,"Life":20,"Status":"Active"}],
             "Objects":{{objects}},"Zones":[],"Combat":{"Attackers":[],"Blocks":[]},"Outcome":"InProgress","WinningTeam":0,"DayNight":"Neither"}
            """;
    }

    private static string Printed(string name) =>
        $$"""{"Handle":0,"Known":true,"CardId":"{{name}}","Name":"{{name}}","CardTypes":["{{(name == "Lightning Bolt" ? "Instant" : "Land")}}"]}""";
}
