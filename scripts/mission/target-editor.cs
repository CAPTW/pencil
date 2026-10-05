using System;
using System.Text;
using System.Runtime.InteropServices;
public static class OwnedTarget {
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc proc,IntPtr data);
 public delegate bool EnumProc(IntPtr window,IntPtr data);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowExW(IntPtr top,IntPtr after,string name,string title);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr GetPropW(IntPtr hwnd,string name);
 [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd,uint msg,IntPtr wp,IntPtr lp);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr SendMessageTimeoutW(IntPtr hwnd,uint msg,IntPtr wp,IntPtr lp,uint flags,uint timeout,out IntPtr result);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageTimeoutW")] static extern IntPtr SendText(IntPtr hwnd,uint msg,IntPtr wp,string lp,uint flags,uint timeout,out IntPtr result);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageTimeoutW")] static extern IntPtr ReadText(IntPtr hwnd,uint msg,IntPtr wp,StringBuilder lp,uint flags,uint timeout,out IntPtr result);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hwnd);
 [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr hwnd);
 [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a,uint b,bool on);
 [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
 [DllImport("user32.dll")] static extern uint SendInput(uint count,INPUT[] data,int size);
 [StructLayout(LayoutKind.Sequential)] struct KEYBDINPUT { public ushort vk,scan; public uint flags,time; public IntPtr extra; }
 [StructLayout(LayoutKind.Explicit,Size=40)] struct INPUT { [FieldOffset(0)] public uint type; [FieldOffset(8)] public KEYBDINPUT key; }
 [DllImport("user32.dll")] static extern bool OpenClipboard(IntPtr owner);
 [DllImport("user32.dll")] static extern bool CloseClipboard();
 [DllImport("user32.dll")] static extern IntPtr GetClipboardData(uint format);
 [DllImport("kernel32.dll")] static extern IntPtr GlobalLock(IntPtr handle);
 [DllImport("kernel32.dll")] static extern bool GlobalUnlock(IntPtr handle);
 [DllImport("kernel32.dll")] static extern UIntPtr GlobalSize(IntPtr handle);
 public static string Clipboard() {
  if(!OpenClipboard(IntPtr.Zero))throw new Exception("owned clipboard unavailable");
  try {var h=GetClipboardData(13);if(h==IntPtr.Zero)throw new Exception("clipboard text absent");ulong n=GlobalSize(h).ToUInt64();if(n>131070)throw new Exception("clipboard too large");var ptr=GlobalLock(h);if(ptr==IntPtr.Zero)throw new Exception("clipboard lock failed");try{return Marshal.PtrToStringUni(ptr,(int)n/2).TrimEnd('\0');}finally{GlobalUnlock(h);}}
  finally {CloseClipboard();}
 }
 public static IntPtr Find(uint wanted) {
   IntPtr found=IntPtr.Zero;
   EnumWindows((w,d)=>{uint pid;GetWindowThreadProcessId(w,out pid);if(pid==wanted&&GetPropW(w,"GrammarReady")!=IntPtr.Zero){found=w;return false;}return true;},IntPtr.Zero);
   return found;
 }
 public static void Send(IntPtr field,uint msg,int wp,int lp) {IntPtr result;if(SendMessageTimeoutW(field,msg,new IntPtr(wp),new IntPtr(lp),3,2000,out result)==IntPtr.Zero)throw new Exception("owned send timeout");}
 public static void Set(IntPtr field,string text) {IntPtr result;if(SendText(field,12,IntPtr.Zero,text,3,2000,out result)==IntPtr.Zero)throw new Exception("owned set timeout");}
 public static string Read(IntPtr field) {IntPtr result;var b=new StringBuilder(4096);if(ReadText(field,13,new IntPtr(b.Capacity),b,3,2000,out result)==IntPtr.Zero)throw new Exception("owned read timeout");return b.ToString();}
 public static bool Focus(IntPtr top,uint app,uint target) {
   uint pid;uint foreground=GetWindowThreadProcessId(GetForegroundWindow(),out pid); uint current=GetCurrentThreadId();
   bool attached=(pid==app||pid==target)&&foreground!=current&&AttachThreadInput(current,foreground,true);
   try{BringWindowToTop(top);SetForegroundWindow(top);}finally{if(attached)AttachThreadInput(current,foreground,false);}
   IntPtr result;
   return GetForegroundWindow()==top && SendMessageTimeoutW(top,0x8000+43,IntPtr.Zero,IntPtr.Zero,3,2000,out result)!=IntPtr.Zero && result==new IntPtr(1);
 }
 public static uint Hotkey(IntPtr top) {
   if(GetForegroundWindow()!=top)throw new Exception("owned target not foreground");
   ushort[] keys={0x11,0x10,0x47,0x47,0x10,0x11}; var data=new INPUT[6];
   for(int i=0;i<6;i++){data[i].type=1;data[i].key.vk=keys[i];data[i].key.flags=i>=3?2u:0u;}
   return SendInput(6,data,Marshal.SizeOf(typeof(INPUT)));
 }
}
public static class Program {
 public static int Main() {
  if(Environment.GetEnvironmentVariable("USERNAME")!="WDAGUtilityAccount")return 2;
  var identity=System.Security.Principal.WindowsIdentity.GetCurrent();
  if(new System.Security.Principal.WindowsPrincipal(identity).IsInRole(System.Security.Principal.WindowsBuiltInRole.Administrator))return 3;
  var start=new System.Diagnostics.ProcessStartInfo(@"C:\GrammarPayload\target-service-probe.exe","--target 0");
  start.UseShellExecute=false;start.CreateNoWindow=true;start.WindowStyle=System.Diagnostics.ProcessWindowStyle.Hidden;
  var child=System.Diagnostics.Process.Start(start);IntPtr top=IntPtr.Zero;
  try {
   DateTime limit=DateTime.UtcNow.AddSeconds(10);
   do{top=OwnedTarget.Find((uint)child.Id);if(top!=IntPtr.Zero)break;System.Threading.Thread.Sleep(20);}while(DateTime.UtcNow<limit&&!child.HasExited);
   if(top==IntPtr.Zero)throw new Exception("Owned target not ready");
   IntPtr field=OwnedTarget.FindWindowExW(top,IntPtr.Zero,"RichEdit50W",null);
   if(field==IntPtr.Zero)throw new Exception("Owned field absent");
   Console.WriteLine("READY "+child.Id);string line;
   while((line=Console.ReadLine())!=null){
    try{
     string[] part=line.Split(' ');uint pid;OwnedTarget.GetWindowThreadProcessId(field,out pid);
     if(child.HasExited||pid!=child.Id)throw new Exception("Owned field identity changed");
     switch(part[0]){
      case "SET":OwnedTarget.Set(field,Encoding.UTF8.GetString(Convert.FromBase64String(part[1])));Console.WriteLine("OK");break;
      case "SELECT":OwnedTarget.Send(field,177,int.Parse(part[1]),int.Parse(part[2]));Console.WriteLine("OK");break;
      case "READ":Console.WriteLine("TEXT "+Convert.ToBase64String(Encoding.UTF8.GetBytes(OwnedTarget.Read(field))));break;
      case "MODULES":
       int count=0;child.Refresh();foreach(System.Diagnostics.ProcessModule module in child.Modules){if(string.Equals(module.ModuleName,"target-apply.dll",StringComparison.OrdinalIgnoreCase))count++;}
       Console.WriteLine("MODULES "+count);break;
      case "CLIPBOARD":Console.WriteLine("TEXT "+Convert.ToBase64String(Encoding.UTF8.GetBytes(OwnedTarget.Clipboard())));break;
      case "PASSWORD":OwnedTarget.Send(field,204,int.Parse(part[1]),0);Console.WriteLine("OK");break;
      case "FOCUS":Console.WriteLine("FOCUS "+OwnedTarget.Focus(top,uint.Parse(Environment.GetEnvironmentVariable("GRAMMAR_EXPECTED_APP_PID")),(uint)child.Id));break;
      case "HOTKEY":Console.WriteLine("SENT "+OwnedTarget.Hotkey(top));break;
      case "EXIT":OwnedTarget.PostMessageW(top,16,IntPtr.Zero,IntPtr.Zero);if(!child.WaitForExit(10000))throw new Exception("Owned target exit pending");Console.WriteLine("EXIT "+child.ExitCode);return child.ExitCode;
      default:throw new Exception("Unknown command");
     }
    }catch(Exception e){Console.WriteLine("ERR "+e.Message);}
   }
   return 0;
  }catch(Exception e){Console.Error.WriteLine(e);return 1;}
  finally{
   if(!child.HasExited){OwnedTarget.PostMessageW(top,16,IntPtr.Zero,IntPtr.Zero);if(!child.WaitForExit(10000)){child.Kill();Console.Error.WriteLine("Owned fixture forced cleanup");}}
   child.Dispose();
  }
 }
}
