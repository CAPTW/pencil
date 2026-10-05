// Real RichEdit/TSF deadline regressions; task-owned restricted Sandbox only.
#include "../../src-tauri/native/tsf_dispatcher.hpp"
#include <cstdio>
using namespace grammar;
static void pump(){MSG m;while(PeekMessageW(&m,nullptr,0,0,PM_REMOVE)){TranslateMessage(&m);DispatchMessageW(&m);}}
int wmain(){
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount" || !same_local_identity(GetCurrentProcess()))return 2;
    if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
    int failures=0;
    {
        ComPtr<ITfThreadMgr> manager;TfClientId client=0;
        if(FAILED(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager)))||FAILED(manager->Activate(&client)))return 4;
        auto library=LoadLibraryExW(L"msctf.dll",nullptr,LOAD_LIBRARY_SEARCH_SYSTEM32);
        auto rich=LoadLibraryExW(L"Msftedit.dll",nullptr,LOAD_LIBRARY_SEARCH_SYSTEM32);
        using SetScope=HRESULT(WINAPI*)(HWND,InputScope);
        auto setScope=reinterpret_cast<SetScope>(GetProcAddress(library,"SetInputScope"));if(!setScope||!rich)return 5;
        for(int test=0;test<9;++test){
            HWND top=CreateWindowW(L"STATIC",L"Grammar owned RichEdit deadline",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,600,250,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
            HWND field=CreateWindowW(L"RICHEDIT50W",L"hello world",WS_CHILD|WS_VISIBLE|ES_MULTILINE,10,10,550,150,top,nullptr,GetModuleHandleW(nullptr),nullptr);
            if(!top||!field)return 6;
            ShowWindow(top,SW_SHOW);ShowWindow(top,SW_SHOW);
            SendMessageW(field,EM_SETEDITSTYLE,SES_USECTF,SES_USECTF);
            SetForegroundWindow(top);SetFocus(field);
            if(FAILED(setScope(field,IS_TEXT)))return 7;
            SendMessageW(field,EM_SETSEL,6,11);
            auto ready=GetTickCount64()+300;do{pump();Sleep(5);}while(GetTickCount64()<ready);
            ComPtr<ITfDocumentMgr> document;ComPtr<ITfContext> context;
            bool bound=SUCCEEDED(manager->GetFocus(&document))&&document&&SUCCEEDED(document->GetTop(&context))&&context;
            OwnedApplyPipe pipe;ChannelHandle peer(CreateFileW(pipe.name.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,0,nullptr));
            if(!ConnectNamedPipe(pipe.get(),nullptr)&&GetLastError()!=ERROR_PIPE_CONNECTED)return 8;
            auto lease=std::make_shared<ApplyConnectionLease>(GetCurrentProcess(),pipe.get());if(!lease->alive())return 9;
            {
                TsfTargetDispatcher dispatcher(client);
                auto capDeadline=std::chrono::steady_clock::now()+std::chrono::milliseconds(test==1?-1:test==2?50:5000);
                auto capTicks=test==5?GetTickCount64()-1:test==6?GetTickCount64()+50:MAXULONGLONG;
                bool began=bound&&dispatcher.begin_capture(context.Get(),field,true,lease,capDeadline,capTicks);
                bool capQueued=began&&dispatcher.capture_status()==DispatchStatus::pending;
                if(test==2||test==6)Sleep(100);
                auto wait=GetTickCount64()+3000;
                while(began&&dispatcher.capture_status()==DispatchStatus::pending&&GetTickCount64()<wait){pump();Sleep(5);}
                if(dispatcher.capture_status()==DispatchStatus::pending)return 10;
                auto id=dispatcher.capture_id();std::wstring selected;bool captured=dispatcher.selected_text(id,selected);
                auto appDeadline=std::chrono::steady_clock::now()+std::chrono::milliseconds(test==3?-1:test==4?50:5000);
                auto appTicks=test==7?GetTickCount64()-1:test==8?GetTickCount64()+50:MAXULONGLONG;
                bool started=dispatcher.begin_apply(id,L"Grammar",appDeadline,appTicks);
                bool appQueued=started&&dispatcher.apply_status()==DispatchStatus::pending;
                if(test==4||test==8)Sleep(100);
                wait=GetTickCount64()+3000;
                while(started&&dispatcher.apply_status()==DispatchStatus::pending&&GetTickCount64()<wait){pump();Sleep(5);}
                if(dispatcher.apply_status()==DispatchStatus::pending)return 11;
                WCHAR actual[128]{};GetWindowTextW(field,actual,128);
                bool captureCase=test==1||test==2||test==5||test==6;
                bool pass=bound&&(captureCase?!captured&&dispatcher.capture_reads()==0:captured&&selected==L"world")&&
                    (test==0?dispatcher.apply_status()==DispatchStatus::applied&&dispatcher.mutation_calls()==1&&std::wstring(actual)==L"hello Grammar":
                    dispatcher.apply_status()==DispatchStatus::rejected&&dispatcher.mutation_calls()==0&&std::wstring(actual)==L"hello world");
                if(test==1||test==5)pass=pass&&!began;
                if(test==2||test==6)pass=pass&&capQueued;
                if(test==3||test==7)pass=pass&&!started;
                if(test==4||test==8)pass=pass&&appQueued;
                printf("case=%d pass=%d cap_queued=%d apply_queued=%d reads=%u mutations=%u\n",test,pass,capQueued,appQueued,dispatcher.capture_reads(),dispatcher.mutation_calls());fflush(stdout);
                if(!pass)++failures;
                dispatcher.retire_completed_apply();dispatcher.cancel();
            }
            DestroyWindow(top);pump();
        }
        manager->Deactivate();manager.Reset();FreeLibrary(library);FreeLibrary(rich);
    }
    CoUninitialize();printf("cases=9 failures=%d\n",failures);return failures?1:0;
}
