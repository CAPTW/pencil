#include "../../src-tauri/native/target_apply_client.hpp"
#include <msctf.h>
#include <wrl/client.h>
#include <richedit.h>
#include <inputscope.h>
#include <cstdio>
#include <future>
using namespace grammar;
using Microsoft::WRL::ComPtr;
static void fixture_pump() {MSG message;while(PeekMessageW(&message,nullptr,0,0,PM_REMOVE)){TranslateMessage(&message);DispatchMessageW(&message);}}
template<class Action> static auto fixture_wait(Action action) {
    // Match the app architecture: blocking native I/O is not the review UI
    // thread. Pump only this fixture's own windows while the worker runs.
    auto worker=std::async(std::launch::async,std::move(action));
    while(worker.wait_for(std::chrono::milliseconds(0))!=std::future_status::ready){fixture_pump();Sleep(1);}
    return worker.get();
}
static LRESULT CALLBACK target_proc(HWND hwnd, UINT message, WPARAM wp, LPARAM lp) {
    // Synthetic fixture setup only: let its owning broker show the review UI.
    if (message == WM_APP+42) return AllowSetForegroundWindow(static_cast<DWORD>(wp));
    if (message == WM_APP+43) {
        HWND field=FindWindowExW(hwnd,nullptr,L"RICHEDIT50W",nullptr);
        if(!field)field=FindWindowExW(hwnd,nullptr,L"Edit",nullptr);
        if (!field || GetForegroundWindow()!=hwnd) return 0;
        SetFocus(field); return GetFocus()==field;
    }
    if(message==WM_APP+44) {
        // Synthetic target reads its own password field after the product has
        // refused capture. Windows does not expose it through external WM_GETTEXT.
        HWND field=FindWindowExW(hwnd,nullptr,L"Edit",nullptr);
        WCHAR value[128]{};
        return field&&GetWindowTextW(field,value,128)&&std::wstring(value)==L"hello world";
    }
    if(message==WM_APP+45) {
        HWND field=FindWindowExW(hwnd,nullptr,L"RICHEDIT50W",nullptr);
        if(!field)field=FindWindowExW(hwnd,nullptr,L"Edit",nullptr);
        if(!field)return 0;
        SetForegroundWindow(hwnd);SetFocus(field);
        return GetForegroundWindow()==hwnd&&GetFocus()==field;
    }
    if(message==WM_APP+46) {
        HWND field=FindWindowExW(hwnd,nullptr,L"RICHEDIT50W",nullptr);
        if(!field)field=FindWindowExW(hwnd,nullptr,L"Edit",nullptr);
        if(!field||wp>0xffff)return 0;
        SendMessageW(field,EM_SETPASSWORDCHAR,wp,0);
        return SendMessageW(field,EM_GETPASSWORDCHAR,0,0)==static_cast<LRESULT>(wp);
    }
    if (message == WM_DESTROY) { PostQuitMessage(0); return 0; }
    return DefWindowProcW(hwnd,message,wp,lp);
}
static int target(int scenario) {
    const bool multiline=scenario>=200;
    const bool standard=scenario>=100;
    if(standard)scenario-=multiline?200:100;
    if (FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED))) return 20;
    ComPtr<ITfThreadMgr> manager; TfClientId client = TF_CLIENTID_NULL;
    if (FAILED(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager))) || FAILED(manager->Activate(&client))) return 21;
    LoadLibraryExW(L"Msftedit.dll",nullptr,LOAD_LIBRARY_SEARCH_SYSTEM32);
    auto input = LoadLibraryExW(L"msctf.dll",nullptr,LOAD_LIBRARY_SEARCH_SYSTEM32);
    using SetScope = HRESULT(WINAPI*)(HWND,InputScope);
    auto setScope = reinterpret_cast<SetScope>(GetProcAddress(input,"SetInputScope"));
    if (!setScope) return 22;
    WNDCLASSW wc{}; wc.lpfnWndProc=target_proc; wc.hInstance=GetModuleHandleW(nullptr); wc.lpszClassName=L"GrammarTargetServiceFixture";
    if (!RegisterClassW(&wc)) return 23;
    HWND top = CreateWindowW(wc.lpszClassName,L"Grammar owned target",WS_OVERLAPPEDWINDOW|WS_VISIBLE,
        30,30,600,250,nullptr,nullptr,wc.hInstance,nullptr);
    HWND edit = CreateWindowW(standard?L"Edit":L"RICHEDIT50W",L"hello world",WS_CHILD|WS_VISIBLE|(multiline?ES_MULTILINE|ES_AUTOVSCROLL:standard?ES_AUTOHSCROLL:ES_MULTILINE),
        10,10,550,150,top,nullptr,wc.hInstance,nullptr);
    if (!top || !edit) return 24;
    ShowWindow(top,SW_SHOW); ShowWindow(top,SW_SHOW);
    if(!standard)SendMessageW(edit,EM_SETEDITSTYLE,SES_USECTF,SES_USECTF);
    if (scenario != 3 && FAILED(setScope(edit,IS_TEXT))) return 25;
    if (scenario == 4) SendMessageW(edit,EM_SETPASSWORDCHAR,L'*',0);
    SetForegroundWindow(top); SetFocus(edit); SendMessageW(edit,EM_SETSEL,6,11);
    SetPropW(top,L"GrammarReady",reinterpret_cast<HANDLE>(1));
    MSG message;
    while (GetMessageW(&message,nullptr,0,0)>0) { TranslateMessage(&message); DispatchMessageW(&message); }
    manager->Deactivate(); manager.Reset(); FreeLibrary(input); CoUninitialize(); return 0;
}
static bool send(HWND field, UINT message, WPARAM wp, LPARAM lp) {
    DWORD_PTR result=0;
    return SendMessageTimeoutW(field,message,wp,lp,SMTO_ABORTIFHUNG|SMTO_BLOCK,2000,&result)!=0;
}
int wmain(int argc,wchar_t** argv) {
    WCHAR user[128]{}; GetEnvironmentVariableW(L"USERNAME",user,128);
    if (std::wstring(user)!=L"WDAGUtilityAccount" || !same_local_identity(GetCurrentProcess())) return 2;
    if (argc==3 && std::wstring(argv[1])==L"--target") return target(_wtoi(argv[2]));
    const bool reviewOnly=argc==2 && std::wstring(argv[1])==L"--standard-review";
    const bool multiline=argc==2 && std::wstring(argv[1])==L"--multiline";
    const bool standard=multiline||reviewOnly||(argc==2 && std::wstring(argv[1])==L"--standard");
    HMODULE module=LoadLibraryExW(L"C:\\GrammarPayload\\target-apply.dll",nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!module) { printf("dll_error=%lu\n",GetLastError()); return 3; }
    WCHAR executable[32768]{}; GetModuleFileNameW(nullptr,executable,32768);
    using Create = uint32_t(WINAPI*)(HWND,uint64_t*);
    using Capture = uint32_t(WINAPI*)(uint64_t,WCHAR*,uint32_t,uint32_t*);
    using Apply = uint32_t(WINAPI*)(uint64_t,const WCHAR*,uint32_t,uint32_t*);
    using Cancel = uint32_t(WINAPI*)(uint64_t);
    using Close = uint32_t(WINAPI*)(uint64_t,uint32_t);
    auto create=reinterpret_cast<Create>(GetProcAddress(module,"GrammarTargetCreate"));
    auto captureRaw=reinterpret_cast<Capture>(GetProcAddress(module,"GrammarTargetCapture"));
    auto applyRaw=reinterpret_cast<Apply>(GetProcAddress(module,"GrammarTargetApply"));
    auto cancel=reinterpret_cast<Cancel>(GetProcAddress(module,"GrammarTargetCancel"));
    auto closeRaw=reinterpret_cast<Close>(GetProcAddress(module,"GrammarTargetClose"));
    if (!create||!captureRaw||!applyRaw||!cancel||!closeRaw) return 5;
    auto capture=[&](uint64_t id,WCHAR* text,uint32_t capacity,uint32_t* units){return fixture_wait([&]{return captureRaw(id,text,capacity,units);});};
    auto apply=[&](uint64_t id,const WCHAR* text,uint32_t units,uint32_t* outcome){return fixture_wait([&]{return applyRaw(id,text,units,outcome);});};
    auto close=[&](uint64_t id,uint32_t milliseconds){return fixture_wait([&]{return closeRaw(id,milliseconds);});};
    // An owned broker window makes foreground handoff deterministic between
    // scenarios. Do not borrow Explorer or an unrelated foreground process.
    HWND controller=CreateWindowW(L"STATIC",L"Grammar owned fixture controller",WS_OVERLAPPEDWINDOW|WS_VISIBLE,
        650,30,300,200,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
    if(!controller)return 6;
    ShowWindow(controller,SW_SHOW);ShowWindow(controller,SW_SHOW);SetForegroundWindow(controller);
    int failures=0;
    for (int scenario=reviewOnly?13:0;scenario<14;++scenario) {
        auto command=L"\""+std::wstring(executable)+L"\" --target "+std::to_wstring(scenario+(multiline?200:standard?100:0));
        STARTUPINFOW startup{}; startup.cb=sizeof(startup); startup.dwFlags=STARTF_USESHOWWINDOW; startup.wShowWindow=SW_HIDE;
#ifdef GRAMMAR_NATIVE_TEST_DIAGNOSTICS
        startup.dwFlags|=STARTF_USESTDHANDLES;
        startup.hStdOutput=GetStdHandle(STD_OUTPUT_HANDLE);startup.hStdError=GetStdHandle(STD_ERROR_HANDLE);startup.hStdInput=GetStdHandle(STD_INPUT_HANDLE);
#endif
        PROCESS_INFORMATION info{};
        if (!CreateProcessW(executable,command.data(),nullptr,nullptr,
#ifdef GRAMMAR_NATIVE_TEST_DIAGNOSTICS
            TRUE,
#else
            FALSE,
#endif
            CREATE_NO_WINDOW,nullptr,nullptr,&startup,&info)) return 4;
        ChannelHandle process(info.hProcess),thread(info.hThread);
        struct Search { DWORD pid; HWND window; } search{info.dwProcessId,nullptr};
        auto end=GetTickCount64()+3000;
        do {
            fixture_pump();
            EnumWindows([](HWND hwnd,LPARAM data)->BOOL {
                auto search=reinterpret_cast<Search*>(data); DWORD pid=0; GetWindowThreadProcessId(hwnd,&pid);
                WCHAR name[64]{}; GetClassNameW(hwnd,name,64);
                if (pid==search->pid && std::wstring(name)==L"GrammarTargetServiceFixture") { search->window=hwnd; return FALSE; }
                return TRUE;
            },reinterpret_cast<LPARAM>(&search));
            if (search.window && GetPropW(search.window,L"GrammarReady")) break;
            Sleep(10);
        } while (GetTickCount64()<end);
        HWND field=search.window?FindWindowExW(search.window,nullptr,standard?L"Edit":L"RICHEDIT50W",nullptr):nullptr;
        bool pass=field && GetPropW(search.window,L"GrammarReady");
        DWORD owner=0;GetWindowThreadProcessId(GetForegroundWindow(),&owner);
        if(owner==GetCurrentProcessId())AllowSetForegroundWindow(info.dwProcessId);
        DWORD_PTR focused=0;
        const bool ready=pass;
        const bool focusReply=pass&&SendMessageTimeoutW(search.window,WM_APP+45,0,0,SMTO_ABORTIFHUNG,2000,&focused);
        pass=focusReply&&focused==1;
#ifdef GRAMMAR_NATIVE_TEST_DIAGNOSTICS
        printf("fixture_admission case=%d ready=%d focus_reply=%d focused=%llu foreground_target=%d\n",scenario,ready,focusReply,static_cast<unsigned long long>(focused),GetForegroundWindow()==search.window);fflush(stdout);
#else
        (void)ready;
#endif
        bool captured=false, applied=false, closed=false; uint32_t outcome=99;
        if (scenario>=8) {
            uint64_t cookie=0,blocked=0;
            pass=pass&&create(field,&cookie)==0&&cookie!=0;
            pass=pass&&create(field,&blocked)==2&&blocked==0;
            if (scenario==11) {
                pass=pass&&cancel(cookie)==0;
            }
            std::wstring selected(65535,L'\0'); uint32_t units=0;
            auto result=capture(cookie,selected.data(),static_cast<uint32_t>(selected.size()),&units);
            selected.resize(units); captured=result==0&&selected==L"world";
            pass=pass&&(scenario==11?result==4:captured);
            HWND review=nullptr;
            if (scenario==13 && captured) {
                pass=send(search.window,WM_APP+42,GetCurrentProcessId(),0)&&pass;
                review=CreateWindowW(L"STATIC",L"Owned review window",WS_OVERLAPPEDWINDOW|WS_VISIBLE,
                    100,100,400,200,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
                const bool activated=review && SetForegroundWindow(review);
                const bool foreground=review && GetForegroundWindow()==review;
                printf("review case=%d activation_return=%d actual_foreground=%d prior_pass=%d\n",scenario,activated,foreground,pass);
                pass=pass&&foreground;
                Sleep(100);
            }
            if (scenario!=11 && captured) {
                if (scenario==9) pass=send(field,WM_SETTEXT,0,reinterpret_cast<LPARAM>(L"application edit"))&&pass;
                if (scenario==10) pass=pass&&cancel(cookie)==0;
                uint32_t ignored=99;
                pass=pass&&apply(cookie+100,L"Grammar",7,&ignored)==1;
                result=apply(cookie,L"Grammar",7,&outcome);
                applied=result==0&&outcome==3;
                pass=pass&&(scenario==8||scenario==12||scenario==13?applied:scenario==9?(result==0&&outcome==0):(result==4&&outcome==4));
                pass=pass&&apply(cookie,L"Grammar",7,&ignored)==0&&ignored==0;
            }
            if(review) DestroyWindow(review);
            closed=close(cookie,5000)==0;
            pass=pass&&closed&&close(cookie,0)==1;
            // A closed slot must be reusable, including cancellation before I/O.
            pass=pass&&create(field,&blocked)==0&&cancel(blocked)==0&&close(blocked,0)==0;
        } else {
            TargetApplyClient client;fixture_wait([&]{client.connect_to(field,module);return client.connected;});pass=pass&&client.connected;
            TargetReply reply; std::wstring selected;
            bool response=fixture_wait([&]{return client.request(ChannelCommand::capture,0,L"",reply,selected);});
            uint64_t id=reply.capture_id;
            captured=response&&reply.status==2&&selected==L"world";
            bool denied=scenario==3||scenario==4;
            pass=pass&&response&&(denied?(reply.status==0&&selected.empty()):captured);
            if (!denied && scenario!=5 && captured) {
                HWND review=nullptr;
                if (scenario==7) {
                    pass=send(search.window,WM_APP+42,GetCurrentProcessId(),0)&&pass;
                    review=CreateWindowW(L"STATIC",L"Owned wrong-token review",WS_OVERLAPPEDWINDOW|WS_VISIBLE,
                        100,100,400,200,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
                    if(review) SetForegroundWindow(review);
                    pass=pass&&review&&GetForegroundWindow()==review;
                }
                if (scenario==1) pass=send(field,WM_SETTEXT,0,reinterpret_cast<LPARAM>(L"application edit"))&&pass;
                if (scenario==2) pass=send(field,EM_SETSEL,0,5)&&pass;
                response=fixture_wait([&]{return client.request(ChannelCommand::apply,scenario==7?999:id,L"Grammar",reply,selected);});
                outcome=reply.status; applied=response&&reply.status==3;
                bool normal=scenario==0||scenario==6;
                pass=pass&&response&&(normal?applied:reply.status==0);
                if(review) { pass=pass&&GetForegroundWindow()==review; DestroyWindow(review); }
                if (scenario==6) {
                    bool replay=fixture_wait([&]{return client.request(ChannelCommand::apply,id,L"Grammar",reply,selected);});
                    pass=pass&&replay&&reply.status==0;
                }
            }
            closed=fixture_wait([&]{return client.close(5000);});pass=pass&&closed;
        }
        WCHAR actual[128]{};
        bool reread=send(field,WM_GETTEXT,ARRAYSIZE(actual),reinterpret_cast<LPARAM>(actual));
        auto expected=scenario==0||scenario==6||scenario==8||scenario==12||scenario==13?L"hello Grammar":scenario==1||scenario==9?L"application edit":L"hello world";
        bool matched=reread&&std::wstring(actual)==expected;
        if(standard&&scenario==4) {
            DWORD_PTR ownRead=0;
            matched=SendMessageTimeoutW(search.window,WM_APP+44,0,0,SMTO_ABORTIFHUNG|SMTO_BLOCK,2000,&ownRead)&&ownRead==1;
        }
        pass=pass&&matched;
        if(search.window) {
            send(search.window,WM_APP+42,GetCurrentProcessId(),0);
            SetForegroundWindow(controller);
            PostMessageW(search.window,WM_CLOSE,0,0);
        }
        bool exited=fixture_wait([&]{return WaitForSingleObject(process.get(),5000)==WAIT_OBJECT_0;});
        DWORD code=STILL_ACTIVE; GetExitCodeProcess(process.get(),&code);
        pass=pass&&exited&&code==0;
        printf("case=%d pass=%d captured=%d applied=%d outcome=%u worker_closed=%d reread=%d target_pid=%lu natural_exit=%d exit=%lu\n",
            scenario,pass,captured,applied,outcome,closed,matched,info.dwProcessId,exited,code);
        if (!exited) { TerminateProcess(process.get(),99); WaitForSingleObject(process.get(),3000); ++failures; break; }
        if (!pass) ++failures;
    }
    DestroyWindow(controller);FreeLibrary(module); printf("failures=%d\n",failures); return failures?1:0;
}
