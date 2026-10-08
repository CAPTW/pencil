// Real Win32 hook lifetime regression. No user input, external window or hook callback dispatch.
#include "../../src-tauri/native/target_apply_client.hpp"
#include <cstdio>
#include <thread>
static LRESULT CALLBACK callback(int code, WPARAM wp, LPARAM lp) { return CallNextHookEx(nullptr,code,wp,lp); }
int main() {
    HHOOK hook=SetWindowsHookExW(WH_CALLWNDPROC,callback,nullptr,GetCurrentThreadId());
    if(!hook) return 10;
    HHOOK removed=hook;
    if(!UnhookWindowsHookEx(hook)) return 11;
    SetLastError(ERROR_SUCCESS);
    if(UnhookWindowsHookEx(removed)) return 12;
    DWORD error=GetLastError();
    std::printf("expired_hook_error=%lu\n",error);
    if(error!=ERROR_INVALID_HOOK_HANDLE) return 13;
    if(!grammar::release_target_hook(removed)||removed) return 20;
    if(!grammar::release_target_hook(removed)) return 21;
    hook=SetWindowsHookExW(WH_CALLWNDPROC,callback,nullptr,GetCurrentThreadId());
    if(!hook) return 22;
    if(!grammar::release_target_hook(hook)||hook) return 23;
    std::thread owner([&hook] {hook=SetWindowsHookExW(WH_CALLWNDPROC,callback,nullptr,GetCurrentThreadId());});
    owner.join();
    if(!hook) return 24;
    SetLastError(ERROR_SUCCESS);
    if(UnhookWindowsHookEx(hook)||GetLastError()!=ERROR_INVALID_HOOK_HANDLE) return 25;
    if(!grammar::release_target_hook(hook)||hook) return 26;
    std::puts("PASS expired hook, repeated release, live owned hook, terminated hook thread");return 0;
}
