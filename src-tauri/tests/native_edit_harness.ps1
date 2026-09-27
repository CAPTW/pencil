# Task-owned synthetic Windows editor for Grammar qualification tests.
# Real Edit controls: 0 multiline, 1 password, 2 read-only, 3 ANSI,
# 4 WinForms TextBox (superclass), 5 multiline in a second top-level window.
# Each counts text-bearing and state-changing messages sent from other threads,
# and change notifications raised while such a message is processed. The
# application itself can edit its text inside its own message handling or on
# a timer (0x8105/0x8106), so Apply can be tested against programmatic edits.
# Drivers use either window messages 0x8101-0x8106 (Rust tests) or one-line
# stdin commands. It never reads user documents, the network, or credentials.
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
    // Kinds: 0 WM_GETTEXT, 1 WM_GETTEXTLENGTH, 2 EM_GETSEL, 3 EM_REPLACESEL,
    // 4 EM_SETREADONLY lock, 5 unlock, 6 WM_SETTEXT, 7 EM_UNDO, 8 EM_SETSEL,
    // 9 EM_SETMODIFY (all sent from another thread), 10 EN_CHANGE raised while
    // another thread's message was processed, 11 edits made by the application,
    // 12 every EN_CHANGE from any source (posted messages and input included).
    public static readonly int[,] Values = new int[8, 16];
    // Nesting depth of cross-thread messages being processed on the UI thread.
    public static int CrossThreadDepth;
    [DllImport("user32.dll")]
    private static extern uint InSendMessageEx(IntPtr reserved);

    public static bool FromOtherThread()
    {
        // A same-thread SendMessage nested in a cross-thread one still reports
        // the outer message, so the application's own edits are excluded here.
        return !AppChange.Active && (InSendMessageEx(IntPtr.Zero) & 1u) != 0;
    }

    public static void Record(int index, int message, IntPtr wParam)
    {
        // Count only messages sent synchronously from another thread (the test process).
        if (!FromOtherThread()) return;
        int kind = -1;
        if (message == 0x000D) kind = 0;
        else if (message == 0x000E) kind = 1;
        else if (message == 0x00B0) kind = 2;
        else if (message == 0x00C2) kind = 3;
        else if (message == 0x00CF) kind = wParam == IntPtr.Zero ? 5 : 4;
        else if (message == 0x000C) kind = 6;
        else if (message == 0x00C7) kind = 7;
        else if (message == 0x00B1) kind = 8;
        else if (message == 0x00B9) kind = 9;
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

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr SendMessageW(IntPtr hwnd, int message, IntPtr wParam, string lParam);
    [DllImport("user32.dll")]
    private static extern IntPtr SendMessageW(IntPtr hwnd, int message, IntPtr wParam, IntPtr lParam);

    public static readonly NativeEdit[] Instances = new NativeEdit[8];
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
        Instances[index] = this;
    }

    // An edit made by the application itself on its UI thread (never counted
    // as a cross-thread message).
    public void AppSetText(string text, int selectionStart, int selectionEnd)
    {
        SendMessageW(Handle, 0x000C, IntPtr.Zero, text);
        SendMessageW(Handle, 0x00B1, new IntPtr(selectionStart), new IntPtr(selectionEnd));
        EditCounters.Values[index, 11]++;
    }

    [DllImport("user32.dll")]
    private static extern int GetWindowLongW(IntPtr hwnd, int index);
    [DllImport("user32.dll")]
    private static extern uint InSendMessageEx(IntPtr reserved);

    // One-shot fault injection: the next cross-thread EM_SETSEL received while
    // the control is read-only is followed at once by a selection change, as a
    // user click would race between an Apply's EM_SETSEL and EM_REPLACESEL.
    public static volatile int RaceSelectionIndex = -1;
    // Packed start | end << 16 of the moved selection; default [0, 1).
    public static volatile int RaceSelectionTarget = 1 << 16;

    private IntPtr Hook(IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam)
    {
        EditCounters.Record(index, (int)message, wParam);
        bool crossThread = EditCounters.FromOtherThread();
        // The application edits its text just before another process's
        // WM_SETTEXT is processed (a change right before a restore).
        if (crossThread && message == 0x000C) AppChange.Fire(index, AppChange.BeforeCrossThreadSetText);
        if (crossThread) EditCounters.CrossThreadDepth++;
        IntPtr result;
        try
        {
            result = ansi
                ? CallWindowProcA(previous, hwnd, message, wParam, lParam)
                : CallWindowProcW(previous, hwnd, message, wParam, lParam);
        }
        finally
        {
            if (crossThread) EditCounters.CrossThreadDepth--;
        }
        if (message == 0x00B1 && RaceSelectionIndex == index && crossThread &&
            (GetWindowLongW(hwnd, -16) & 0x0800) != 0)
        {
            RaceSelectionIndex = -1;
            int target = RaceSelectionTarget;
            CallWindowProcW(previous, hwnd, 0x00B1, new IntPtr(target & 0xFFFF), new IntPtr((target >> 16) & 0xFFFF));
        }
        // The application edits its text while it handles messages, right after
        // another process read the selection (a change after verification).
        if (crossThread && message == 0x00B0) AppChange.Fire(index, AppChange.AfterCrossThreadGetSel);
        return result;
    }
}

// Programmatic edits by the application itself, independent of user input.
public static class AppChange
{
    public const int AfterCrossThreadGetSel = 1;
    public const int BeforeCrossThreadSetText = 2;
    public static bool Active;
    private static int armedIndex = -1;
    private static int armedMode;
    private static System.Windows.Forms.Timer fallback;
    private static System.Windows.Forms.Timer periodic;
    private static int periodicIndex = -1;
    private static bool periodicFlip;

    // Arms one application edit. If the matching cross-thread message never
    // arrives (for example because Apply sends none), a one-shot timer makes
    // the same edit, so the application's newer text exists either way.
    public static void Arm(int index, int mode)
    {
        if (fallback == null)
        {
            fallback = new System.Windows.Forms.Timer();
            fallback.Tick += (sender, args) => { fallback.Stop(); Fire(armedIndex, armedMode); };
        }
        fallback.Stop();
        armedIndex = mode == 0 ? -1 : index;
        armedMode = mode;
        if (mode != 0)
        {
            fallback.Interval = 400;
            fallback.Start();
        }
    }

    public static void Fire(int index, int mode)
    {
        if (index < 0 || armedIndex != index || armedMode != mode) return;
        armedIndex = -1;
        if (fallback != null) fallback.Stop();
        if (mode == AfterCrossThreadGetSel) Edit(index, "hello newer", 6, 11);
        else Edit(index, "hello brave world", 0, 0);
    }

    // A timer that toggles one word, or stops with an interval of 0.
    public static void Periodic(int index, int intervalMs)
    {
        if (periodic == null)
        {
            periodic = new System.Windows.Forms.Timer();
            periodic.Tick += (sender, args) =>
            {
                periodicFlip = !periodicFlip;
                Edit(periodicIndex, periodicFlip ? "hello newer" : "hello world", 6, 11);
            };
        }
        periodic.Stop();
        periodicIndex = index;
        periodicFlip = false;
        if (intervalMs > 0)
        {
            periodic.Interval = intervalMs;
            periodic.Start();
        }
    }

    private static void Edit(int index, string text, int start, int end)
    {
        Active = true;
        try { NativeEdit.Instances[index].AppSetText(text, start, end); }
        finally { Active = false; }
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
        if (message.Msg == 0x8104)
        {
            NativeEdit.RaceSelectionIndex = message.WParam.ToInt32();
            int target = message.LParam.ToInt32();
            NativeEdit.RaceSelectionTarget = target == 0 ? 1 << 16 : target;
            message.Result = new IntPtr(1);
            return;
        }
        if (message.Msg == 0x8105)
        {
            AppChange.Arm(message.WParam.ToInt32(), message.LParam.ToInt32());
            message.Result = new IntPtr(1);
            return;
        }
        if (message.Msg == 0x8106)
        {
            AppChange.Periodic(message.WParam.ToInt32(), message.LParam.ToInt32());
            message.Result = new IntPtr(1);
            return;
        }
        if (message.Msg == 0x0111)
        {
            long command = message.WParam.ToInt64();
            int id = (int)(command & 0xFFFF);
            int code = (int)((command >> 16) & 0xFFFF);
            // EN_CHANGE while another process's message is being processed: the
            // application observed a text change caused by that process.
            if (code == 0x0300 && id >= 100 && id < 108 && EditCounters.CrossThreadDepth > 0 && !AppChange.Active)
                EditCounters.Values[id - 100, 10]++;
            if (code == 0x0300 && id >= 100 && id < 108)
                EditCounters.Values[id - 100, 12]++;
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
            case "COUNTS":
                int counted = int.Parse(part[1]);
                var values = new List<string>();
                for (int kind = 0; kind < 13; kind++) values.Add(EditCounters.Values[counted, kind].ToString());
                return "COUNTS " + String.Join(" ", values);
            case "RESET":
                EditCounters.Reset();
                return "OK";
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
