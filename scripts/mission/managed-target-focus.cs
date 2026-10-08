using System;
using System.IO;
using System.Diagnostics;
using System.Windows.Automation;
using System.Runtime.InteropServices;
class ManagedTargetFocus {
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindowExW(IntPtr parent,IntPtr after,string name,string title);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window,out uint processId);
 static int Main(string[] args) {
  try {
   if(Environment.UserName!="WDAGUtilityAccount" || Environment.MachineName==File.ReadAllText(@"C:\GrammarPayload\host-machine.txt").Trim())throw new Exception("Guest only");
   var target=Process.GetProcessById(int.Parse(args[0]));
   if(!string.Equals(target.MainModule.FileName,@"C:\GrammarPayload\target-service-probe.exe",StringComparison.OrdinalIgnoreCase))throw new Exception("Unexpected target image");
   var element=AutomationElement.FromHandle(target.MainWindowHandle);
   if(element.Current.ProcessId!=target.Id)throw new Exception("Target changed");
   var handle=FindWindowExW(target.MainWindowHandle,IntPtr.Zero,"RichEdit50W",null);
   if(handle==IntPtr.Zero)throw new Exception("Owned native field absent");
   var field=AutomationElement.FromHandle(handle);
   uint owner;GetWindowThreadProcessId(handle,out owner);
   if(field==null || owner!=(uint)target.Id)throw new Exception("Owned field absent");
   field.SetFocus();return 0;
  }catch(Exception e){Console.Error.WriteLine(e);return 1;}
 }
}
