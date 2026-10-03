using System;
using System.IO;
using System.Text;
using System.Runtime.InteropServices;
using System.ComponentModel;
using System.Security.Principal;
using System.Security.AccessControl;
class MediumLaunch {
 [StructLayout(LayoutKind.Sequential)] struct Label { public IntPtr Sid; public uint Attributes; }
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct Startup { public int cb;public string reserved,desktop,title;public int x,y,cx,cy,cols,rows,fill,flags;public short show,reserved2;public IntPtr reservedPtr,input,output,error; }
 [StructLayout(LayoutKind.Sequential)] struct Info {public IntPtr process,thread;public uint pid,tid;}
 [DllImport("advapi32.dll",SetLastError=true)]static extern bool OpenProcessToken(IntPtr p,uint access,out IntPtr token);
 [DllImport("advapi32.dll",SetLastError=true)]static extern bool CreateRestrictedToken(IntPtr token,uint flags,uint dc,IntPtr ds,uint pc,IntPtr ps,uint sc,IntPtr ss,out IntPtr result);
 [DllImport("advapi32.dll",CharSet=CharSet.Unicode,SetLastError=true)]static extern bool ConvertStringSidToSid(string text,out IntPtr sid);
 [DllImport("advapi32.dll")]static extern uint GetLengthSid(IntPtr sid);
 [DllImport("advapi32.dll",SetLastError=true)]static extern bool SetTokenInformation(IntPtr token,int kind,ref Label label,uint size);
 [DllImport("advapi32.dll",SetLastError=true,EntryPoint="SetTokenInformation")]static extern bool SetPointerInfo(IntPtr token,int kind,ref IntPtr pointer,uint size);
 [DllImport("advapi32.dll",CharSet=CharSet.Unicode,SetLastError=true)]static extern bool CreateProcessAsUser(IntPtr token,string app,StringBuilder args,IntPtr pa,IntPtr ta,bool inherit,uint flags,IntPtr env,string cwd,ref Startup startup,out Info info);
 [DllImport("kernel32.dll")]static extern uint WaitForSingleObject(IntPtr h,uint ms);
 [DllImport("kernel32.dll")]static extern bool GetExitCodeProcess(IntPtr h,out uint code);
 [DllImport("kernel32.dll")]static extern IntPtr GetCurrentProcess();
 [DllImport("kernel32.dll")]static extern bool CloseHandle(IntPtr h);
 [DllImport("kernel32.dll")]static extern IntPtr LocalFree(IntPtr p);
 static void Check(bool value){if(!value)throw new Win32Exception(Marshal.GetLastWin32Error());}
 static int Main(string[] args){
  try{
   if(Environment.UserName!="WDAGUtilityAccount" || Environment.MachineName==File.ReadAllText(@"C:\GrammarPayload\host-machine.txt").Trim())throw new Exception("Guest only");
   if(args.Length!=1 || !args[0].StartsWith(@"C:\GrammarPayload\Guest-App-",StringComparison.OrdinalIgnoreCase) || !args[0].EndsWith(".ps1") || args[0].Contains("..") || args[0].Contains("\""))throw new Exception("Owned guest script required");
   IntPtr original=IntPtr.Zero,restricted=IntPtr.Zero,sid=IntPtr.Zero;
   try{
    Check(OpenProcessToken(GetCurrentProcess(),0x8B,out original));
    Check(CreateRestrictedToken(original,5,0,IntPtr.Zero,0,IntPtr.Zero,0,IntPtr.Zero,out restricted));
    Check(ConvertStringSidToSid("S-1-16-8192",out sid));
    var label=new Label{Sid=sid,Attributes=0x20};
    Check(SetTokenInformation(restricted,25,ref label,(uint)Marshal.SizeOf(typeof(Label))+GetLengthSid(sid)));
    // Restricted children must own and access their own process/thread objects.
    IntPtr userSid=IntPtr.Zero,aclMemory=IntPtr.Zero;
    try {
     string user=WindowsIdentity.GetCurrent().User.Value;
     Check(ConvertStringSidToSid(user,out userSid));
     Check(SetPointerInfo(restricted,4,ref userSid,(uint)IntPtr.Size));
     var sd=new RawSecurityDescriptor("D:(A;;GA;;;SY)(A;;GA;;;"+user+")");
     var bytes=new byte[sd.DiscretionaryAcl.BinaryLength];sd.DiscretionaryAcl.GetBinaryForm(bytes,0);
     aclMemory=Marshal.AllocHGlobal(bytes.Length);Marshal.Copy(bytes,0,aclMemory,bytes.Length);
     Check(SetPointerInfo(restricted,6,ref aclMemory,(uint)IntPtr.Size));
    }finally{if(aclMemory!=IntPtr.Zero)Marshal.FreeHGlobal(aclMemory);if(userSid!=IntPtr.Zero)LocalFree(userSid);}
    var start=new Startup{cb=Marshal.SizeOf(typeof(Startup)),desktop=@"winsta0\default"};Info info;
    string exe=@"C:\GrammarPayload\pwsh\pwsh.exe";
    var command=new StringBuilder("\""+exe+"\" -NoProfile -File \""+args[0]+"\"");
    Check(CreateProcessAsUser(restricted,exe,command,IntPtr.Zero,IntPtr.Zero,false,0x08000400,IntPtr.Zero,@"C:\GrammarPayload",ref start,out info));
    try{uint wait=WaitForSingleObject(info.process,10000);uint code;GetExitCodeProcess(info.process,out code);File.WriteAllText(@"C:\GrammarOutput\restricted-exit.txt",wait+" "+code);File.WriteAllText(@"C:\GrammarOutput\restricted-launch.json","{\"pid\":"+info.pid+",\"targetIntegrity\":\"medium\",\"method\":\"own restricted LUA token\"}");}finally{CloseHandle(info.thread);CloseHandle(info.process);}
   }finally{if(sid!=IntPtr.Zero)LocalFree(sid);if(restricted!=IntPtr.Zero)CloseHandle(restricted);if(original!=IntPtr.Zero)CloseHandle(original);}
   return 0;
  }catch(Exception e){File.WriteAllText(@"C:\GrammarOutput\restricted-launch-error.txt",e.ToString());return 1;}
 }
}
