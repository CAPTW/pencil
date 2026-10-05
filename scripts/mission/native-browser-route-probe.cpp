// Owned Sandbox only. No text reads, clipboard, injected input or document writes.
#include "../../src-tauri/native/browser_route.hpp"
#include <cstdio>
using namespace grammar;
int wmain(int argc,wchar_t** argv) {
    if(argc!=2)return 2;
    WCHAR user[128]{};DWORD units=128;
    if(!GetUserNameW(user,&units) || _wcsicmp(user,L"WDAGUtilityAccount"))return 3;
    ChannelHandle file(CreateFileW(argv[1],GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,nullptr));
    ApplyImageDigest digest{};const auto deadline=GetTickCount64()+5000;
    if(file.get()==INVALID_HANDLE_VALUE || !apply_image_digest(file.get(),digest,deadline))return 4;
    HWND window=GetForegroundWindow();
    DWORD host=0,browser=0;GetWindowThreadProcessId(window,&browser);
    ChannelHandle snapshot(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS,0));
    PROCESSENTRY32W item{};item.dwSize=sizeof(item);
    if(!Process32FirstW(snapshot.get(),&item))return 5;
    do {
        
        auto candidate=verify_apply_peer(item.th32ProcessID,argv[1],digest,deadline);
        if(candidate) {
            printf("host_candidate=%lu parent=%lu foreground_browser=%lu\n",item.th32ProcessID,item.th32ParentProcessID,browser);
            DWORD parent=item.th32ParentProcessID;
            for(int depth=0;depth<3 && parent;++depth) {
                ChannelHandle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,parent));
                WCHAR image[32768]{};DWORD length=32768;
                if(process.get() && QueryFullProcessImageNameW(process.get(),0,image,&length))
                    printf("ancestor_depth=%d pid=%lu image=%ls\n",depth,parent,image);
                parent=browser_host_parent(parent,deadline);
            }
            if(host)return 6;host=item.th32ProcessID;
        }
    } while(Process32NextW(snapshot.get(),&item));
    auto route=verify_browser_route(host,window,argv[1],digest,deadline);
    unsigned failed=0,count=0;
    auto check=[&](const char* name,bool ok){++count;if(!ok)++failed;printf("%s=%s\n",name,ok?"PASS":"FAIL");};
    check("real_chromium_native_host_parent_route",route && route->foreground() && route->host_pid()==host && route->browser_pid()==browser);
    auto bad=digest;bad[0]^=1;
    check("wrong_host_hash",!verify_browser_route(host,window,argv[1],bad,deadline));
    check("zero_host",!verify_browser_route(0,window,argv[1],digest,deadline));
    check("zero_parent_window_is_not_identity",!verify_browser_route(host,nullptr,argv[1],digest,deadline));
    check("browser_not_host",!verify_browser_route(browser,window,argv[1],digest,deadline));
    check("expired_absolute_deadline",!verify_browser_route(host,window,argv[1],digest,GetTickCount64()));
    check("wrong_installation_path",!verify_browser_route(host,window,L"C:\\GrammarPayload\\absent-host.exe",digest,deadline));
    check("current_process_cannot_impersonate_host",!verify_browser_route(GetCurrentProcessId(),window,argv[1],digest,deadline));
    // Exact host image, but launched by this owned probe instead of Chromium.
    // Suspended startup makes rejection deterministic without any protocol work.
    STARTUPINFOW startup{};startup.cb=sizeof(startup);PROCESS_INFORMATION child{};
    std::wstring command=L"\""+std::wstring(argv[1])+L"\" invalid-origin";
    bool created=CreateProcessW(argv[1],command.data(),nullptr,nullptr,FALSE,CREATE_SUSPENDED|CREATE_NO_WINDOW,nullptr,nullptr,&startup,&child)!=FALSE;
    bool refused=false,exited=false;
    if(created) {
        ChannelHandle process(child.hProcess),thread(child.hThread);
        refused=!verify_browser_route(child.dwProcessId,window,argv[1],digest,GetTickCount64()+5000);
        ResumeThread(thread.get());exited=WaitForSingleObject(process.get(),3000)==WAIT_OBJECT_0;
    }
    check("trusted_host_wrong_parent",created && refused && exited);
    HWND owned=CreateWindowExW(0,L"STATIC",L"Grammar owned route fixture",WS_OVERLAPPEDWINDOW|WS_VISIBLE,80,80,300,180,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
    if(owned)SetForegroundWindow(owned);
    check("review_focus_is_not_capture_foreground",owned && GetForegroundWindow()==owned && route && route->alive() && !route->foreground());
    check("unrelated_owned_window_rejected",owned && !verify_browser_route(host,owned,argv[1],digest,GetTickCount64()+5000));
    if(owned)DestroyWindow(owned);
    printf("checks=%u failures=%u host=%lu browser=%lu\n",count,failed,host,browser);
    return failed?1:0;
}
