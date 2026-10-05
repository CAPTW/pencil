// Owned restricted Sandbox only. Real TSF metadata, no fabricated plain scope.
#include "../../src-tauri/native/standard_edit_dispatcher.hpp"
#include <cstdio>
using namespace grammar;
static void pump() { MSG m; while(PeekMessageW(&m,nullptr,0,0,PM_REMOVE)){TranslateMessage(&m);DispatchMessageW(&m);} }
int wmain(int argc,wchar_t** argv) {
    const bool autovscroll=argc==2&&std::wstring(argv[1])==L"--autovscroll";
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount")return 2;
    if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
    int failures=0;
    {
        ComPtr<ITfThreadMgr> manager;TfClientId client=0;
        if(FAILED(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager)))||FAILED(manager->Activate(&client)))return 4;
        auto library=LoadLibraryExW(L"msctf.dll",nullptr,LOAD_LIBRARY_SEARCH_SYSTEM32);
        using SetScope=HRESULT(WINAPI*)(HWND,InputScope);
        auto setScope=reinterpret_cast<SetScope>(GetProcAddress(library,"SetInputScope"));if(!setScope)return 5;
        for(int test=0;test<22;++test) {
            HWND top=CreateWindowW(L"STATIC",L"Grammar owned Edit scope",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,600,250,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
            HWND field=CreateWindowW(L"Edit",L"hello world",WS_CHILD|WS_VISIBLE|(autovscroll?ES_MULTILINE|ES_AUTOVSCROLL:0),10,10,550,100,top,nullptr,GetModuleHandleW(nullptr),nullptr);
            ShowWindow(top,SW_SHOW);ShowWindow(top,SW_SHOW);SetForegroundWindow(top);SetFocus(field);
            if(test!=2&&FAILED(setScope(field,test==1?IS_PASSWORD:IS_TEXT)))return 6;
            SendMessageW(field,EM_SETSEL,6,11);
            auto until=GetTickCount64()+200;do{pump();Sleep(5);}while(GetTickCount64()<until);
            ComPtr<ITfDocumentMgr> document;ComPtr<ITfContext> context;
            bool bound=SUCCEEDED(manager->GetFocus(&document))&&document&&SUCCEEDED(document->GetTop(&context))&&context;
            OwnedApplyPipe pipe;
            ChannelHandle peer(CreateFileW(pipe.name.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,0,nullptr));
            if(!ConnectNamedPipe(pipe.get(),nullptr)&&GetLastError()!=ERROR_PIPE_CONNECTED)return 7;
            auto lease=std::make_shared<ApplyConnectionLease>(GetCurrentProcess(),pipe.get());
            if(!lease->alive())return 8;
            StandardEditDispatcher dispatcher(client);
            const auto captureDeadline=std::chrono::steady_clock::now()+std::chrono::milliseconds(test==18?-1:test==19?50:5000);
            bool began=bound&&dispatcher.begin_capture(context.Get(),field,true,lease,captureDeadline);
            if(test==19)Sleep(100); // Intentionally withhold STA pumping until expired.
            auto deadline=GetTickCount64()+3000;
            while(began&&dispatcher.capture_status()==DispatchStatus::pending&&GetTickCount64()<deadline){pump();Sleep(5);}
            if(dispatcher.capture_status()==DispatchStatus::pending){printf("pending capture deadline\n");fflush(stdout);ExitProcess(30);}
            std::wstring selected;
            auto id=dispatcher.capture_id();
            bool captured=dispatcher.selected_text(id,selected);
            if(test==3||test==5){setScope(field,IS_PASSWORD);pump();}
            if(test==5){setScope(field,IS_TEXT);pump();}
            if(test==4)lease->revoke();
            if(test==6){SetWindowTextW(field,L"changed");SetWindowTextW(field,L"hello world");SendMessageW(field,EM_SETSEL,6,11);}
            if(test==7){SendMessageW(field,EM_SETSEL,0,5);SendMessageW(field,EM_SETSEL,6,11);}
            if(test==8)dispatcher.cancel_capture(id);
            if(test==10){setScope(field,IS_DEFAULT);pump();}
            if(test==11){setScope(field,IS_TEXT);pump();}
            if(test>=12&&test<=17) {
                bool called=false;
                bool activated=dispatcher.with_activation(test==12?id+1:id,[&]{
                    called=true;
                    if(test==17){if(dispatcher.begin_apply(id,L"reentrant"))++failures;return true;}
                    if(test==14)setScope(field,IS_PASSWORD);else setScope(field,IS_DEFAULT);
                    pump();
                    if(test==16)lease->revoke();
                    if(test!=15){setScope(field,IS_TEXT);pump();}
                    return true;
                });
                if(test==12&&called)++failures;
                if(test==13||test==17?!activated:(test!=12&&activated))++failures;
            }
            const auto applyDeadline=std::chrono::steady_clock::now()+std::chrono::milliseconds(test==20?-1:test==21?50:5000);
            bool started=dispatcher.begin_apply(test==9?id+1:id,L"Grammar",applyDeadline);
            if(test==21)Sleep(100); // Queued write must reject before mutation.
            deadline=GetTickCount64()+3000;
            while(started&&dispatcher.apply_status()==DispatchStatus::pending&&GetTickCount64()<deadline){pump();Sleep(5);}
            if(dispatcher.apply_status()==DispatchStatus::pending){printf("pending apply deadline\n");fflush(stdout);ExitProcess(31);}
            auto result=dispatcher.apply_status();
            WCHAR text[128]{};GetWindowTextW(field,text,128);
            bool pass=(test==1||test==2||test==18||test==19?!captured:captured&&selected==L"world")&&
                (test==0||test==11||test==12||test==13||test==17?result==DispatchStatus::applied&&std::wstring(text)==L"hello Grammar":result==DispatchStatus::rejected&&std::wstring(text)==L"hello world");
            if(test==0||test==11||test==12||test==13||test==17)pass=pass&&!dispatcher.begin_apply(id,L"replay");
            if(test==18)pass=pass&&!began;
            if(test==19)pass=pass&&began;
            if(test==20)pass=pass&&!started;
            if(test==21)pass=pass&&started;
            dispatcher.retire_completed_apply();dispatcher.cancel();
            printf("dispatcher_case=%d pass=%d bound=%d capture=%d started=%d result=%u\n",test,pass,bound,captured,started,static_cast<unsigned>(result));
            if(!pass)++failures;
            DestroyWindow(top);pump();
        }
        manager->Deactivate();manager.Reset();FreeLibrary(library);
    }
    CoUninitialize();printf("failures=%d\n",failures);return failures?1:0;
}
