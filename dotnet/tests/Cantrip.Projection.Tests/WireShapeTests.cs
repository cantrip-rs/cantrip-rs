using System.Globalization;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using Cantrip.Protocol;
using MessagePack;
using Xunit;

namespace Cantrip.Projection.Tests;

/// <summary>
/// Holds the one step of adding a prompt that nothing else can catch: the <see cref="PlayerView.ProtocolVersion"/> bump.
/// <para>
/// A union key or a <c>[Key]</c> added to <c>Messages.cs</c> changes what the bytes mean. A client built before it
/// cannot read the new member and does not know that it cannot: the handshake is the only place the mismatch is
/// ever noticed, and it only notices if the version moved. Forgetting the bump is therefore invisible until two
/// builds meet, which is the worst moment to find out.
/// </para>
/// <para>
/// So the shape of every message is reduced to one line here. Change the wire and this fails, naming the new line
/// to paste back — and the instruction to bump the version at the same time. It says nothing about whether the
/// change is good, only that it is a change.
/// </para>
/// </summary>
public sealed class WireShapeTests
{
    /// <summary>
    /// The wire as it stands. Written as version and digest together so the two are updated in one edit rather than
    /// one of them being remembered — and so this line never names a version it has drifted away from.
    /// </summary>
    private const string Recorded = "219:10AE1D096D129B4E";

    [Fact]
    public void The_wire_has_not_changed_without_the_protocol_version_moving()
    {
        string digest = Digest(Shape());
        string actual = $"{PlayerView.ProtocolVersion}:{digest}";
        int recordedVersion = int.Parse(Recorded.Split(':')[0], CultureInfo.InvariantCulture);

        // The version to suggest: the next one, unless it has already been bumped for this change.
        int next = recordedVersion == PlayerView.ProtocolVersion ? PlayerView.ProtocolVersion + 1 : PlayerView.ProtocolVersion;

        Assert.True(
            string.Equals(Recorded, actual, StringComparison.Ordinal),
            $"""
             The messages no longer have the shape protocol {recordedVersion} recorded.

             If that is deliberate, set PlayerView.ProtocolVersion to {next} and Recorded to "{next}:{digest}".
             A client of an older build cannot read the new shape, and only the version tells it so.
             """);
    }

    /// <summary>Every message type, its union members and its keyed members, in an order that does not depend on reflection.</summary>
    private static string Shape()
    {
        var text = new StringBuilder();

        IEnumerable<Type> messages = typeof(PlayerView).Assembly
            .GetTypes()
            .Where(type =>
                type.GetCustomAttribute<MessagePackObjectAttribute>() is not null
                || type.GetCustomAttributes<UnionAttribute>().Any())
            .OrderBy(type => type.FullName, StringComparer.Ordinal);

        foreach (Type message in messages)
        {
            text.Append(message.FullName).Append('{');

            foreach (UnionAttribute union in message.GetCustomAttributes<UnionAttribute>().OrderBy(u => u.Key))
            {
                text.Append(union.Key.ToString(CultureInfo.InvariantCulture))
                    .Append('=')
                    .Append(union.SubType.Name)
                    .Append(';');
            }

            IEnumerable<(int Key, string Line)> members = message
                .GetMembers(BindingFlags.Public | BindingFlags.Instance)
                .Select(member => (Member: member, Key: member.GetCustomAttribute<KeyAttribute>()))
                .Where(pair => pair.Key?.IntKey is not null)
                .Select(pair => (pair.Key!.IntKey!.Value, $"{pair.Key.IntKey!.Value}:{pair.Member.Name}:{Render(Kind(pair.Member))}"))
                .OrderBy(pair => pair.Item1);

            foreach ((_, string line) in members)
            {
                text.Append(line).Append(';');
            }

            text.Append("}\n");
        }

        // Every enum the messages carry, by member name and value: a member added to one (an action kind, a prompt kind) is
        // as much a change to what the bytes mean as a key, and an older client cannot read it either.
        foreach (Type enumeration in typeof(PlayerView).Assembly.GetTypes().Where(type => type.IsEnum).OrderBy(type => type.FullName, StringComparer.Ordinal))
        {
            text.Append(enumeration.FullName).Append('{');
            foreach (object value in Enum.GetValues(enumeration))
            {
                text.Append(Enum.GetName(enumeration, value)).Append('=')
                    .Append(Convert.ToInt64(value, CultureInfo.InvariantCulture).ToString(CultureInfo.InvariantCulture)).Append(';');
            }

            text.Append("}\n");
        }

        return text.ToString();
    }

    private static Type Kind(MemberInfo member) => member switch
    {
        PropertyInfo property => property.PropertyType,
        FieldInfo field => field.FieldType,
        _ => typeof(void),
    };

    /// <summary>A name for a type that changes when the bytes would: <c>List&lt;ViewHandle&gt;</c>, not <c>List`1</c>.</summary>
    private static string Render(Type type) =>
        type.IsGenericType
            ? $"{type.Name[..type.Name.IndexOf('`', StringComparison.Ordinal)]}<" +
              $"{string.Join(",", type.GetGenericArguments().Select(Render))}>"
            : type.Name;

    private static string Digest(string shape) =>
        System.Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(shape)))[..16];
}
