using System.Runtime.InteropServices;
using System.Text;
using SteamDatabase.ValvePak;

namespace VpkInfo;

/*
 * 读取指定的vpk文件，获取其文件中的addoninfo.txt和missions/文件夹下的文本文件的内容
 */
public class Vpk : IDisposable
{
    private readonly PackageEntry? _addoninfoEntry;
    private readonly PackageEntry? _missionEntry;
    private readonly Package _package;

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

    public void Dispose()
    {
        _package.Dispose();
    }

    public string? GetMissionContent()
    {
        if (_missionEntry is null) return null;
        _package.ReadEntry(_missionEntry, out byte[] buffer, false);
        return Encoding.UTF8.GetString(buffer);
    }

    public string? GetAddonInfoContent()
    {
        if (_addoninfoEntry is null) return null;
        _package.ReadEntry(_addoninfoEntry, out byte[] buffer, false);
        return Encoding.UTF8.GetString(buffer);
    }
}

public static class NativeExports
{
    private static string _lastErrMsg = string.Empty;

    [UnmanagedCallersOnly(EntryPoint = "GetLastErrorMessage")]
    public static unsafe byte* GetLastErrorMessage() => StringToNativeMemUTF8(_lastErrMsg);

    [UnmanagedCallersOnly(EntryPoint = "FreeString")]
    public static unsafe void FreeString(byte* strPtr) => NativeMemory.Free(strPtr);

    // 返回值为-1 有错误
    [UnmanagedCallersOnly(EntryPoint = "CreateVpk")]
    public static unsafe int CreateVpk(byte* pathPtr, void** handle)
    {
        *handle = (void*) 0;
        try
        {
            if (pathPtr == (byte*) 0)
            {
                throw new Exception("path ptr is not zero");
            }

            string path = Marshal.PtrToStringUTF8((nint) pathPtr)!;
            var vpk = new Vpk(path);
            *handle = GCHandle.ToIntPtr(GCHandle.Alloc(vpk)).ToPointer();
            return 0;
        }
        catch (Exception e)
        {
            _lastErrMsg = e.Message;
            return -1;
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "DestroyVpk")]
    public static unsafe void DestroyVpk(void* handle)
    {
        if (handle == (void*) 0) return;
        var gch = GCHandle.FromIntPtr((nint) handle);
        ((Vpk) gch.Target!).Dispose();
        gch.Free();
    }

    // 返回值为-1 有错误  *contentPtr 为0时  文件不存在
    [UnmanagedCallersOnly(EntryPoint = "GetMissionContent")]
    public static unsafe int GetMissionContent(void* handle, byte** contentPtr)
    {
        *contentPtr = (byte*) 0;
        try
        {
            var vpk = HandleToVpk(handle);
            string? content = vpk.GetMissionContent();
            *contentPtr = StringToNativeMemUTF8(content);
            return 0;
        }
        catch (Exception e)
        {
            _lastErrMsg = e.Message;
            return -1;
        }
    }

    // 返回值为-1 有错误  *contentPtr 为0时 文件不存在
    [UnmanagedCallersOnly(EntryPoint = "GetAddonInfoContent")]
    public static unsafe int GetAddonInfoContent(void* handle, byte** contentPtr)
    {
        *contentPtr = (byte*) 0;
        try
        {
            var vpk = HandleToVpk(handle);
            string? content = vpk.GetAddonInfoContent();
            *contentPtr = StringToNativeMemUTF8(content);
            return 0;
        }
        catch (Exception e)
        {
            _lastErrMsg = e.Message;
            return -1;
        }
    }

    // ReSharper disable once InconsistentNaming
    private static unsafe byte* StringToNativeMemUTF8(string? s)
    {
        if (s is null) return (byte*) 0;
        int maxByteCount = Encoding.UTF8.GetMaxByteCount(s.Length);
        byte* pointer = (byte*) NativeMemory.Alloc((nuint) checked(maxByteCount + 1));
        int bytes = Encoding.UTF8.GetBytes(s.AsSpan(), new Span<byte>(pointer, maxByteCount));
        pointer[bytes] = 0;
        return pointer;
    }

    private static unsafe Vpk HandleToVpk(void* handle)
    {
        if (handle == (void*) 0)
        {
            throw new Exception("handle is not zero");
        }

        return (Vpk) GCHandle.FromIntPtr((nint) handle).Target!;
    }
}
