using System.Globalization;
using System.Runtime.InteropServices.JavaScript;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Cantrip.Protocol;
using MessagePack;
using Windows.Foundation;

namespace Cantrip.Client.Web;

/// <summary>
/// The advisor's panel (<c>?advisor=API</c>), for a page at the advisor's own table (dotnet/tools/Cantrip.Advisor) and
/// no other: the person plays the AI's seat through the table as ever, and beside it the panel shows what the AI would
/// do at each of their decisions, as the advisor's API says: its choice, its top candidates with the network's prior,
/// the search's visit share and the win chance it sees in each, how much its sampled worlds disagreed, and the belief's
/// guesses at the opponent's hand beside the prior.
/// <para>
/// The play made is whatever this seat's client sends. One click sends the AI's choice (or any candidate) as this
/// seat's answer; any other legal play is made through the table as ever. A decision the AI treats as needing no
/// thought - one legal answer, a payment, the priority shortcut - plays itself unless that is turned off, and the
/// page's own automatic passing of priority is the advisor's to do (<see cref="AdvisorHoldsPriority"/>): Pass Turn and
/// Resolve All, which the person asks for by name, still pass. "Hide the suggestion until I've chosen" keeps the advice
/// out of sight until the play is made. A one-line reason, and a flag, are kept with the decision's record.
/// </para>
/// <para>
/// The true hidden cards come from the API only once the game is over, and are shown only when asked for then.
/// </para>
/// </summary>
public sealed partial class MainPage
{
    private static readonly HttpClient AdvisorHttp = new() { Timeout = TimeSpan.FromSeconds(60) };

    private Border _advisorPanel = null!;
    private StackPanel _advisorBody = null!;
    private TextBlock _advisorTitle = null!;
    private TextBlock _advisorStatus = null!;
    private TextBlock _advisorChoice = null!;
    private StackPanel _advisorCandidates = null!;
    private TextBlock _advisorWorlds = null!;
    private TextBlock _advisorBelief = null!;
    private TextBlock _advisorValues = null!;
    private TextBlock _advisorHistory = null!;
    private TextBlock _advisorTruth = null!;
    private TextBox _advisorReason = null!;
    private DockButton _advisorAccept = null!;
    private DockButton _advisorFlag = null!;
    private DockButton _advisorCollapse = null!;
    private DockToggle _advisorHide = null!;
    private DockToggle _advisorAuto = null!;
    private DockToggle _advisorReveal = null!;
    private readonly List<(string Name, FrameworkElement Element)> _advisorControls = [];

    /// <summary>The API's last state, and the request the advice shown is for.</summary>
    private JsonNode? _advisorState;
    private int _advisorVersion = -1;
    private int _advisorPlayedFor;
    private int _advisorReasonFor;
    private bool _advisorCollapsed;
    private string _advisorError = string.Empty;

    /// <summary>
    /// Whether the page leaves this priority to the advisor rather than passing it by itself: at the advisor's table,
    /// unless Pass Turn or Resolve All asked for passing by name.
    /// </summary>
    private bool AdvisorHoldsPriority() => ClientConfig.Advisor is not null && !_passTurn && _resolveAll is null;

    /// <summary>Whether a point of the page is on the advisor's panel; never, at any other table.</summary>
    private bool AdvisorCovers(Point at) =>
        ClientConfig.Advisor is not null && _advisorPanel is { Visibility: Visibility.Visible } panel && panel.ActualWidth > 0
        && panel.TransformToVisual(Root).TransformBounds(new Rect(0, 0, panel.ActualWidth, panel.ActualHeight)).Contains(at);

    private void BuildAdvisor()
    {
        _advisorTitle = AdvisorText(13, Theme.Fg, bold: true);
        _advisorStatus = AdvisorText(12, Theme.FgSoft);
        _advisorChoice = AdvisorText(14, Theme.Accent, bold: true);
        _advisorCandidates = new StackPanel { Spacing = 3 };
        _advisorWorlds = AdvisorText(11.5, Theme.FgSoft);
        _advisorBelief = AdvisorText(11.5, Theme.FgSoft);
        _advisorValues = AdvisorText(11.5, Theme.FgSoft);
        _advisorHistory = AdvisorText(11, Theme.FgMuted);
        _advisorTruth = AdvisorText(11.5, Theme.Theirs);

        _advisorAccept = new DockButton("Play the bot's choice", primary: true) { Tooltip = "Send the AI's answer as yours" };
        _advisorAccept.Clicked += () => PlayAdvice(-1);
        _advisorFlag = new DockButton("Flag", primary: false) { Tooltip = "Mark this decision for review" };
        _advisorFlag.Clicked += FlagAdvice;
        _advisorCollapse = new DockButton("-", primary: false) { Tooltip = "Fold the advisor away" };
        _advisorCollapse.Clicked += () =>
        {
            _advisorCollapsed = !_advisorCollapsed;
            _advisorBody.Visibility = _advisorCollapsed ? Visibility.Collapsed : Visibility.Visible;
            _advisorCollapse.Caption = _advisorCollapsed ? "+" : "-";
        };

        _advisorReason = new TextBox
        {
            PlaceholderText = "Why (one line, optional)",
            FontSize = 12,
            MaxLength = 300,
            RequestedTheme = ElementTheme.Dark,
            AcceptsReturn = false,
        };
        _advisorReason.KeyDown += (_, e) =>
        {
            if (e.Key == Windows.System.VirtualKey.Enter)
            {
                SendReason();
                e.Handled = true;
            }
        };
        _advisorReason.LostFocus += (_, _) => SendReason();

        _advisorHide = new DockToggle("Hide the suggestion until I've chosen");
        _advisorHide.Toggled += on => _ = PostAdvisorAsync("settings", new JsonObject { ["Hidden"] = on });
        _advisorAuto = new DockToggle("Play forced and trivial decisions by themselves");
        _advisorAuto.SetOn(true);
        _advisorAuto.Toggled += on => _ = PostAdvisorAsync("settings", new JsonObject { ["AutoTrivial"] = on });
        _advisorReveal = new DockToggle("Reveal the true hidden cards") { Visibility = Visibility.Collapsed };
        _advisorReveal.Toggled += _ => RenderAdvisor();

        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6, Children = { _advisorAccept, _advisorFlag } };
        _advisorBody = new StackPanel
        {
            Spacing = 6,
            Children =
            {
                _advisorStatus,
                _advisorChoice,
                buttons,
                _advisorCandidates,
                _advisorWorlds,
                _advisorBelief,
                _advisorReason,
                _advisorHide,
                _advisorAuto,
                _advisorValues,
                _advisorReveal,
                _advisorTruth,
                _advisorHistory,
            },
        };
        var header = new Grid { ColumnDefinitions = { new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) }, new ColumnDefinition { Width = GridLength.Auto } } };
        header.Children.Add(_advisorTitle);
        Grid.SetColumn(_advisorCollapse, 1);
        header.Children.Add(_advisorCollapse);

        _advisorPanel = new Border
        {
            Width = 360,
            MaxHeight = 820,
            Margin = new Thickness(0, 8, 8, 8),
            Padding = new Thickness(10),
            CornerRadius = new CornerRadius(8),
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Top,
            Background = Theme.Brush(Theme.WithAlpha(Theme.Panel, 0.94)),
            BorderBrush = Theme.Brush(Theme.Line),
            BorderThickness = new Thickness(1),
            Child = new ScrollViewer
            {
                VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                Content = new StackPanel { Spacing = 6, Children = { header, _advisorBody } },
            },
        };
        Root.Children.Add(_advisorPanel);

        _advisorControls.AddRange([("accept", _advisorAccept), ("flag", _advisorFlag), ("reason", _advisorReason), ("hide", _advisorHide),
            ("auto", _advisorAuto), ("reveal", _advisorReveal), ("panel", _advisorPanel)]);
        _advisorTitle.Text = "Advisor";
        _advisorStatus.Text = "Waiting for the table...";
        RenderAdvisor();

        // What the page does by itself: a trivial decision played, and the panel kept in step with the prompt.
        var tick = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(100) };
        tick.Tick += (_, _) =>
        {
            AutoPlayAdvice();
            if (ClientConfig.Test)
            {
                JSHost.GlobalThis.SetProperty("cantripAdvisor", AdvisorTestState());
            }
        };
        tick.Start();
        _ = FollowAdvisorAsync();
    }

    private static TextBlock AdvisorText(double size, Windows.UI.Color color, bool bold = false) => new()
    {
        FontSize = size,
        Foreground = Theme.Brush(color),
        TextWrapping = TextWrapping.Wrap,
        FontWeight = bold ? Microsoft.UI.Text.FontWeights.SemiBold : Microsoft.UI.Text.FontWeights.Normal,
    };

    /// <summary>Follows the API's state for as long as the page is open (long-polled: it answers when anything changes).</summary>
    private async Task FollowAdvisorAsync()
    {
        while (true)
        {
            try
            {
                string text = await AdvisorHttp.GetStringAsync(new Uri($"{ClientConfig.Advisor}/state?since={_advisorVersion}"));
                JsonNode? state = JsonNode.Parse(text);
                _advisorError = string.Empty;
                if (state?["Version"]?.GetValue<int>() is int version)
                {
                    _advisorVersion = version;
                    _advisorState = state;
                    SyncAdvisorSettings(state);
                    RenderAdvisor();
                    AutoPlayAdvice();
                }
            }
            catch (Exception error) when (error is HttpRequestException or TaskCanceledException or JsonException)
            {
                _advisorError = $"The advisor is not answering ({error.Message}).";
                RenderAdvisor();
                await Task.Delay(1000);
            }
        }
    }

    private void SyncAdvisorSettings(JsonNode state)
    {
        if (state["Settings"]?["Hidden"]?.GetValue<bool>() is bool hidden && hidden != _advisorHide.On)
        {
            _advisorHide.SetOn(hidden);
        }

        if (state["Settings"]?["AutoTrivial"]?.GetValue<bool>() is bool auto && auto != _advisorAuto.On)
        {
            _advisorAuto.SetOn(auto);
        }
    }

    private static async Task PostAdvisorAsync(string what, JsonObject body)
    {
        try
        {
            using var content = new StringContent(body.ToJsonString(), Encoding.UTF8, "application/json");
            using HttpResponseMessage _ = await AdvisorHttp.PostAsync(new Uri($"{ClientConfig.Advisor}/{what}"), content);
        }
        catch (Exception error) when (error is HttpRequestException or TaskCanceledException)
        {
            // The next state says whether it took.
        }
    }

    /// <summary>The decision the advice is for, when it is the one this seat is being asked now.</summary>
    private JsonNode? CurrentAdvice()
    {
        JsonNode? current = _advisorState?["Current"];
        return current is not null && _gameClient?.Prompt is { } asked && current["RequestId"]?.GetValue<int>() == asked.RequestId ? current : null;
    }

    private bool AdviceHidden(JsonNode current) => _advisorHide.On && current["Answered"]?.GetValue<bool>() != true;

    /// <summary>Plays the AI's choice (<paramref name="candidate"/> -1) or one of its candidates, as this seat's answer.</summary>
    private void PlayAdvice(int candidate)
    {
        if (_gameClient is not { } client || _advisorState?["Game"]?["Divergence"] is JsonValue || CurrentAdvice() is not { } current
            || current["Status"]?.GetValue<string>() != "ready")
        {
            return;
        }

        string? wire = candidate < 0
            ? current["Chosen"]?["Payload"]?.GetValue<string>()
            : current["Candidates"]?[candidate]?["Payload"]?.GetValue<string>();
        if (wire is null)
        {
            return;
        }

        int requestId = current["RequestId"]!.GetValue<int>();
        SendReason();
        _ = PostAdvisorAsync("note", new JsonObject { ["RequestId"] = requestId, ["Via"] = candidate < 0 ? "accept" : $"candidate {candidate + 1}" });
        _advisorPlayedFor = requestId;
        Send(client, MessagePackSerializer.Deserialize<AnswerPayload>(Convert.FromBase64String(wire)));
        RedrawNow();
    }

    /// <summary>Plays a decision the AI treats as needing no thought, once, when the person has not turned that off.</summary>
    private void AutoPlayAdvice()
    {
        if (!_advisorAuto.On || _advisorState?["Game"]?["Divergence"] is JsonValue || CurrentAdvice() is not { } current || current["Trivial"]?.GetValue<bool>() != true
            || current["Answered"]?.GetValue<bool>() == true || _advisorPlayedFor == current["RequestId"]?.GetValue<int>()
            || current["Chosen"]?["Payload"]?.GetValue<string>() is not { } wire || _gameClient is not { } client)
        {
            return;
        }

        int requestId = current["RequestId"]!.GetValue<int>();
        _advisorPlayedFor = requestId;
        _ = PostAdvisorAsync("note", new JsonObject { ["RequestId"] = requestId, ["Via"] = "auto" });
        Send(client, MessagePackSerializer.Deserialize<AnswerPayload>(Convert.FromBase64String(wire)));
        RedrawNow();
    }

    private void FlagAdvice()
    {
        if (_advisorState?["Current"] is not { } current || current["RequestId"]?.GetValue<int>() is not int requestId)
        {
            return;
        }

        bool flagged = current["Flagged"]?.GetValue<bool>() != true;
        _ = PostAdvisorAsync("note", new JsonObject { ["RequestId"] = requestId, ["Flagged"] = flagged });
    }

    /// <summary>The reason typed, kept with the decision it was typed at (the one shown when typing began).</summary>
    private void SendReason()
    {
        int requestId = _advisorReasonFor != 0 ? _advisorReasonFor : _advisorState?["Current"]?["RequestId"]?.GetValue<int>() ?? 0;
        if (requestId == 0 || _advisorReason.Text.Length == 0)
        {
            return;
        }

        _ = PostAdvisorAsync("note", new JsonObject { ["RequestId"] = requestId, ["Reason"] = _advisorReason.Text });
    }

    private void RenderAdvisor()
    {
        JsonNode? state = _advisorState;
        JsonNode? game = state?["Game"];
        if (game is not null)
        {
            _advisorTitle.Text = $"Advisor: {game["Model"]}, {Seconds(game["BudgetMs"])} a decision, against {game["Opponent"]}";
        }

        JsonNode? current = state?["Current"];
        int? requestId = current?["RequestId"]?.GetValue<int>();

        // A new decision clears the reason box, after keeping what was typed for the last.
        if (requestId is int id && id != _advisorReasonFor)
        {
            if (_advisorReasonFor != 0)
            {
                SendReason();
            }

            _advisorReasonFor = id;
            _advisorReason.Text = current?["Reason"]?.GetValue<string>() ?? string.Empty;
        }

        _advisorCandidates.Children.Clear();
        _advisorAccept.Visibility = Visibility.Collapsed;
        _advisorChoice.Text = string.Empty;
        _advisorWorlds.Text = string.Empty;
        _advisorBelief.Text = string.Empty;
        _advisorFlag.On = current?["Flagged"]?.GetValue<bool>() == true;

        bool over = game?["Over"]?.GetValue<bool>() == true;
        if (_advisorError.Length > 0)
        {
            _advisorStatus.Text = _advisorError;
        }
        else if (game?["Divergence"]?.GetValue<string>() is { } diverged)
        {
            // The AI's copy of the game is not this table's: its advice is about another game, so none is offered.
            _advisorStatus.Text = $"The advisor has lost track of the game ({diverged}); its advice is withheld.";
            current = null;
        }
        else if (over)
        {
            _advisorStatus.Text = $"The game is over: {game?["Result"]}. Recorded in {game?["Session"]}.";
        }
        else if (current is null)
        {
            _advisorStatus.Text = "No decision of yours yet.";
        }
        else
        {
            string kind = current["Kind"]?.GetValue<string>() ?? "decision";
            string status = current["Status"]?.GetValue<string>() ?? string.Empty;
            bool answered = current["Answered"]?.GetValue<bool>() == true;
            _advisorStatus.Text = status switch
            {
                "thinking" => $"Decision {current["Index"]}: the bot is thinking...",
                "error" => $"Decision {current["Index"]}: the bot could not answer ({current["Error"]}).",
                _ when answered => $"Decision {current["Index"]} (turn {current["Turn"]}, {kind}): you played {current["You"] ?? "your answer"}.",
                _ => $"Decision {current["Index"]} (turn {current["Turn"]}, {kind}){(current["Trivial"]?.GetValue<bool>() == true ? $": trivial ({current["TrivialWhy"]})" : string.Empty)}.",
            };

            if (status == "ready" && AdviceHidden(current))
            {
                _advisorChoice.Text = "The suggestion is hidden until you choose.";
            }
            else if (status == "ready")
            {
                RenderAdvice(current, answered);
            }
        }

        // The bot's win chance over the game, as it saw it at each of your decisions.
        if (state?["Values"] is JsonArray values && values.Count > 0)
        {
            IEnumerable<string> last = values.TakeLast(12).Select(v => $"{v?[2]}");
            _advisorValues.Text = $"The bot's win chance at your decisions: {string.Join(" > ", last)}";
        }

        // The true hidden cards, only after the game, and only when asked.
        _advisorReveal.Visibility = over ? Visibility.Visible : Visibility.Collapsed;
        _advisorTruth.Text = over && _advisorReveal.On ? Truth(state) : string.Empty;
        _advisorTruth.Visibility = _advisorTruth.Text.Length > 0 ? Visibility.Visible : Visibility.Collapsed;

        if (state?["History"] is JsonArray history)
        {
            var lines = new StringBuilder("Your decisions, latest first:\n");
            foreach (JsonNode? h in history.Take(12))
            {
                if (h is null || h["Trivial"]?.GetValue<bool>() == true)
                {
                    continue;
                }

                string mark = h["Agreed"]?.GetValue<bool>() switch { true => "same", false => "DIFFERENT", null => "..." };
                lines.Append(CultureInfo.InvariantCulture, $"{h["Index"]}. T{h["Turn"]} {h["Kind"]}: you {h["You"] ?? "-"}; bot {h["Bot"] ?? "-"} ({mark})");
                if (over && _advisorReveal.On && h["TheirHand"] is JsonArray hand)
                {
                    lines.Append(CultureInfo.InvariantCulture, $" | their hand: {string.Join(", ", hand.Select(c => c?.ToString()))}");
                }

                lines.Append('\n');
            }

            _advisorHistory.Text = lines.ToString().TrimEnd();
        }
    }

    private void RenderAdvice(JsonNode current, bool answered)
    {
        string? chosen = current["Chosen"]?["Label"]?.GetValue<string>();
        _advisorChoice.Text = $"Bot: {chosen ?? "?"}{(current["WinPct"] is JsonNode win ? $"  (its search's root: {win}% to win)" : string.Empty)}";
        _advisorAccept.Visibility = answered ? Visibility.Collapsed : Visibility.Visible;

        if (current["Candidates"] is JsonArray candidates)
        {
            _advisorCandidates.Children.Add(AdvisorText(11, Theme.FgMuted).With("Candidate: prior / visits / win"));
            for (int i = 0; i < candidates.Count; i++)
            {
                JsonNode? c = candidates[i];
                if (c is null)
                {
                    continue;
                }

                var row = new Grid { ColumnDefinitions = { new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) }, new ColumnDefinition { Width = GridLength.Auto } } };
                string refused = c["Refused"] is JsonNode why ? $" (refused: {why})" : string.Empty;
                if (c["PrunedWorlds"]?.GetValue<int>() is int cut and > 0)
                {
                    // Branch trimming cut it in some worlds: its visits and value are only the other worlds'.
                    refused += $" (cut by trimming in {cut} world{(cut == 1 ? string.Empty : "s")})";
                }

                TextBlock words = AdvisorText(12, c["Chosen"]?.GetValue<bool>() == true ? Theme.Fg : Theme.FgSoft).With(
                    $"{c["Label"]}{refused}\n   {Percent(c["Prior"])} / {Percent(c["VisitShare"])} ({c["Visits"]}) / {(c["WinPct"] is JsonNode w ? $"{w}%" : "-")}");
                row.Children.Add(words);
                if (!answered && c["Payload"] is not null)
                {
                    int index = i;
                    var play = new DockButton("Play", primary: false, fontSize: 11) { Tooltip = "Send this answer as yours", VerticalAlignment = VerticalAlignment.Center };
                    play.Clicked += () => PlayAdvice(index);
                    Grid.SetColumn(play, 1);
                    row.Children.Add(play);
                    _advisorControls.RemoveAll(control => control.Name == $"candidate {index + 1}");
                    _advisorControls.Add(($"candidate {index + 1}", play));
                }

                _advisorCandidates.Children.Add(row);
            }
        }

        // How much the sampled worlds disagreed.
        var worlds = new StringBuilder();
        if (current["Search"] is JsonNode search)
        {
            worlds.Append(CultureInfo.InvariantCulture, $"Searched {search["Simulations"]} lines over {search["Worlds"]} worlds in {Seconds(search["SearchMs"])}.");
        }

        if (current["Fusion"] is JsonNode fusion)
        {
            worlds.Append(CultureInfo.InvariantCulture, $" Worlds' best move: {fusion["Plurality"]} of {fusion["Worlds"]} agree ({fusion["Distinct"]} different).");
        }

        if (current["WorldValues"] is JsonNode spread)
        {
            worlds.Append(CultureInfo.InvariantCulture, $" Win chance across worlds {Win(spread["Min"])}-{Win(spread["Max"])} (sd {Points(spread["Sd"])}).");
        }

        _advisorWorlds.Text = worlds.ToString().Trim();

        // The belief's guesses at the opponent's hand, beside the prior.
        if (current["Belief"] is JsonArray belief && belief.Count > 0 && belief[0] is JsonNode seat)
        {
            IEnumerable<string> cards = (seat["Cards"] as JsonArray ?? []).Take(6)
                .Select(c => $"{c?["Card"]} {Percent(c?["Belief"])} (prior {Percent(c?["Prior"])})");
            _advisorBelief.Text = $"Their hand ({seat["HiddenInHand"]} unknown), belief: {string.Join("; ", cards)}";
        }
    }

    private static string Truth(JsonNode? state)
    {
        if (state?["Truth"]?["Seats"] is not JsonArray seats)
        {
            return string.Empty;
        }

        var text = new StringBuilder("At your last decision:\n");
        foreach (JsonNode? seat in seats)
        {
            text.Append(CultureInfo.InvariantCulture, $"Seat {seat?["Seat"]} hand: {string.Join(", ", (seat?["Hand"] as JsonArray ?? []).Select(c => c?.ToString()))}\n");
            text.Append(CultureInfo.InvariantCulture, $"Seat {seat?["Seat"]} library top: {string.Join(", ", (seat?["LibraryTop"] as JsonArray ?? []).Take(5).Select(c => c?.ToString()))}\n");
        }

        return text.ToString().TrimEnd();
    }

    private static string Percent(JsonNode? share) =>
        share?.GetValue<double>() is double p ? (p * 100).ToString("0", CultureInfo.InvariantCulture) + "%" : "-";

    private static string Win(JsonNode? value) =>
        value?.GetValue<double>() is double v ? ((v + 1) * 50).ToString("0", CultureInfo.InvariantCulture) + "%" : "-";

    private static string Points(JsonNode? value) =>
        value?.GetValue<double>() is double v ? (v * 50).ToString("0.0", CultureInfo.InvariantCulture) + " pts" : "-";

    private static string Seconds(JsonNode? ms) =>
        ms?.GetValue<double>() is double v ? (v / 1000).ToString("0.0", CultureInfo.InvariantCulture) + " s" : "-";

    /// <summary>The panel as a test reads it (<c>globalThis.cantripAdvisor</c>, under <c>?test</c>), its controls by name.</summary>
    private string AdvisorTestState()
    {
        using var buffer = new MemoryStream();
        using (var json = new Utf8JsonWriter(buffer))
        {
            JsonNode? current = _advisorState?["Current"];
            json.WriteStartObject();
            json.WriteNumber("version", _advisorVersion);
            json.WriteString("status", current?["Status"]?.GetValue<string>() ?? string.Empty);
            json.WriteNumber("requestId", current?["RequestId"]?.GetValue<int>() ?? 0);
            json.WriteBoolean("matchesPrompt", CurrentAdvice() is not null);
            json.WriteBoolean("trivial", current?["Trivial"]?.GetValue<bool>() == true);
            json.WriteBoolean("answered", current?["Answered"]?.GetValue<bool>() == true);
            json.WriteBoolean("over", _advisorState?["Game"]?["Over"]?.GetValue<bool>() == true);
            json.WriteString("choice", _advisorChoice.Text);
            json.WriteString("statusText", _advisorStatus.Text);
            json.WriteString("worlds", _advisorWorlds.Text);
            json.WriteString("belief", _advisorBelief.Text);
            json.WriteString("truth", _advisorTruth.Text);
            json.WriteString("history", _advisorHistory.Text);
            json.WriteNumber("candidates", (current?["Candidates"] as JsonArray)?.Count ?? 0);
            json.WriteStartArray("controls");
            foreach ((string name, FrameworkElement element) in _advisorControls)
            {
                if (element.Visibility != Visibility.Visible || element.ActualWidth <= 0 || !IsShown(element))
                {
                    continue;
                }

                Rect box = element.TransformToVisual(Root).TransformBounds(new Rect(0, 0, element.ActualWidth, element.ActualHeight));
                json.WriteStartObject();
                json.WriteString("name", name);
                json.WriteNumber("x", box.X);
                json.WriteNumber("y", box.Y);
                json.WriteNumber("width", box.Width);
                json.WriteNumber("height", box.Height);
                json.WriteEndObject();
            }

            json.WriteEndArray();
            json.WriteEndObject();
        }

        return Encoding.UTF8.GetString(buffer.ToArray());
    }
}

/// <summary>Sets a text block's text in an expression.</summary>
internal static class AdvisorTextExtensions
{
    public static TextBlock With(this TextBlock block, string text)
    {
        block.Text = text;
        return block;
    }
}
