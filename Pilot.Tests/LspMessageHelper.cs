using System.Text;
using System.Text.Json.Nodes;

namespace Pilot.Tests;

public static class LspMessageHelper
{
    public static string BuildLspMessage(string json)
    {
        int length = Encoding.UTF8.GetByteCount(json);
        return $"Content-Length: {length}\r\n\r\n{json}";
    }
}
