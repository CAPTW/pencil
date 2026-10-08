// Production cleanup helper with injected Unhook failure and real process handles.
#include "../../src-tauri/native/target_apply_protocol.hpp"
static BOOL WINAPI failed_unhook(HHOOK) { SetLastError(ERROR_ACCESS_DENIED); return FALSE; }
#define UnhookWindowsHookEx failed_unhook
#include "../../src-tauri/native/target_apply_client.hpp"
#undef UnhookWindowsHookEx
#include <cstdio>
int wmain(int argc,wchar_t**) {
    if(argc>1)return 0;
    HHOOK hook=reinterpret_cast<HHOOK>(1);
    if(grammar::release_target_hook(hook)||!hook)return 10;
    if(grammar::release_target_hook(hook,GetCurrentProcess())||!hook)return 11;
    HANDLE invalid=reinterpret_cast<HANDLE>(static_cast<intptr_t>(0x123456));
    if(grammar::release_target_hook(hook,invalid)||!hook)return 12;
    wchar_t image[32768]{};
    if(!GetModuleFileNameW(nullptr,image,32768))return 13;
    std::wstring command=L"\""+std::wstring(image)+L"\" --child";
    STARTUPINFOW startup{};startup.cb=sizeof(startup);PROCESS_INFORMATION child{};
    if(!CreateProcessW(image,command.data(),nullptr,nullptr,FALSE,CREATE_NO_WINDOW,nullptr,nullptr,&startup,&child))return 14;
    DWORD wait=WaitForSingleObject(child.hProcess,5000);
    const bool released=wait==WAIT_OBJECT_0&&grammar::release_target_hook(hook,child.hProcess)&&!hook;
    CloseHandle(child.hThread);CloseHandle(child.hProcess);
    if(!released)return 15;
    std::puts("PASS missing/live/unavailable process refused; confirmed exited owned process released");return 0;
}
