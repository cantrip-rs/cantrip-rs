using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;

namespace Cantrip.Server.InPerson;

/// <summary>One of this computer's addresses the table's devices might reach it at, and the network it is on.</summary>
public sealed record LanAddress(IPAddress Address, string Name)
{
    /// <summary>
    /// This computer's addresses on a local network, best first: interfaces that are up, IPv4, private (RFC 1918) and not
    /// link-local (169.254), those with a gateway - the network the phones and tablets are likely on - before those
    /// without, and virtual adapters (Hyper-V, WSL, Docker, VPNs) last, since no phone is on those.
    /// </summary>
    public static IReadOnlyList<LanAddress> Candidates()
    {
        List<(LanAddress Address, int Rank)> found = [];
        NetworkInterface[] interfaces;
        try
        {
            interfaces = NetworkInterface.GetAllNetworkInterfaces();
        }
        catch (NetworkInformationException)
        {
            return [];
        }

        foreach (NetworkInterface each in interfaces)
        {
            if (each.OperationalStatus != OperationalStatus.Up || each.NetworkInterfaceType == NetworkInterfaceType.Loopback) continue;
            IPInterfaceProperties properties = each.GetIPProperties();
            bool gateway = properties.GatewayAddresses.Any(gate => gate.Address.AddressFamily == AddressFamily.InterNetwork && !gate.Address.Equals(IPAddress.Any));
            foreach (UnicastIPAddressInformation unicast in properties.UnicastAddresses)
            {
                if (!IsPrivate(unicast.Address)) continue;
                int rank = (Virtual(each) ? 2 : 0) + (gateway ? 0 : 1);
                found.Add((new LanAddress(unicast.Address, each.Name), rank));
            }
        }

        return [.. found.OrderBy(each => each.Rank).Select(each => each.Address).DistinctBy(each => each.Address)];
    }

    /// <summary>A private IPv4 address (10/8, 172.16/12, 192.168/16): one a local network hands out, never link-local.</summary>
    public static bool IsPrivate(IPAddress address)
    {
        ArgumentNullException.ThrowIfNull(address);
        if (address.AddressFamily != AddressFamily.InterNetwork) return false;
        byte[] bytes = address.GetAddressBytes();
        return bytes[0] == 10
            || (bytes[0] == 172 && bytes[1] >= 16 && bytes[1] <= 31)
            || (bytes[0] == 192 && bytes[1] == 168);
    }

    /// <summary>An adapter no phone is on: a virtual switch, a container network, a VPN's tunnel.</summary>
    private static bool Virtual(NetworkInterface each)
    {
        string said = $"{each.Name} {each.Description}";
        return each.NetworkInterfaceType is NetworkInterfaceType.Tunnel or NetworkInterfaceType.Ppp
            || new[] { "vEthernet", "WSL", "Hyper-V", "Docker", "VirtualBox", "VMware", "VPN", "TAP", "Tailscale", "ZeroTier", "WireGuard" }
                .Any(word => said.Contains(word, StringComparison.OrdinalIgnoreCase));
    }
}
