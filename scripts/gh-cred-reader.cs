using System;
using System.Runtime.InteropServices;

public class CredReader
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct CREDENTIAL
    {
        public uint Flags;
        public uint Type;
        public string TargetName;
        public string Comment;
        public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        public uint CredentialBlobSize;
        public IntPtr CredentialBlob;
        public uint Persist;
        public uint AttributeCount;
        public IntPtr Attributes;
        public string TargetAlias;
        public string UserName;
    }

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool CredRead(string target, uint type, uint flags, out IntPtr cred);

    [DllImport("advapi32.dll")]
    public static extern void CredFree(IntPtr cred);

    /// <summary>读取指定 target 的通用凭据；不经过 GCM，绝不弹窗。</summary>
    public static string Read(string target, out string user)
    {
        user = null;
        IntPtr p;
        if (!CredRead(target, 1, 0, out p)) return null;
        try
        {
            CREDENTIAL c = (CREDENTIAL)Marshal.PtrToStructure(p, typeof(CREDENTIAL));
            user = c.UserName;
            byte[] b = new byte[c.CredentialBlobSize];
            Marshal.Copy(c.CredentialBlob, b, 0, (int)c.CredentialBlobSize);
            string u8 = System.Text.Encoding.UTF8.GetString(b).TrimEnd('\0');
            string u16 = System.Text.Encoding.Unicode.GetString(b).TrimEnd('\0');
            return u8.StartsWith("gh") ? u8 : u16;
        }
        finally
        {
            CredFree(p);
        }
    }
}
