using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using SteamDatabase.ValvePak;

namespace VpkInfo;

/*
 * 读取指定的vpk文件，获取其文件中的addoninfo.txt和missions/文件夹下的文本文件的内容
 */
public class Vpk : IDisposable
{
    private readonly Package _package;
    private readonly PackageEntry? _missionEntry;
    private readonly PackageEntry? _addoninfoEntry;

    public Vpk(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        if (!File.Exists(path))
        {
            throw new FileNotFoundException(path);
        }

        _package = new Package();
        _package.Read(path);

        var entries = _package.Entries;
        if (entries is null)
        {
            throw new Exception("No entries found");
        }

        if (!entries.TryGetValue("txt", out var txtEntries)) return;

        foreach (var entry in txtEntries)
        {
            if (entry.DirectoryName.Equals("missions", StringComparison.OrdinalIgnoreCase))
            {
                _missionEntry = entry;
            }
            else if (entry.FileName.Equals("addoninfo", StringComparison.OrdinalIgnoreCase))
            {
                _addoninfoEntry = entry;
            }
        }
    }

    public byte[]? GetMissionContent()
    {
        if (_missionEntry is null) return null;
        _package.ReadEntry(_missionEntry, out byte[] buf, false);
        return buf;
    }

    public byte[]? GetAddonInfoContent()
    {
        if (_addoninfoEntry is null) return null;
        _package.ReadEntry(_addoninfoEntry, out byte[] buf, false);
        return buf;
    }

    public void Dispose()
    {
        _package.Dispose();
    }
}

public static class NativeExports
{
    [UnmanagedCallersOnly(EntryPoint = "create_vpk", CallConvs = [typeof(CallConvCdecl)])]
    public static nint CreateVpk(nint pathPtr)
    {
        string path = Marshal.PtrToStringUTF8(pathPtr)!;
        var vpk = new Vpk(path);
        GCHandle handle = GCHandle.Alloc(vpk);
        return (nint) handle;
    }

    [UnmanagedCallersOnly(EntryPoint = "destroy_vpk", CallConvs = [typeof(CallConvCdecl)])]
    public static void DestroyVpk(nint handle)
    {
        if (handle == nint.Zero) return;
        var gch = (GCHandle) handle;
        if (gch.Target is IDisposable dis)
        {
            dis.Dispose();
        }

        gch.Free();
    }

    [UnmanagedCallersOnly(EntryPoint = "get_mission_content", CallConvs = [typeof(CallConvCdecl)])]
    public static int GetMissionContent(nint handle, nint buffer, int bufferSize)
    {
        if (handle == nint.Zero) return -1;
        if (buffer == nint.Zero) return -2;
        var vpk = (Vpk) ((GCHandle) handle).Target!;
        byte[]? bytes = vpk.GetMissionContent();
        if (bytes is null) return -3; //不存在该文件时，返回-3
        int toCopy = Math.Min(bytes.Length, bufferSize);
        Marshal.Copy(bytes, 0, buffer, toCopy);
        return toCopy;
    }

    [UnmanagedCallersOnly(EntryPoint = "get_addoninfo_content", CallConvs = [typeof(CallConvCdecl)])]
    public static int GetAddonInfoContent(nint handle, nint buffer, int bufferSize)
    {
        if (handle == nint.Zero) return -1;
        if (buffer == nint.Zero) return -2;
        var vpk = (Vpk) ((GCHandle) handle).Target!;
        byte[]? bytes = vpk.GetAddonInfoContent();
        if (bytes is null) return -3; //不存在该文件时，返回-3
        int toCopy = Math.Min(bytes.Length, bufferSize);
        Marshal.Copy(bytes, 0, buffer, toCopy);
        return toCopy;
    }
}
