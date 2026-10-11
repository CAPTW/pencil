// Owned Sandbox fixture only. Click only a visible, exact owned RichEdit field.
using System;
using System.IO;
using System.Diagnostics;
using System.Runtime.InteropServices;
class OwnedClickFocus {
 [StructLayout(LayoutKind.Sequential)] struct Rect { public int left,top,right,bottom; }
 [StructLayout(LayoutKind.Sequential)] struct Point { public int x,y; }
 [StructLayout(LayoutKind.Sequential)] struct Mouse {public int x,y;public uint data,flags,time;public IntPtr extra;}
 [StructLayout(LayoutKind.Explicit,Size=40)] struct Input {[FieldOffset(0)]public uint type;[FieldOffset(8)]public Mouse mouse;}
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]static extern IntPtr FindWindowExW(IntPtr p,IntPtr a,string c,string t);
 [DllImport("user32.dll")]static extern bool BringWindowToTop(IntPtr w);
 [DllImport("user32.dll")]static extern bool GetWindowRect(IntPtr w,out Rect r);
 [DllImport("user32.dll")]static extern bool IsWindowVisible(IntPtr w);
 [DllImport("user32.dll")]static extern IntPtr WindowFromPoint(Point p);
 [DllImport("user32.dll",SetLastError=true)]static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")]static extern uint GetWindowThreadProcessId(IntPtr w,out uint p);
 [DllImport("user32.dll")]static extern uint SendInput(uint n,Input[] input,int size);
 [DllImport("user32.dll")]static extern bool SetProcessDPIAware();
 static int Main(string[] args){try{
  if(Environment.UserName!="WDAGUtilityAccount"||Environment.MachineName==File.ReadAllText(@"C:\GrammarPayload\host-machine.txt").Trim())throw new Exception("guest only");
  SetProcessDPIAware();
  var p=Process.GetProcessById(int.Parse(args[0]));
  if(!string.Equals(p.MainModule.FileName,@"C:\GrammarPayload\target-service-probe.exe",StringComparison.OrdinalIgnoreCase))throw new Exception("target image");
  var top=p.MainWindowHandle;var field=FindWindowExW(top,IntPtr.Zero,"RichEdit50W",null);uint owner;
  GetWindowThreadProcessId(field,out owner);
  if(field==IntPtr.Zero||owner!=(uint)p.Id||!IsWindowVisible(field))throw new Exception("target field");
  BringWindowToTop(top);Rect r;if(!GetWindowRect(field,out r)||r.right<=r.left||r.bottom<=r.top)throw new Exception("target rectangle");
  var point=new Point{x=(r.left+r.right)/2,y=(r.top+r.bottom)/2};
  var hit=WindowFromPoint(point);uint hitOwner;GetWindowThreadProcessId(hit,out hitOwner);
  Console.WriteLine("target="+field+" hit="+hit+" hitPid="+hitOwner+" x="+point.x+" y="+point.y);
  if(hit!=field)throw new Exception("target occluded");
  if(!SetCursorPos(point.x,point.y))throw new Exception("cursor unavailable "+Marshal.GetLastWin32Error());
  if(WindowFromPoint(point)!=field)throw new Exception("target occluded after cursor move");
  var input=new Input[2];input[0].mouse.flags=2;input[1].mouse.flags=4;
  if(SendInput(2,input,Marshal.SizeOf(typeof(Input)))!=2)throw new Exception("click incomplete");
  Console.WriteLine("owned field click sent; caller must verify foreground and restore synthetic selection");return 0;
 }catch(Exception e){Console.Error.WriteLine(e.Message);return 1;}}
}
