using System.Runtime.InteropServices.JavaScript;

namespace Cantrip.Client.Web.InPerson;

/// <summary>What a device at an in-person table is (docs/in-person.md): nothing, for every other page.</summary>
public enum InPersonRole
{
    /// <summary>Not an in-person device: the remote client, the deck editor, a fixture.</summary>
    None,

    /// <summary>One player's own device, for their hidden information and every prompt (<c>?inperson=phone</c>).</summary>
    Phone,

    /// <summary>A public board (<c>?inperson=tablet</c>).</summary>
    Tablet,
}

/// <summary>
/// Whether this page is an in-person device, and which, read from the page's address:
/// <c>?inperson=phone</c>, or <c>?inperson=tablet&amp;side=1#device=TOKEN</c>. Every other page reads
/// <see cref="InPersonRole.None"/> and is unchanged by it. <see cref="ClientConfig"/> still reads the rest of the query
/// (the server, a phone's ticket).
/// </summary>
public static class InPersonConfig
{
    static InPersonConfig()
    {
        JSObject location = JSHost.GlobalThis.GetPropertyAsJSObject("location")!;
        Dictionary<string, string> query = Parse(location.GetPropertyAsString("search") ?? string.Empty);
        Dictionary<string, string> fragment = Parse(location.GetPropertyAsString("hash") ?? string.Empty);

        Role = query.GetValueOrDefault("inperson")?.ToLowerInvariant() switch
        {
            "phone" => InPersonRole.Phone,
            "tablet" => InPersonRole.Tablet,
            _ => InPersonRole.None,
        };
        Side = int.TryParse(query.GetValueOrDefault("side"), out int side) && side > 0 ? side : 0;
        // InPerson.js may already have taken it out of the address and kept it aside, as Ticket.js does a ticket.
        Device = (JSHost.GlobalThis.GetPropertyAsString("cantripDevice") ?? fragment.GetValueOrDefault("device")) is { Length: > 0 } device
            ? device
            : null;
    }

    /// <summary>What this page is at an in-person table, or <see cref="InPersonRole.None"/>.</summary>
    public static InPersonRole Role { get; }

    /// <summary>
    /// The board a tablet shows (<c>?side=N</c>): zero for the shared board of both halves, or the seat whose half it is.
    /// </summary>
    public static int Side { get; }

    /// <summary>
    /// A tablet's device token (<c>#device=...</c>), a bearer credential the relay minted: sent in its hello, shown
    /// nowhere. Null for a phone and every other page.
    /// </summary>
    public static string? Device { get; }

    private static Dictionary<string, string> Parse(string part)
    {
        Dictionary<string, string> values = new(StringComparer.OrdinalIgnoreCase);

        foreach (string pair in part.TrimStart('?', '#').Split('&', StringSplitOptions.RemoveEmptyEntries))
        {
            int equals = pair.IndexOf('=');
            if (equals > 0)
            {
                values[Uri.UnescapeDataString(pair[..equals])] = Uri.UnescapeDataString(pair[(equals + 1)..].Replace('+', ' '));
            }
            else if (equals < 0)
            {
                values[Uri.UnescapeDataString(pair)] = string.Empty;
            }
        }

        return values;
    }
}
