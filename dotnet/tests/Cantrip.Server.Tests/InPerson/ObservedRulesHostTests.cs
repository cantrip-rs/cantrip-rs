using System.Reflection;
using Cantrip.Protocol;
using Cantrip.Server.InPerson;
using Cantrip.Server.Rules;
using Xunit;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>
/// The rules seam in-person play runs the ordinary table through: a host that is another, which the table must not be
/// able to tell from the host it wraps.
/// </summary>
public sealed class ObservedRulesHostTests
{
    private sealed class Watcher : IStepObserver
    {
        public List<(IRulesHost Game, RulesStep Step, bool Started)> Steps { get; } = [];

        public List<IRulesHost> Closed { get; } = [];

        public void Stepped(IRulesHost game, RulesStep step, bool started) => Steps.Add((game, step, started));

        void IStepObserver.Closed(IRulesHost game) => Closed.Add(game);
    }

    /// <summary>
    /// Every member of <see cref="IRulesHost"/>, the ones with a default body too, is implemented by the wrapper itself:
    /// a member left to its default would answer for the wrapper, not forward to the host it wraps, and a member added to
    /// the interface later fails this until the wrapper forwards it.
    /// </summary>
    [Fact]
    public void Every_member_of_the_rules_host_is_forwarded()
    {
        InterfaceMapping map = typeof(ObservedRulesHost).GetInterfaceMap(typeof(IRulesHost));
        for (int i = 0; i < map.InterfaceMethods.Length; i++)
        {
            Assert.True(map.TargetMethods[i].DeclaringType == typeof(ObservedRulesHost),
                $"{map.InterfaceMethods[i].Name} is not forwarded by {nameof(ObservedRulesHost)}.");
        }

        InterfaceMapping disposing = typeof(ObservedRulesHost).GetInterfaceMap(typeof(IDisposable));
        Assert.Equal(typeof(ObservedRulesHost), Assert.Single(disposing.TargetMethods).DeclaringType);
    }

    /// <summary>
    /// What each member returns is the wrapped host's, and each call reaches it: every member called through a recording
    /// proxy, by reflection, so none is missed.
    /// </summary>
    [Fact]
    public void Every_member_reaches_the_wrapped_host()
    {
        IRulesHost proxy = DispatchProxy.Create<IRulesHost, Recorder>();
        var inner = (Recorder)(object)proxy;
        var observed = new ObservedRulesHost(proxy, new Watcher());

        foreach (MethodInfo method in typeof(IRulesHost).GetMethods().Concat(typeof(IDisposable).GetMethods()))
        {
            object?[] arguments = [.. method.GetParameters().Select(parameter => Default(parameter.ParameterType))];
            object? returned = method.Invoke(observed, arguments);
            Assert.Equal(method.Name, inner.Calls[^1]);
            if (method.ReturnType.IsValueType && method.ReturnType != typeof(void)) Assert.Equal(Recorder.Returned(method.ReturnType), returned);
            else if (method.ReturnType != typeof(void)) Assert.Same(Recorder.Returned(method.ReturnType), returned);
        }

        Assert.Equal(typeof(IRulesHost).GetMethods().Length + 1, inner.Calls.Count);
    }

    [Fact]
    public void Each_step_is_shown_to_the_observer_as_the_table_is_given_it()
    {
        var inner = new PublicHost();
        inner.Steps.Enqueue(PublicHost.Step([], sequence: 2));
        var watcher = new Watcher();
        IRulesHost game = new ObservedRulesFactory(new Single(inner), watcher).Create([], 7, new RulesGameOptions { MinimumDeckSize = 40 });

        RulesStep opening = game.Start();
        RulesStep answered = game.Answer(1, 1, new PassAnswer());
        RulesStep conceded = game.Concede(2);
        RulesStep left = game.Leave(1);
        Assert.Null(game.End(1));
        game.Dispose();

        Assert.Equal([(opening, true), (answered, false), (conceded, false), (left, false)], watcher.Steps.Select(each => (each.Step, each.Started)));
        Assert.All(watcher.Steps, each => Assert.Same(game, each.Game));
        Assert.Same(game, Assert.Single(watcher.Closed));
        Assert.True(inner.Disposed);
        Assert.Equal([(1, 1)], inner.Answers);
    }

    [Fact]
    public void A_refused_answer_is_the_tables_to_hear_and_shows_nothing()
    {
        var watcher = new Watcher();
        var game = new ObservedRulesHost(new PublicHost(), watcher);
        Assert.Throws<InvalidOperationException>(() => game.Answer(1, 1, new PassAnswer()));
        Assert.Empty(watcher.Steps);
    }

    private sealed class Single(IRulesHost host) : IRulesHostFactory
    {
        public IRulesHost Create(IReadOnlyList<RulesSeat> seats, ulong seed, RulesGameOptions options) => host;
    }

    private static object? Default(Type type) => type.IsValueType ? Activator.CreateInstance(type) : null;

    /// <summary>A host that records each member called on it and answers each with one object per return type.</summary>
    public class Recorder : DispatchProxy
    {
        private static readonly Dictionary<Type, object> Answers = new()
        {
            [typeof(IReadOnlyList<int>)] = new List<int> { 1, 2 },
            [typeof(IReadOnlyList<string>)] = new List<string> { "A", "B" },
            [typeof(IReadOnlyList<DeckCardView>)] = new List<DeckCardView>(),
            [typeof(RulesStep)] = PublicHost.Step([]),
            [typeof(SelectionVerdictView)] = new SelectionVerdictView(),
            [typeof(BlockVerdictView)] = new BlockVerdictView(),
            [typeof(AttackVerdictView)] = new AttackVerdictView(),
            [typeof(PlayerView)] = new PlayerView(),
            [typeof(ObjectView)] = new ObjectView(),
            [typeof(IReadOnlyList<RulesAnswer>)] = new List<RulesAnswer>(),
            [typeof(int)] = 41,
            [typeof(bool)] = true,
            [typeof(GameOutcomeView)] = GameOutcomeView.Draw,
        };

        public List<string> Calls { get; } = [];

        /// <summary>What a member returning <paramref name="type"/> answers; a value type boxed afresh, so compared by value.</summary>
        public static object Returned(Type type) => Answers[Nullable.GetUnderlyingType(type) ?? type];

        protected override object? Invoke(MethodInfo? targetMethod, object?[]? args)
        {
            Calls.Add(targetMethod!.Name);
            return targetMethod.ReturnType == typeof(void) ? null : Returned(targetMethod.ReturnType);
        }
    }
}
