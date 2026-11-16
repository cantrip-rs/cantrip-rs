using Cantrip.Datasets;
using Cantrip.Decks;
using Cantrip.Server.Rules;

namespace Cantrip.Server.Decks;

/// <summary>
/// The decks a host offers for a constructed format: every list in a folder (<c>decks/standard</c>) whose cards the engine
/// has and that is legal in the format, as the server checks every deck it deals (<see cref="DeckValidation"/>). The
/// Discord bot's Standard gauntlet and in-person constructed play offer the same ones.
/// </summary>
public static class FormatDecks
{
    /// <summary>What a folder of lists came to: the decks offered, how many name cards the engine lacks, and why each other one is not legal.</summary>
    public sealed record Loaded(IReadOnlyList<DeckList> Decks, int Unplayable, IReadOnlyList<string> Illegal);

    /// <summary>Every list in <paramref name="directory"/> that is playable and legal in <paramref name="format"/>, in file order.</summary>
    public static Loaded Load(string directory, DeckFormat format, string contentRoot, IRulesHostFactory? rules = null)
    {
        ArgumentNullException.ThrowIfNull(format);
        List<DeckList> decks = [];
        List<string> illegal = [];
        int unplayable = 0;
        if (!Directory.Exists(directory))
        {
            return new Loaded(decks, 0, illegal);
        }

        foreach (string path in Directory.GetFiles(directory, "*.txt").Order(StringComparer.Ordinal))
        {
            try
            {
                DeckList candidate = DeckList.Load(path);
                candidate.Resolve([ContentCache.Get(contentRoot).Cards]);
                IReadOnlyList<string> problems = DeckValidation.Problems(DeckValidation.Of(candidate, format), format,
                    DeckCatalog.ByName(contentRoot), rules ?? RulesHostFactory.Native);
                if (problems.Count > 0)
                {
                    illegal.Add($"{candidate.Name}: {problems[0]}");
                    continue;
                }

                decks.Add(candidate);
            }
            catch (Exception error) when (error is IOException or InvalidOperationException or FormatException)
            {
                unplayable++;
            }
        }

        return new Loaded(decks, unplayable, illegal);
    }

    /// <summary>
    /// A deck as the list names it: its file's first comment up to a colon (<c># Dimir Midrange: a Standard meta deck</c>),
    /// else its file name in words.
    /// </summary>
    public static string Title(DeckList deck)
    {
        ArgumentNullException.ThrowIfNull(deck);
        try
        {
            if (File.ReadLines(deck.Source).FirstOrDefault() is { } first && first.StartsWith('#'))
            {
                string title = first.TrimStart('#').Split(':', 2)[0].Trim();
                if (title.Length > 0) return title;
            }
        }
        catch (IOException)
        {
            // Named from the file instead.
        }

        return string.Join(' ', deck.Name.Split('-', StringSplitOptions.RemoveEmptyEntries).Select(word => char.ToUpperInvariant(word[0]) + word[1..]));
    }
}
