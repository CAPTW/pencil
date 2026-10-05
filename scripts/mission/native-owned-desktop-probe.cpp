// Guest-only experiment: existing native suite on a fresh interactive desktop.
#include "../../src-tauri/native/apply_channel.hpp"
#include <cstdio>
using namespace grammar;
int wmain() {
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount" || !same_local_identity(GetCurrentProcess()))return 2;
    auto original=OpenInputDesktop(0,FALSE,DESKTOP_READOBJECTS|DESKTOP_SWITCHDESKTOP);
    auto threadDesktop=GetThreadDesktop(GetCurrentThreadId());
    if(!original || !threadDesktop)return 3;
    const auto name=L"Grammar.Native.Sequence."+std::to_wstring(GetCurrentProcessId())+L"."+std::to_wstring(GetTickCount64());
    auto existing=OpenDesktopW(name.c_str(),0,FALSE,DESKTOP_READOBJECTS);
    if(existing){CloseDesktop(existing);CloseDesktop(original);return 4;}
    auto owned=CreateDesktopW(name.c_str(),nullptr,nullptr,0,
        DESKTOP_CREATEWINDOW|DESKTOP_READOBJECTS|DESKTOP_WRITEOBJECTS|DESKTOP_SWITCHDESKTOP|DESKTOP_HOOKCONTROL|DESKTOP_ENUMERATE,nullptr);
    if(!owned){printf("create_desktop_error=%lu\n",GetLastError());CloseDesktop(original);return 5;}
    ChannelHandle job(CreateJobObjectW(nullptr,nullptr));
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limit{};limit.BasicLimitInformation.LimitFlags=JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    bool configured=job.get() && SetInformationJobObject(job.get(),JobObjectExtendedLimitInformation,&limit,sizeof(limit));
    bool switched=configured && SwitchDesktop(owned);
    bool launched=false,exited=false,empty=false,forced=false;DWORD childExit=999;
    if(switched) {
        // Use the same task-owned PowerShell runtime as MediumLaunch, not the
        // OS PowerShell 5.1 whose Process receipt behavior differs.
        const std::wstring executable=L"C:\\GrammarPayload\\pwsh\\pwsh.exe";
        auto command=L"\""+executable+L"\" -NoProfile -ExecutionPolicy Bypass -File C:\\GrammarPayload\\Guest-WordRoute-Native.ps1";
        auto desktop=L"WinSta0\\"+name;
        STARTUPINFOW startup{};startup.cb=sizeof(startup);startup.lpDesktop=desktop.data();
        startup.dwFlags=STARTF_USESHOWWINDOW;startup.wShowWindow=SW_HIDE;
        PROCESS_INFORMATION info{};
        launched=CreateProcessW(executable.c_str(),command.data(),nullptr,nullptr,FALSE,CREATE_SUSPENDED|CREATE_NO_WINDOW,nullptr,nullptr,&startup,&info)!=FALSE;
        if(launched) {
            ChannelHandle process(info.hProcess),thread(info.hThread);
            bool assigned=AssignProcessToJobObject(job.get(),process.get())!=FALSE;
            if(assigned && ResumeThread(thread.get())!=static_cast<DWORD>(-1)) {
                exited=WaitForSingleObject(process.get(),300000)==WAIT_OBJECT_0;
                if(exited)GetExitCodeProcess(process.get(),&childExit);
                auto deadline=GetTickCount64()+2000;
                do{JOBOBJECT_BASIC_ACCOUNTING_INFORMATION accounting{};
                    empty=QueryInformationJobObject(job.get(),JobObjectBasicAccountingInformation,&accounting,sizeof(accounting),nullptr) && !accounting.ActiveProcesses;
                    if(empty)break;Sleep(10);
                }while(GetTickCount64()<deadline);
            }
            if(!exited || !empty){forced=true;if(assigned)TerminateJobObject(job.get(),99);else TerminateProcess(process.get(),99);WaitForSingleObject(process.get(),5000);}
        }
    }
    const bool restored=SwitchDesktop(original)!=FALSE;
    const bool threadRestored=SetThreadDesktop(threadDesktop)!=FALSE;
    job.reset();
    const bool desktopClosed=CloseDesktop(owned)!=FALSE;
    const bool originalClosed=CloseDesktop(original)!=FALSE;
    printf("configured=%d switched=%d launched=%d exited=%d child_exit=%lu job_empty=%d forced=%d restored=%d thread_restored=%d desktop_closed=%d original_closed=%d\n",
        configured,switched,launched,exited,childExit,empty,forced,restored,threadRestored,desktopClosed,originalClosed);
    // This is only launcher/cleanup success. Native case results remain separate.
    return configured&&switched&&launched&&exited&&childExit==0&&empty&&!forced&&restored&&threadRestored&&desktopClosed&&originalClosed?0:1;
}
