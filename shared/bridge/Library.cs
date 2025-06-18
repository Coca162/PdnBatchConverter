using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
using OpenRasterFileType;
using PaintDotNet;

namespace PdnBridge;

public static class Library {
    private static unsafe delegate*<char*, int, byte*> CopyToCString;

    [UnmanagedCallersOnly]
    public static unsafe void SetCopyToCStringFunctionPtr(delegate*<char*, int, byte*> copyToCString) => CopyToCString = copyToCString;

    [StructLayoutAttribute(LayoutKind.Sequential)]
    public struct InternalVersion {
        public int major;
        public int minor;
        public int build;
        public int revision;
    }

    [UnmanagedCallersOnly]
    public static unsafe void SetPdnVersion(InternalVersion pdn_version) {
        var installed_version = new Version(pdn_version.major, pdn_version.minor, pdn_version.build, pdn_version.revision);

        AppDomain.CurrentDomain.AssemblyResolve += new ResolveEventHandler((_, sender) => {
            var loadName = new AssemblyName(sender.Name) {
                Version = installed_version
            };
            return Assembly.Load(loadName);
        });
    }

    [StructLayoutAttribute(LayoutKind.Sequential)]
    public struct IO {
        public nint input;
        public nint output;
    }

    [UnmanagedCallersOnly]
    public static unsafe byte* PdnToOra(IO pair) {
        try {
            Document doc;
            using (var input = new FileStream(new SafeFileHandle(pair.input, true), FileAccess.Read)) {
                doc = Document.FromStream(input);
            }

            using var output = new FileStream(new SafeFileHandle(pair.output, true), FileAccess.ReadWrite);
            typeof(OraFileType).InvokeMember("OnSave", BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.InvokeMethod, null, new OraFileType(), [doc, output, null, null, null]);
        } catch (Exception e) {
            var message = e.ToString();
            fixed (char* ptr = message) {
                return CopyToCString(ptr, message.Length);
            }
        }

        return null;
    }
}
