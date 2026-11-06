using System.Globalization;
using Cantrip.Protocol;

namespace Cantrip.Client;

/// <summary>Exact bounded numeric input, shared by the dock and its validation tests.</summary>
public static class NumberChoice
{
    public static bool TryValue(ChooseNumberPromptMessage prompt, string? text, out uint value) =>
        uint.TryParse(text, NumberStyles.None, CultureInfo.InvariantCulture, out value)
        && value >= prompt.Least && value <= prompt.Most;

    public static uint? Step(ChooseNumberPromptMessage prompt, string? text, bool up)
    {
        if (!TryValue(prompt, text, out uint value)) return null;
        if (up) return value < prompt.Most ? value + 1 : null;
        return value > prompt.Least ? value - 1 : null;
    }
}
