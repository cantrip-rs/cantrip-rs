using System.Globalization;
using System.Text;

namespace Cantrip.Client.Llm;

/// <summary>
/// Answers questions by asking a language model, and holds the conversation the model is answering inside.
/// <para>
/// A running conversation rather than a fresh request per decision, because a plan is most of what playing Magic
/// well consists of: a seat that is holding up two mana for a counterspell has to remember that it is, and a seat
/// asked each question in isolation cannot. The model sees what it decided a moment ago and why.
/// </para>
/// <para>
/// The conversation is bounded all the same. Every question carries the whole position with it, so an old exchange
/// is worth only the intent in it — <see cref="Memory"/> of them are kept and the rest dropped, which is what keeps
/// a two-hundred-decision game from ending each request with a hundred thousand tokens of history.
/// </para>
/// <para>
/// A reply that names no choice is not an answer. It is asked again, once, in plainer terms; after that the caller
/// is told there is no choice and falls back to the prompt's own, which is always legal. A model that cannot say
/// "B" never stalls the game, it only plays worse.
/// </para>
/// </summary>
public sealed class ModelChooser : IChooser
{
    /// <summary>How many question-and-answer pairs are kept behind the current one.</summary>
    public const int Memory = 10;

    /// <summary>How many times one question is put before it is given up on.</summary>
    private const int Attempts = 2;

    /// <summary>How much of a reply is remembered. Enough for the reasoning, not enough to grow without bound.</summary>
    private const int Remembered = 600;

    private readonly ILanguageModel _model;
    private readonly List<ChatTurn> _conversation;
    private readonly int _memory;

    /// <param name="model">Who answers.</param>
    /// <param name="briefing">The standing instructions: who this seat is, what its deck is, how to answer.</param>
    /// <param name="memory">How many earlier exchanges to keep.</param>
    public ModelChooser(ILanguageModel model, string briefing, int memory = Memory)
    {
        ArgumentNullException.ThrowIfNull(model);
        ArgumentException.ThrowIfNullOrWhiteSpace(briefing);

        _model = model;
        _memory = Math.Max(memory, 0);
        _conversation = [new ChatTurn(ChatRole.System, briefing)];
    }

    /// <summary>
    /// The position and what has happened since this seat last acted, put in front of the next question and then
    /// cleared. Set once per prompt: the follow-up questions of one decision — which target, how large X — are about
    /// a board that has not moved, and repeating it would pay for the same page twice.
    /// </summary>
    public string Position { get; set; } = string.Empty;

    /// <summary>Every question and answer as it happens, for a run that wants to print the game as it is played.</summary>
    public Action<string, string>? Watching { get; set; }

    /// <summary>
    /// The whole message going out, position and all, for a run that wants to see exactly what the model was told.
    /// Separate from <see cref="Watching"/> because it is a page each time and most runs want the one line.
    /// </summary>
    public Action<string>? Sending { get; set; }

    public IReadOnlyList<int> Choose(Question question)
    {
        ArgumentNullException.ThrowIfNull(question);

        string asked = Asked(question.ToString());

        for (int attempt = 1; attempt <= Attempts; attempt++)
        {
            string reply = Reply(asked);

            if (Read(question, reply) is { } picks)
            {
                Watching?.Invoke(question.ToString(), Chosen(question, picks, reply));
                return picks;
            }

            _model.Usage.Retried();

            // Asked again rather than reworded from scratch: the model can see its own unusable reply in the
            // conversation, so the second ask only has to say what was wrong with it.
            asked = "That reply did not name one of the letters. Answer again with nothing but "
                + (question.Picks > 1 ? "the letters, separated by commas." : "the letter.");
        }

        Watching?.Invoke(question.ToString(), "(no answer — falling back to the game's own default)");
        throw new NoChoiceException($"No letter came back after {Attempts} attempts.");
    }

    /// <summary>The question as it goes out: the position, once, and then what is being asked.</summary>
    private string Asked(string question)
    {
        if (Position.Length == 0)
        {
            return question;
        }

        string preamble = Position;
        Position = string.Empty;
        return preamble + "\n\n" + question;
    }

    public uint ChooseNumber(NumberQuestion question)
    {
        ArgumentNullException.ThrowIfNull(question);
        if (question.Least > question.Most) throw new NoChoiceException("The numeric range is empty.");
        if (question.Least == question.Most) return question.Least;
        string asked = Asked(question.ToString());
        for (int attempt = 1; attempt <= Attempts; attempt++)
        {
            string reply = Reply(asked);
            string last = reply.ReplaceLineEndings("\n").Split('\n').LastOrDefault(line => !string.IsNullOrWhiteSpace(line)) ?? string.Empty;
            if (uint.TryParse(Clean(last), NumberStyles.None, CultureInfo.InvariantCulture, out uint value)
                && value >= question.Least && value <= question.Most)
            {
                Watching?.Invoke(question.ToString(), $"{value.ToString(CultureInfo.InvariantCulture)}\n      [it said: {Short(reply)}]");
                return value;
            }
            _model.Usage.Retried();
            asked = $"That reply did not give a whole number from {question.Least} to {question.Most}. Answer again with only the whole number.";
        }
        Watching?.Invoke(question.ToString(), "(no answer — falling back to the game's own default)");
        throw new NoChoiceException($"No valid number came back after {Attempts} attempts.");
    }

    /// <summary>One bounded conversation exchange, shared by keyed and scalar questions.</summary>
    private string Reply(string asked)
    {
        Sending?.Invoke(asked);
        _conversation.Add(new ChatTurn(ChatRole.User, asked));
        string reply = _model.Continue(_conversation);
        _conversation.Add(new ChatTurn(ChatRole.Assistant, Short(reply)));
        Trim();
        return reply;
    }

    /// <summary>
    /// The choices one reply names, or null for a reply that names none.
    /// <para>
    /// Strict on purpose, because the way this fails is silent. Single letters are ordinary English words: the ninth
    /// key is <c>I</c> and the first is <c>A</c>, so a reader that hunted for any key in "I think attacking is a
    /// mistake" would find one and make a move nobody chose. A line counts as an answer only if it is nothing but
    /// keys, or begins with a key followed by punctuation — "B) attack" — or begins with two keys in a row, which
    /// prose does not produce by accident. Anything else is not an answer, and a question with no answer is asked
    /// again rather than guessed at.
    /// </para>
    /// <para>
    /// Lines are read last first, so a model that reasons out loud and then answers is understood.
    /// </para>
    /// </summary>
    private static IReadOnlyList<int>? Read(Question question, string reply)
    {
        string[] lines = reply.ReplaceLineEndings("\n").Split('\n');

        for (int i = lines.Length - 1; i >= 0; i--)
        {
            string line = Clean(lines[i]);

            if (line.Length == 0)
            {
                continue;
            }

            string[] raw = line.Split(
                [',', ';', ' ', '/', '|', '\t'],
                StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

            List<string> tokens = [.. raw.Select(token => token.Trim('.', ')', '(', ':', '-', '*', '`', '"', '\''))];

            if (tokens.Count == 0)
            {
                continue;
            }

            if (question.UpTo && string.Equals(tokens[0], "NONE", StringComparison.OrdinalIgnoreCase))
            {
                return [];
            }

            List<int> keys = [];

            foreach (string token in tokens)
            {
                int index = question.IndexOf(token);

                if (index < 0)
                {
                    // The run of keys has ended. Whatever follows is the model explaining itself.
                    break;
                }

                keys.Add(index);
            }

            bool answer = keys.Count > 0
                && (keys.Count == tokens.Count || keys.Count > 1 || raw[0].Length > tokens[0].Length);

            if (!answer)
            {
                continue;
            }

            List<int> picks = [];

            foreach (int key in keys)
            {
                if (!picks.Contains(key) && picks.Count < Math.Max(question.Picks, 1))
                {
                    picks.Add(key);
                }
            }

            if (picks.Count > 0)
            {
                return picks;
            }
        }

        return null;
    }

    /// <summary>A line with the markdown and the "Answer:" taken off the front of it.</summary>
    private static string Clean(string line)
    {
        string text = line.Trim().Trim('*', '`', '#', '>', ' ');

        foreach (string lead in (string[])["final answer", "my answer", "my choice", "answer", "choice"])
        {
            if (text.StartsWith(lead, StringComparison.OrdinalIgnoreCase))
            {
                string rest = text[lead.Length..].TrimStart();

                if (rest.StartsWith(':') || rest.StartsWith('='))
                {
                    text = rest[1..].Trim();
                }
            }
        }

        return text.Trim('*', '`', ' ');
    }

    /// <summary>What was chosen and why, for a watcher printing the game.</summary>
    private static string Chosen(Question question, IReadOnlyList<int> picks, string reply)
    {
        var text = new StringBuilder();

        if (picks.Count == 0)
        {
            text.Append("none of them");
        }

        foreach (int pick in picks)
        {
            text.Append(text.Length > 0 ? "; " : string.Empty);
            text.Append(CultureInfo.InvariantCulture, $"{Question.Key(pick)}) {question.Choices[pick]}");
        }

        string said = reply.ReplaceLineEndings(" ").Trim();

        // Whatever the model said beyond the key is its reasoning, and reading it is most of the point of watching a
        // game. It is only worth printing when it said something more than the key itself.
        return said.Length > 4 ? $"{text}\n      [it said: {Short(said)}]" : text.ToString();
    }

    private static string Short(string text)
    {
        string one = text.Trim();
        return one.Length <= Remembered ? one : string.Concat(one.AsSpan(0, Remembered), "...");
    }

    /// <summary>Drops the oldest exchanges, never the briefing.</summary>
    private void Trim()
    {
        int keep = 1 + (_memory * 2);

        if (_conversation.Count > keep)
        {
            _conversation.RemoveRange(1, _conversation.Count - keep);
        }
    }
}
