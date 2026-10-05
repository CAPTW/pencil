// Owned desktop peer: real browser route and host transport; no UI automation.
#include "../../src-tauri/native/browser_bridge.hpp"
#include "../../src-tauri/native/browser_route.hpp"
#include <cstdio>
using namespace grammar;
int wmain(int argc,wchar_t** argv) {
    WCHAR user[128]{};DWORD size=128;
    if(argc!=3 || !GetUserNameW(user,&size) || wcscmp(user,L"WDAGUtilityAccount"))return 2;
    const DWORD expected=wcstoul(argv[2],nullptr,10);if(expected!=0 && expected!=3 && expected!=4)return 3;
#ifdef GRAMMAR_BROWSER_CLIENT_ABI
    std::wstring library=argv[1];library=library.substr(0,library.find_last_of(L"\\/"))+L"\\target-apply.dll";
    HMODULE module=LoadLibraryExW(library.c_str(),nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_SYSTEM32);
    if(!module)return 4;
    using Create=uint32_t(WINAPI*)(HWND,uint64_t*);
    using Capture=uint32_t(WINAPI*)(uint64_t,WCHAR*,uint32_t,uint32_t*);
    using Apply=uint32_t(WINAPI*)(uint64_t,const WCHAR*,uint32_t,uint32_t*);
    using Close=uint32_t(WINAPI*)(uint64_t,uint32_t);
    auto create=reinterpret_cast<Create>(GetProcAddress(module,"GrammarBrowserClientCreate"));
    auto capture=reinterpret_cast<Capture>(GetProcAddress(module,"GrammarBrowserClientCapture"));
    auto apply=reinterpret_cast<Apply>(GetProcAddress(module,"GrammarBrowserClientApply"));
    auto close=reinterpret_cast<Close>(GetProcAddress(module,"GrammarBrowserClientClose"));
    if(!create || !capture || !apply || !close)return 5;
    uint64_t cookie=0,duplicate=0;WCHAR captured[8193]{};uint32_t units=0,outcome=4;
    if(create(nullptr,&cookie)!=3 || create(GetForegroundWindow(),&cookie)!=0 || !cookie ||
        create(GetForegroundWindow(),&duplicate)!=2 || duplicate || close(cookie+1,0)!=1 ||
        capture(cookie+1,captured,8193,&units)!=1 || capture(cookie,captured,8192,&units)!=3 ||
        apply(cookie,L"early",5,&outcome)!=0 || outcome!=0)return 6;
    if(capture(cookie,captured,8193,&units)!=0 || std::wstring(captured,units)!=L"seperate")return 10;
    SecureZeroMemory(captured,sizeof(captured));
#else
    const auto deadline=GetTickCount64()+3000;
    ChannelHandle file(CreateFileW(argv[1],GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,nullptr));
    ApplyImageDigest hash{};if(file.get()==INVALID_HANDLE_VALUE || !apply_image_digest(file.get(),hash,deadline))return 4;
    const HWND foreground=GetForegroundWindow();
    ChannelHandle snapshot(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS,0));
    PROCESSENTRY32W entry{};entry.dwSize=sizeof(entry);
    if(snapshot.get()==INVALID_HANDLE_VALUE || !Process32FirstW(snapshot.get(),&entry))return 5;
    std::unique_ptr<BrowserProcessRoute> route;
    unsigned visited=0;
    do {
        if(++visited>65536 || GetTickCount64()>=deadline)return 6;
        if(_wcsicmp(entry.szExeFile,L"grammar-chromium-host.exe"))continue;
        // Only the dedicated review actor creates this pipe. Other Instant/Deep
        // host processes never receive a speculative capture request.
        if(!WaitNamedPipeW(browser_bridge_name(entry.th32ProcessID).c_str(),1))continue;
        auto candidate=verify_browser_route(entry.th32ProcessID,foreground,argv[1],hash,deadline);
        if(candidate){if(route)return 7;route=std::move(candidate);}
    }while(Process32NextW(snapshot.get(),&entry));
    if(!route || !route->foreground())return 8;
    BrowserBridgeClient client;
    if(!client.connect(verify_apply_peer(route->host_pid(),argv[1],hash,deadline),deadline))return 9;
    TargetReply reply;std::wstring source;
    if(!client.request(ChannelCommand::capture,L"",reply,source) || reply.status!=2 || source!=L"seperate" || !route->foreground())return 10;
#endif
    printf("captured=1\n");fflush(stdout);
    // The fixture must explicitly release this synthetic Accept. No document
    // text is placed in argv, environment, disk or stdout.
    const auto acceptDeadline=GetTickCount64()+3000;char accept=0;
    while(GetTickCount64()<acceptDeadline){
        DWORD available=0,read=0;
        if(!PeekNamedPipe(GetStdHandle(STD_INPUT_HANDLE),nullptr,0,nullptr,&available,nullptr))return 11;
        if(available){if(!ReadFile(GetStdHandle(STD_INPUT_HANDLE),&accept,1,&read,nullptr) || read!=1 || accept!='A')return 12;break;}
        Sleep(2);
    }
    if(accept!='A')return 13;
#ifdef GRAMMAR_BROWSER_CLIENT_ABI
    const bool replied=apply(cookie,L"\xAC80\xD1A0 \xD83D\xDE42",5,&outcome)==0;
    const bool correct=replied && outcome==expected;
    const bool replay=apply(cookie,L"replay",5,&outcome)==0 && outcome==0;
    const bool closed=close(cookie,8000)==0 && close(cookie,0)==1;
    FreeLibrary(module);if(!closed)return 15;
#else
    if(!route->alive())return 13;
    const bool replied=client.request(ChannelCommand::apply,L"\xAC80\xD1A0 \xD83D\xDE42",reply,source);
    const bool correct=replied && reply.status==expected;
    const bool replay=!client.request(ChannelCommand::apply,L"replay",reply,source);
#endif
    printf("outcome_correct=%d replay_refused=%d\n",correct,replay);
    return correct && replay?0:14;
}
