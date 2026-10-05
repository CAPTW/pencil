// Physical shortcut input only inside the owned Medium Sandbox browser fixture.
#include "../../src-tauri/native/apply_peer.hpp"
#include <cstdio>
int wmain() {
    WCHAR user[128]{};DWORD size=128;
    if(!GetUserNameW(user,&size) || wcscmp(user,L"WDAGUtilityAccount") || !grammar::same_local_identity(GetCurrentProcess()))return 2;
    HWND window=GetForegroundWindow();DWORD pid=0;WCHAR title[256]{},klass[64]{};
    if(!window || !GetWindowThreadProcessId(window,&pid) || !GetWindowTextW(window,title,256) ||
        !wcsstr(title,L"Grammar desktop browser fixture") || !GetClassNameW(window,klass,64) || wcscmp(klass,L"Chrome_WidgetWin_1"))return 3;
    grammar::ChannelHandle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,pid));
    WCHAR path[32768]{};size=32768;
    if(!process.get() || !QueryFullProcessImageNameW(process.get(),0,path,&size) ||
        _wcsicmp(path,L"C:\\GrammarPayload\\chromium-1234\\chrome-win64\\chrome.exe"))return 4;
    INPUT events[6]{};const WORD keys[]={VK_CONTROL,VK_SHIFT,'G','G',VK_SHIFT,VK_CONTROL};
    for(unsigned i=0;i<6;i++){events[i].type=INPUT_KEYBOARD;events[i].ki.wVk=keys[i];events[i].ki.dwFlags=i>=3?KEYEVENTF_KEYUP:0;}
    if(GetForegroundWindow()!=window)return 5;
    const UINT sent=SendInput(6,events,sizeof(INPUT));
    if(sent!=6){SendInput(3,events+3,sizeof(INPUT));return 6;}
    printf("SENT 6\n");return 0;
}
