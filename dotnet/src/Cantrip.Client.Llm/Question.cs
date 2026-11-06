using System.Text;

namespace Cantrip.Client.Llm;

/// <summary>
/// One thing to decide, as a keyed multiple choice.
/// <para>
/// Every decision this seat makes is one of these, and the whole answer is an index into
/// <see cref="Choices"/> — never a card name, a handle, or a sentence. That is what makes an answer impossible
/// to get wrong in a way the server would refuse: the choices are built only out of what a prompt said was
/// allowed, so whichever one comes back was legal when it was offered.
/// </para>
/// <para>
/// A question that offers one choice is never asked. Most of a game of Magic is holding priority with nothing
/// worth doing, and a seat that asked about every one of those would spend most of its money on the word "pass".
/// </para>
/// </summary>
public sealed class Question
{
    /// <summary>What is being decided, as a sentence: "You have priority. What do you do?"</summary>
    public required string Heading { get; init; }

    /// <summary>What may be chosen, each written so it can be read on its own. Answers are indexes into this.</summary>
    public required IReadOnlyList<string> Choices { get; init; }

    /// <summary>How many to choose, or with <see cref="UpTo"/> the most that may be.</summary>
    public int Picks { get; init; } = 1;

    /// <summary>Whether fewer than <see cref="Picks"/> may be chosen, down to none.</summary>
    public bool UpTo { get; init; }

    /// <summary>
    /// Anything the chooser needs alongside the choices but cannot read off them: what is already declared this
    /// combat, what a rule requires of the answer. Empty for most questions.
    /// </summary>
    public string Note { get; init; } = string.Empty;

    /// <summary>Whether there is anything here to decide. A question with one answer is answered without asking.</summary>
    public bool Trivial => Choices.Count <= 1 && !UpTo;

    /// <summary>
    /// The key for one choice: <c>A</c> to <c>Z</c>, then <c>AA</c> onwards. Letters rather than numbers, because a
    /// question about a card is full of numbers — a cost, a power, a life total — and a key has to be the one token
    /// in the question that could not be anything else.
    /// </summary>
    public static string Key(int index)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(index);

        string key = string.Empty;

        for (int left = index; ; left = (left / 26) - 1)
        {
            key = (char)('A' + (left % 26)) + key;

            if (left < 26)
            {
                return key;
            }
        }
    }

    /// <summary>The choice one key names, or -1 for a key this question does not offer.</summary>
    public int IndexOf(string key)
    {
        for (int i = 0; i < Choices.Count; i++)
        {
            if (string.Equals(Key(i), key, StringComparison.OrdinalIgnoreCase))
            {
                return i;
            }
        }

        return -1;
    }

    /// <summary>The question as it is asked, keys and all.</summary>
    public override string ToString()
    {
        var text = new StringBuilder();
        text.Append(Heading);

        if (Note.Length > 0)
        {
            text.Append('\n').Append(Note);
        }

        text.Append('\n');

        for (int i = 0; i < Choices.Count; i++)
        {
            text.Append("\n  ").Append(Key(i)).Append(") ").Append(Choices[i]);
        }

        text.Append("\n\n").Append(Instruction());
        return text.ToString();
    }

    /// <summary>
    /// How the answer is written, said in the question so a reply never has to be guessed at.
    /// <para>
    /// One line of reasoning before the letter, and it earns its keep twice over. A model that states the tradeoff
    /// before choosing tends to choose better, and a game that went badly is otherwise unaccountable — a bad move
    /// and a sensible answer to a badly worded question read exactly alike from the outside. It costs a few dozen
    /// output tokens against the several thousand input tokens the question itself costs.
    /// </para>
    /// <para>
    /// The letter goes last and alone, which is what <see cref="ModelChooser"/> reads: a line that begins with
    /// anything else is reasoning, not an answer.
    /// </para>
    /// </summary>
    private string Instruction()
    {
        string letters = (Picks, UpTo) switch
        {
            (1, false) => "the letter of your choice, and nothing else",
            (_, false) => $"exactly {Picks} letters separated by commas, and nothing else",
            (1, true) => "the letter of your choice, or NONE to choose nothing",
            (_, true) => $"up to {Picks} letters separated by commas, or NONE to choose nothing",
        };

        return $"First give one short line saying why. Then, on the last line, write {letters}.";
    }
}

/// <summary>Whoever answers a <see cref="Question"/>: the language model in a game, a fixed list in a test.</summary>
public interface IChooser
{
    /// <summary>
    /// Which choices, by index into <see cref="Question.Choices"/>. Never out of range, and never more than the
    /// question asked for.
    /// </summary>
    /// <exception cref="NoChoiceException">No answer could be got at all, so the caller falls back to the prompt's own.</exception>
    IReadOnlyList<int> Choose(Question question);

    /// <summary>An exact whole number in the question's inclusive bounds, without enumerating the range.</summary>
    uint ChooseNumber(NumberQuestion question) => throw new NoChoiceException("This chooser does not answer numeric questions.");
}

/// <summary>A bounded scalar question, separate from keyed choices so every numeric value stays representable.</summary>
public sealed class NumberQuestion
{
    public required string Heading { get; init; }
    public required uint Least { get; init; }
    public required uint Most { get; init; }
    public override string ToString() => $"{Heading}\nChoose a whole number from {Least} to {Most}, inclusive.\n\nFirst give one short line saying why. Then, on the last line, write only the whole number.";
}

/// <summary>
/// Nothing usable came back, after asking as many times as the chooser is willing to.
/// <para>
/// Thrown rather than answered around, because a decision half made is worse than one not made: the caller gives up
/// on the whole prompt and sends the prompt's own default, which is always legal, instead of assembling an answer
/// out of one real choice and one invented one.
/// </para>
/// </summary>
public sealed class NoChoiceException : Exception
{
    public NoChoiceException()
        : this("No choice could be got.")
    {
    }

    public NoChoiceException(string message)
        : base(message)
    {
    }

    public NoChoiceException(string message, Exception inner)
        : base(message, inner)
    {
    }
}
