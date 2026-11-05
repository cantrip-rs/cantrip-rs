using System.Globalization;
using System.Text;
using Cantrip.Protocol;

namespace Cantrip.Client.Llm;

/// <summary>
/// A seat played by a language model.
/// <para>
/// The third way a seat can be answered, beside <see cref="AutoPlay"/>, which plays the same game every time, and
/// <see cref="RandomPlay"/>, which plays a different one. This one is asked. It is not a better bot by
/// construction — the point of it is to find out whether a general model, told the rules of nothing and shown only
/// what a player can see, can beat the one that draws its choices out of a hat. If it cannot, that is the finding.
/// </para>
/// <para>
/// What it is shown is what the server sent this seat and nothing else: <see cref="GameText"/> writes the position
/// out of the same <see cref="PlayerView"/> the web client draws, so a hidden zone is a count here as it is on
/// screen. What it may answer is what the prompt offered, turned into lettered choices by
/// <see cref="Decisions"/> — so a model that answers nonsense plays badly and can never play illegally.
/// </para>
/// <para>
/// Two things are decided without asking, and both are arithmetic rather than judgement: which land to tap for a
/// cost, and how a division must run when the rules leave one way open. Everything a player would think about is
/// asked.
/// </para>
/// <para>
/// It blocks while the model thinks. That is why this bot plays one game at a time in its own tool rather than a
/// thousand at once in <c>cantrip-playtest</c> — see <see cref="ILanguageModel"/>.
/// </para>
/// </summary>
public sealed class LlmPlay : IAutoPlayer
{
    /// <summary>
    /// How many times one refused answer is tried again before this seat stops arguing and takes the prompt's own.
    /// <para>
    /// A prompt does not carry every rule its answer is judged by — a creature that must block, an attack
    /// restriction — so an answer built only out of what it offered can still be refused. Being told so and asked
    /// again finds a legal one when there is one; the prompt's default is the way out when there is not.
    /// </para>
    /// </summary>
    private const int Attempts = 2;

    /// <summary>How many lines of "what happened while you were not being asked" one preamble carries.</summary>
    private const int Recent = 40;

    private readonly ILanguageModel _model;
    private readonly string _deck;
    private readonly List<string> _log = [];

    /// <summary>How far through the server's log this seat has read, so one event is never narrated twice.</summary>
    private long _read = -1;

    private ModelChooser? _chooser;
    private Decisions? _decisions;

    /// <summary>The question last answered, so the same one asked again is recognised as a refusal.</summary>
    private int _askedId = -1;

    /// <summary>How many times <see cref="_askedId"/> has come back.</summary>
    private int _attempts;

    /// <summary>How many refusals this seat has been sent, so a new one can be told apart from an old one.</summary>
    private int _refusals;

    /// <summary>The decision being made somewhere else, or null while none is. See <see cref="Background"/>.</summary>
    private Task<AnswerPayload>? _deciding;

    /// <param name="model">Who answers the questions.</param>
    /// <param name="deck">
    /// This seat's decklist, as names and counts. Put in front of the model once, at the start: what is left in the
    /// library is most of what a Magic decision turns on, and it is the one thing a player knows that no projection
    /// of the position can tell them.
    /// </param>
    /// <param name="background">
    /// Whether to think off the calling thread, answering a frame or two later rather than holding the caller.
    /// <para>
    /// Off for a harness, which has nothing else to do and reads more simply for waiting. On for a person playing
    /// against this seat, where holding the caller means holding the window: a model takes seconds, and a client
    /// that stopped drawing for each of them would be a client nobody could play against.
    /// </para>
    /// </param>
    public LlmPlay(ILanguageModel model, string deck = "", bool background = false)
    {
        ArgumentNullException.ThrowIfNull(model);
        _model = model;
        _deck = deck;
        Background = background;
    }

    /// <summary>Whether this seat thinks off the calling thread.</summary>
    public bool Background { get; }

    /// <summary>What went wrong the last time a decision could not be got at all, or empty. For showing.</summary>
    public string Failure { get; private set; } = string.Empty;

    /// <summary>Every question and the answer it got, for a run printing the game as it is played.</summary>
    public Action<string, string>? Watching { get; set; }

    /// <summary>Everything sent to the model, position and all, for a run that wants to see what it was told.</summary>
    public Action<string>? Sending { get; set; }

    /// <summary>How many questions this seat has put to the model.</summary>
    public int Questions { get; private set; }

    /// <summary>How many prompts it has answered, asked about or not.</summary>
    public int Answers { get; private set; }

    /// <summary>How many of those fell back to the prompt's own answer because nothing usable came back.</summary>
    public int Fallbacks { get; private set; }

    public bool Retrying => _attempts < Attempts;

    /// <summary>
    /// Whether the model is asked about a draw another player offers (docs/rules-exceptions.md, RX-4). Off, so this seat
    /// declines as every automated seat does for now; on, the model is asked to accept or decline, and a reply that names
    /// neither declines.
    /// </summary>
    public bool AsksAboutDraws { get; init; }

    /// <summary>How many draws offered the model has been asked about.</summary>
    public int DrawsAsked { get; private set; }

    public bool AcceptsDraw(GameClient client)
    {
        ArgumentNullException.ThrowIfNull(client);
        if (!AsksAboutDraws || client.DrawOffer is not { } offer)
        {
            return false;
        }

        Record(client);
        Ready(client);
        _chooser!.Position = Preamble(client);
        DrawsAsked++;
        int before = _model.Usage.Calls;
        try
        {
            return _chooser.Choose(DrawQuestion(client, offer)) is [0];
        }
        catch (NoChoiceException)
        {
            return false;
        }
        finally
        {
            Questions += _model.Usage.Calls - before;
        }
    }

    /// <summary>The question a draw offered is put as: accept it, or play on.</summary>
    public static Question DrawQuestion(GameClient client, DrawOfferMessage offer)
    {
        ArgumentNullException.ThrowIfNull(client);
        ArgumentNullException.ThrowIfNull(offer);
        return new Question
        {
            Heading = string.Create(CultureInfo.InvariantCulture, $"The Opponent in seat {offer.Offerer} offers a draw. Do you accept?"),
            Choices = ["Accept the draw: the game ends with nobody winning, once every other player has accepted too.", "Decline, and play on."],
        };
    }

    public bool Thinking => _deciding is { IsCompleted: false };

    public bool Act(GameClient client)
    {
        ArgumentNullException.ThrowIfNull(client);

        // A decision already under way somewhere else. Nothing here reads the client until it is finished, because
        // the thread making it is reading the client — which is also why AutoSeat stops pumping while Thinking.
        if (_deciding is { } deciding)
        {
            if (!deciding.IsCompleted)
            {
                return false;
            }

            _deciding = null;
            client.Answer(Reached(deciding, client.Prompt!));
            Answers++;
            return true;
        }

        // Read before answering and whether or not this seat is being asked anything: most of what happens in a game
        // happens on somebody else's turn, and a handle only names a card while this seat can still see it.
        Record(client);

        if (client.IsOver || !client.IsWaitingOnMe)
        {
            return false;
        }

        PromptMessage asked = client.Prompt!;

        // The identical question, id and all, is what the server sends back when it refuses an answer.
        if (asked.RequestId == _askedId)
        {
            _attempts++;
        }
        else
        {
            _askedId = asked.RequestId;
            _attempts = 0;
        }

        Ready(client);
        _chooser!.Position = Preamble(client);

        if (Background)
        {
            // Handed off and picked up again in a later frame. The client is not touched here again until it
            // finishes, so the position it is reading cannot change halfway through being read.
            _deciding = Task.Run(() => Decide(client, asked));
            return false;
        }

        client.Answer(Decide(client, asked));
        Answers++;
        return true;
    }

    /// <summary>
    /// What a finished decision came to, or the prompt's own answer if it came to nothing.
    /// <para>
    /// A model reached across a network can fail in ways a local one cannot — a timeout, a provider falling over,
    /// a key that stopped working. None of those may end the game: the prompt's own answer is always legal, so a
    /// seat whose model has gone away plays badly and the person across the table can finish the game.
    /// </para>
    /// </summary>
    private AnswerPayload Reached(Task<AnswerPayload> deciding, PromptMessage asked)
    {
        if (deciding.IsCompletedSuccessfully)
        {
            return deciding.Result;
        }

        Failure = deciding.Exception?.GetBaseException().Message ?? "the model did not answer";
        Fallbacks++;
        return asked.Prompt.Default();
    }

    private AnswerPayload Decide(GameClient client, PromptMessage asked)
    {
        if (_attempts >= Attempts)
        {
            Fallbacks++;
            return asked.Prompt.Default();
        }

        int before = _model.Usage.Calls;

        try
        {
            return _decisions!.Answer(client, asked.Prompt);
        }
        catch (NoChoiceException)
        {
            // The model could not name a letter. The prompt's own answer is always legal, so the game goes on and the
            // run's report says how often this happened — a seat that falls back often is not playing.
            Fallbacks++;
            return asked.Prompt.Default();
        }
        finally
        {
            Questions += _model.Usage.Calls - before;
        }
    }

    /// <summary>
    /// Builds the conversation, once the server has said which seat this is.
    /// <para>
    /// Not in the constructor, because a seat does not know who it is or who it is playing against until the first
    /// view arrives, and the briefing is mostly those two facts.
    /// </para>
    /// </summary>
    private void Ready(GameClient client)
    {
        if (_chooser is not null)
        {
            return;
        }

        _chooser = new ModelChooser(_model, Briefing(client)) { Watching = Watching, Sending = Sending };
        _decisions = new Decisions(_chooser);
    }

    /// <summary>The standing instructions: who this seat is, what it is holding, and how an answer is written.</summary>
    private string Briefing(GameClient client)
    {
        var text = new StringBuilder();
        int others = client.Opponents.Count();

        // Nobody is named, here or anywhere else this seat reads. See GameText.Seat: the client calls its human
        // player "You", and a seat told its opponent's name was choosing between "you" and "You".
        text.Append("You are playing a game of Magic: The Gathering. You are the player called ")
            .Append(CultureInfo.InvariantCulture, $"Yourself, in seat {client.Seat}.")
            .Append(others == 1
                ? " There is one other player, called Opponent."
                : string.Create(
                    CultureInfo.InvariantCulture,
                    $" There are {others} other players, each called Opponent and told apart by seat number."));

        text.Append("""


            You will be shown the position from your seat and asked a question with lettered choices. Reply with the
            letter, and nothing else. A numeric question instead gives an inclusive range: reply with the whole number
            it asks for, exactly as its instructions say. Every choice you are offered is legal; the rules have already been checked, so
            you are choosing what is good, not what is allowed. Anything you are not asked about — which land to tap
            for a cost, and the like — is handled for you.

            You are playing to win: reduce your opponent to zero life, or win by whatever your cards do instead. Think
            about what your opponent can do on their turn before you tap out on yours, keep enough back to block when
            you are behind on the board, and do not spend a removal spell on something that was not going to matter.
            Passing is a real choice: an instant kept for their turn is often worth more than the same card cast on
            yours.

            Two things cost most games at this level, so weigh them every time they come up. In combat, compare each
            attacker against what could block it before you send it: an attacker that trades down, or dies for
            nothing, has cost you a card to deal no damage, and a creature that stays home is a blocker on the way
            back. And an ability you may activate at any time is usually worth more unused than spent — an ability
            held up answers a removal spell on your turn or theirs, while the same ability spent to push one damage
            has bought a point of damage and given up the answer.

            Objects carry a #number so that two copies of the same card can be told apart. It is only a label; you
            never answer with one.
            """);

        if (_deck.Length > 0)
        {
            text.Append("\n\nThis is your whole deck. Anything not in play, in your hand or in your graveyard is still ")
                .Append("in your library somewhere:\n\n")
                .Append(_deck.Trim());
        }

        return text.ToString();
    }

    /// <summary>The position, what has happened since this seat last acted, and any complaint the server made.</summary>
    private string Preamble(GameClient client)
    {
        var text = new StringBuilder(GameText.Position(client));

        // A decision of a player this seat controls (CR 723.5): "you" in the question below is that player.
        if (client.Prompt is { ForSeat: > 0 } forOther)
        {
            text.Append("\n\nThe question below is ")
                .Append(GameText.Whose(client, forOther.ForSeat))
                .Append(" to answer, and you answer it for them: \"you\" in it means them, and what it costs is paid with their cards and mana.");
        }

        if (_log.Count > 0)
        {
            text.Append("\n\nSince you last acted:");

            foreach (string line in _log.Count > Recent ? _log.Skip(_log.Count - Recent) : _log)
            {
                text.Append("\n  ").Append(line);
            }

            _log.Clear();
        }

        if (client.Errors.Count > _refusals)
        {
            _refusals = client.Errors.Count;
            ErrorMessage refused = client.Errors[^1];

            text.Append("\n\nYour last answer was refused by the game (")
                .Append(refused.Detail.ReplaceLineEndings(" "))
                .Append("). Something about it broke a rule the question did not state. Choose differently.");
        }

        return text.ToString();
    }

    /// <summary>Narrates anything new in the server's log, at the moment it arrives.</summary>
    private void Record(GameClient client)
    {
        foreach (GameEventView entry in client.RecentEvents)
        {
            long sequence = client.SequenceOf(entry);

            // A batch can be pumped more than once between answers, so the sequence is what says whether this line
            // has already been written.
            if (sequence >= 0 && sequence <= _read)
            {
                continue;
            }

            if (sequence > _read)
            {
                _read = sequence;
            }

            if (GameText.Line(client, entry) is not { Length: > 0 } line)
            {
                continue;
            }

            _log.Add(line);

            // Anything worth a line is worth reconsidering a decision to sit out the turn — except the step header,
            // which is the one thing that is always true and never news. A turn moving on is exactly what this seat
            // said it did not want to be asked about again.
            if (entry is not StepBeganEvent)
            {
                _decisions?.Stir();
            }
        }
    }
}
