// Synthetic desktop trial only. A real mouse click activates the owned review
// window; a separate helper cannot assume SetForegroundWindow permission.
using System;
using System.Diagnostics;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Text;
class OwnedReviewClick {
 [StructLayout(LayoutKind.Sequential)] struct Point { public int x,y; }
 [StructLayout(LayoutKind.Sequential)] struct Rect { public int left,top,right,bottom; }
 [StructLayout(LayoutKind.Sequential)] struct Mouse { public int dx,dy; public uint data,flags,time; public IntPtr extra; }
 [StructLayout(LayoutKind.Explicit,Size=40)] struct Input { [FieldOffset(0)]public uint type; [FieldOffset(8)]public Mouse mouse; }
 [DllImport("user32.dll")] static extern bool SetProcessDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32.dll")] static extern IntPtr GetAncestor(IntPtr h,uint flags);
 [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(Point p);
 [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr h,out Rect rect);
 [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr h,ref Point p);
 [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] static extern bool IsIconic(IntPtr h);
 [DllImport("user32.dll")] static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] static extern bool GetCursorPos(out Point point);
 [DllImport("user32.dll")] static extern uint SendInput(uint n,Input[] data,int size);
 [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint access,bool inherit,int pid);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern bool QueryFullProcessImageName(IntPtr p,uint flags,StringBuilder s,ref uint n);
 [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr p);
 static string Image(int pid) { var h=OpenProcess(0x1000,false,pid);if(h==IntPtr.Zero)throw new Exception("process unavailable");try{var s=new StringBuilder(32768);uint n=(uint)s.Capacity;if(!QueryFullProcessImageName(h,0,s,ref n))throw new Exception("image unavailable");return s.ToString();}finally{CloseHandle(h);} }
 static uint Owner(IntPtr h) {uint pid;GetWindowThreadProcessId(h,out pid);return pid;}
 static double Number(string s){double v=double.Parse(s,CultureInfo.InvariantCulture);if(double.IsNaN(v)||double.IsInfinity(v))throw new Exception("invalid coordinate");return v;}
 static int Main(string[] args) {try{
  if(Environment.GetEnvironmentVariable("GRAMMAR_HOST_TRIAL")!="20261011-owned"||args.Length!=9)throw new Exception("trial not admitted");
  int app=int.Parse(args[0]),target=int.Parse(args[1]);
  if(!string.Equals(Image(app),System.IO.Path.GetFullPath(args[2]),StringComparison.OrdinalIgnoreCase)||!string.Equals(Image(target),System.IO.Path.GetFullPath(args[3]),StringComparison.OrdinalIgnoreCase))throw new Exception("owned image mismatch");
  SetProcessDpiAwarenessContext(new IntPtr(-4));
  var window=Process.GetProcessById(app).MainWindowHandle;
  if(window==IntPtr.Zero||Owner(window)!=app||!IsWindowVisible(window)||IsIconic(window))throw new Exception("review window unavailable");
  double scale=Number(args[8]);if(scale<=0||scale>8)throw new Exception("invalid scale");
  double x=Number(args[4])*scale,y=Number(args[5])*scale,w=Number(args[6])*scale,h=Number(args[7])*scale;
  Rect rect;if(!GetClientRect(window,out rect)||Math.Abs(rect.right-w)>2||Math.Abs(rect.bottom-h)>2||x<0||y<0||x>=w||y>=h)throw new Exception("viewport mismatch");
  var point=new Point{x=(int)Math.Round(x),y=(int)Math.Round(y)};
  if(!ClientToScreen(window,ref point))throw new Exception("screen mapping failed");
  uint foreground=Owner(GetForegroundWindow());
  if((foreground!=app&&foreground!=target)||GetAncestor(WindowFromPoint(point),2)!=window)throw new Exception("click would target an unowned or obscured window");
  if(!SetCursorPos(point.x,point.y))throw new Exception("cursor unavailable");
  Point actual;if(!GetCursorPos(out actual)||actual.x!=point.x||actual.y!=point.y)throw new Exception("requested click lies outside the accessible desktop");
  // Recheck immediately before input; never click through an unrelated window.
  uint current=Owner(GetForegroundWindow());
  if((current!=app&&current!=target)||GetAncestor(WindowFromPoint(point),2)!=window)throw new Exception("click target changed");
  var events=new Input[2];events[0].mouse.flags=2;events[1].mouse.flags=4;
  uint sent=SendInput(2,events,Marshal.SizeOf(typeof(Input)));
  if(sent!=2){if(sent==1)SendInput(1,new[]{events[1]},Marshal.SizeOf(typeof(Input)));throw new Exception("incomplete mouse input");}
  Console.WriteLine("owned_mouse_click sent=2 foregroundBefore="+foreground+" x="+point.x+" y="+point.y);return 0;
 }catch(Exception e){Console.Error.WriteLine(e.Message);return 1;} }
}
