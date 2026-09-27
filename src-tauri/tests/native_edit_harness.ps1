# Task-owned synthetic Windows editor for Grammar qualification tests.
# Real Edit controls: 0 multiline, 1 password, 2 read-only, 3 ANSI,
# 4 WinForms TextBox (superclass), 5 multiline in a second top-level window.
# Each counts text-bearing messages sent from other threads. Drivers use either
# window messages 0x8101-0x8103 (Rust tests) or one-line stdin commands.
# It never reads user documents, the network, or credentials.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms.dll,System.Drawing.dll -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Windows.Forms;

public delegate IntPtr EditWndProc(IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam);

public static class EditCounters
{
    public static readonly int[,] Values = new int[8, 16];
    [DllImport("user32.dll")]
    private static extern uint InSendMessageEx(IntPtr reserved);

    public static void Record(int index, int message, IntPtr wParam)
    {
        // Count only messages sent synchronously from another thread (the test process).
        if ((InSendMessageEx(IntPtr.Zero) & 1u) == 0) return;
        int kind = -1;
        if (message == 0x000D) kind = 0;
        else if (message == 0x000E) kind = 1;
        else if (message == 0x00B0) kind = 2;
        else if (message == 0x00C2) kind = 3;
        else if (message == 0x00CF) kind = wParam == IntPtr.Zero ? 5 : 4;
        else if (message == 0x000C) kind = 6;
        else if (message == 0x00C7) kind = 7;
        if (kind >= 0) Values[index, kind]++;
    }

    public static void Reset()
    {
        Array.Clear(Values, 0, Values.Length);
    }
}

public sealed class NativeEdit
{
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowExW(int exStyle, string className, string windowName, int style, int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll", CharSet = CharSet.Ansi, SetLastError = true)]
    private static extern IntPtr CreateWindowExA(int exStyle, string className, string windowName, int style, int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll")]
    private static extern IntPtr SetWindowLongPtrW(IntPtr hwnd, int index, IntPtr value);
    [DllImport("user32.dll")]
    private static extern IntPtr SetWindowLongPtrA(IntPtr hwnd, int index, IntPtr value);
    [DllImport("user32.dll")]
    private static extern IntPtr CallWindowProcW(IntPtr previous, IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")]
    private static extern IntPtr CallWindowProcA(IntPtr previous, IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam);

    public readonly IntPtr Handle;
    private readonly int index;
    private readonly bool ansi;
    private readonly IntPtr previous;
    private readonly EditWndProc hook;

    public NativeEdit(IntPtr parent, int index, int style, bool ansi, int x, int y, int width, int height)
    {
        this.index = index;
        this.ansi = ansi;
        int full = 0x40000000 | 0x10000000 | 0x00800000 | 0x00010000 | style;
        Handle = ansi
            ? CreateWindowExA(0, "EDIT", "", full, x, y, width, height, parent, new IntPtr(100 + index), IntPtr.Zero, IntPtr.Zero)
            : CreateWindowExW(0, "EDIT", "", full, x, y, width, height, parent, new IntPtr(100 + index), IntPtr.Zero, IntPtr.Zero);
        if (Handle == IntPtr.Zero) throw new InvalidOperationException("synthetic edit creation failed");
        hook = Hook;
        IntPtr pointer = Marshal.GetFunctionPointerForDelegate(hook);
        previous = ansi ? SetWindowLongPtrA(Handle, -4, pointer) : SetWindowLongPtrW(Handle, -4, pointer);
    }

    private IntPtr Hook(IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam)
    {
        EditCounters.Record(index, (int)message, wParam);
        return ansi
            ? CallWindowProcA(previous, hwnd, message, wParam, lParam)
            : CallWindowProcW(previous, hwnd, message, wParam, lParam);
    }
}

public sealed class CountingTextBox : TextBox
{
    private readonly int index;

    public CountingTextBox(int index)
    {
        this.index = index;
    }

    protected override void WndProc(ref Message message)
    {
        EditCounters.Record(index, message.Msg, message.WParam);
        base.WndProc(ref message);
    }
}

public sealed class NativeEditForm : Form
{
    [DllImport("user32.dll")]
    private static extern IntPtr SetFocus(IntPtr hwnd);

    public readonly List<IntPtr> Edits = new List<IntPtr>();
    private readonly List<NativeEdit> keep = new List<NativeEdit>();
    private readonly int firstIndex;
    private readonly bool full;
    private int desired;

    public NativeEditForm(string title, int left, int firstIndex, bool full)
    {
        this.firstIndex = firstIndex;
        this.full = full;
        desired = firstIndex;
        Text = title;
        StartPosition = FormStartPosition.Manual;
        Location = new Point(left, 40);
        Size = full ? new Size(520, 400) : new Size(420, 220);
    }

    protected override void OnHandleCreated(EventArgs args)
    {
        base.OnHandleCreated(args);
        Add(new NativeEdit(Handle, firstIndex, 0x0004 | 0x0040 | 0x1000 | 0x00200000, false, 8, 8, 480, 110));
        if (!full) return;
        Add(new NativeEdit(Handle, 1, 0x0020 | 0x0080, false, 8, 126, 480, 24));
        Add(new NativeEdit(Handle, 2, 0x0004 | 0x0800, false, 8, 156, 480, 60));
        Add(new NativeEdit(Handle, 3, 0x0080, true, 8, 222, 480, 24));
        var box = new CountingTextBox(4) { Left = 8, Top = 252, Width = 480 };
        Controls.Add(box);
        Edits.Add(box.Handle);
    }

    private void Add(NativeEdit edit)
    {
        keep.Add(edit);
        Edits.Add(edit.Handle);
    }

    private IntPtr EditFor(int index)
    {
        return Edits[index - firstIndex];
    }

    public void FocusEdit(int index)
    {
        desired = index;
        SetFocus(EditFor(desired));
    }

    protected override void OnActivated(EventArgs args)
    {
        base.OnActivated(args);
        SetFocus(EditFor(desired));
    }

    protected override void WndProc(ref Message message)
    {
        if (message.Msg == 0x8101)
        {
            int packed = message.WParam.ToInt32();
            message.Result = new IntPtr(EditCounters.Values[packed / 16, packed % 16]);
            return;
        }
        if (message.Msg == 0x8102)
        {
            EditCounters.Reset();
            message.Result = new IntPtr(1);
            return;
        }
        if (message.Msg == 0x8103)
        {
            FocusEdit(message.WParam.ToInt32());
            message.Result = new IntPtr(1);
            return;
        }
        base.WndProc(ref message);
    }
}

public static class NativeEditHarness
{
    [DllImport("user32.dll")]
    private static extern IntPtr SendMessageW(IntPtr hwnd, int message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern bool SetWindowTextW(IntPtr hwnd, string text);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowTextLengthW(IntPtr hwnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowTextW(IntPtr hwnd, System.Text.StringBuilder text, int count);
    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern bool BringWindowToTop(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hwnd, IntPtr pid);
    [DllImport("user32.dll")]
    private static extern bool AttachThreadInput(uint attach, uint to, bool on);
    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")]
    private static extern uint SendInput(uint count, INPUT[] inputs, int size);

    [StructLayout(LayoutKind.Sequential)]
    private struct KEYBDINPUT
    {
        public ushort wVk;
        public ushort wScan;
        public uint dwFlags;
        public uint time;
        public IntPtr dwExtraInfo;
    }

    [StructLayout(LayoutKind.Explicit)]
    private struct INPUT
    {
        [FieldOffset(0)] public uint type;
        [FieldOffset(8)] public KEYBDINPUT ki;
        [FieldOffset(8)] private long padding0;
        [FieldOffset(16)] private long padding1;
        [FieldOffset(24)] private long padding2;
        [FieldOffset(32)] private long padding3; // sizeof(INPUT) is 40 on x64
    }

    private static NativeEditForm main;
    private static NativeEditForm other;

    private static IntPtr Edit(int index)
    {
        return index == 5 ? other.Edits[0] : main.Edits[index];
    }

    private static NativeEditForm FormOf(int index)
    {
        return index == 5 ? other : main;
    }

    private static void ForceForeground(IntPtr hwnd)
    {
        uint current = GetCurrentThreadId();
        uint foreground = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        bool attached = foreground != 0 && foreground != current && AttachThreadInput(current, foreground, true);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        if (attached) AttachThreadInput(current, foreground, false);
    }

    private static INPUT Key(ushort key, bool up)
    {
        var input = new INPUT();
        input.type = 1;
        input.ki.wVk = key;
        input.ki.dwFlags = up ? 2u : 0u;
        return input;
    }

    private static string Execute(string line)
    {
        string[] part = line.Split(' ');
        switch (part[0])
        {
            case "SET":
                SetWindowTextW(Edit(int.Parse(part[1])), System.Text.Encoding.UTF8.GetString(Convert.FromBase64String(part.Length > 2 ? part[2] : "")));
                return "OK";
            case "SELECT":
                SendMessageW(Edit(int.Parse(part[1])), 0x00B1, new IntPtr(int.Parse(part[2])), new IntPtr(int.Parse(part[3])));
                return "OK";
            case "FOCUS":
                int index = int.Parse(part[1]);
                FormOf(index).FocusEdit(index);
                ForceForeground(FormOf(index).Handle);
                FormOf(index).FocusEdit(index);
                return "OK";
            case "TEXT":
                IntPtr edit = Edit(int.Parse(part[1]));
                var text = new System.Text.StringBuilder(GetWindowTextLengthW(edit) + 1);
                GetWindowTextW(edit, text, text.Capacity);
                return "TEXT " + Convert.ToBase64String(System.Text.Encoding.UTF8.GetBytes(text.ToString()));
            case "SEL":
                long packed = SendMessageW(Edit(int.Parse(part[1])), 0x00B0, IntPtr.Zero, IntPtr.Zero).ToInt64();
                return "SEL " + (packed & 0xFFFF) + " " + ((packed >> 16) & 0xFFFF);
            case "FOREGROUND":
                IntPtr foreground = GetForegroundWindow();
                return "FOREGROUND " + (foreground == main.Handle ? "main" : foreground == other.Handle ? "other" : "elsewhere");
            case "HOTKEY":
                // The configured default shortcut; the app's global hotkey receives it.
                INPUT[] keys = { Key(0x11, false), Key(0x10, false), Key(0x47, false), Key(0x47, true), Key(0x10, true), Key(0x11, true) };
                return "SENT " + SendInput((uint)keys.Length, keys, Marshal.SizeOf(typeof(INPUT)));
            case "CLIPBOARD":
                return "CLIP " + Convert.ToBase64String(System.Text.Encoding.UTF8.GetBytes(Clipboard.ContainsText() ? Clipboard.GetText() : ""));
            default:
                return "ERR unknown_command";
        }
    }

    private static void CommandLoop()
    {
        string line;
        while ((line = Console.In.ReadLine()) != null)
        {
            string reply;
            try
            {
                reply = (string)main.Invoke(new Func<string>(() => Execute(line)));
            }
            catch (Exception error)
            {
                reply = "ERR " + error.GetType().Name;
            }
            Console.Out.WriteLine(reply);
            Console.Out.Flush();
        }
    }

    public static void Run()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        main = new NativeEditForm("Synthetic Native Edit Target", 40, 0, true);
        other = new NativeEditForm("Synthetic Other Editor", 580, 5, false);
        main.Show();
        other.Show();
        main.Activate();
        Application.DoEvents();
        var parts = new List<string>();
        parts.Add(main.Handle.ToInt64().ToString());
        foreach (IntPtr edit in main.Edits) parts.Add(edit.ToInt64().ToString());
        parts.Add(other.Handle.ToInt64().ToString());
        parts.Add(other.Edits[0].ToInt64().ToString());
        Console.Out.WriteLine("NATIVE_READY:" + String.Join(":", parts));
        Console.Out.Flush();
        // Drivers without window-message access use stdin; EOF just ends the loop.
        new System.Threading.Thread(CommandLoop) { IsBackground = true }.Start();
        Application.Run();
    }
}
'@
[NativeEditHarness]::Run()
