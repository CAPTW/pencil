// Real owned TSF and kernel pipe lease; INERT Word model, not Office acceptance.
#include "../../src-tauri/native/desktop_dispatcher.hpp"
#include "word-events-fixture.hpp"
#include <richedit.h>
#include <cstdio>
using namespace grammar;
static void pump(DWORD ms) {
    const auto end=GetTickCount64()+ms;
    do{MSG msg;while(PeekMessageW(&msg,nullptr,0,0,PM_REMOVE)){TranslateMessage(&msg);DispatchMessageW(&msg);}Sleep(5);}while(GetTickCount64()<end);
}
static void drain(DesktopTargetDispatcher& actor) {
    for(unsigned n=0;n<500 && actor.executing();++n)pump(10);
    if(actor.executing()){puts("FAIL callback retained past fixture deadline");fflush(stdout);ExitProcess(9);}
}
static std::wstring readFixture(HWND edit) {
    std::vector<WCHAR> text(static_cast<size_t>(GetWindowTextLengthW(edit))+1);
    GetWindowTextW(edit,text.data(),static_cast<int>(text.size()));return text.data();
}
struct Port final:WordNativeWindowPort {
    Fixture* f;HWND field,root;
    Port(Fixture* value,HWND edit,HWND top):f(value),field(edit),root(top){}
    bool eligible(HWND edit,HWND top,DWORD thread) override{return edit==field && top==root && thread==GetCurrentThreadId();}
    HRESULT native_window(HWND,IDispatch** out) override{*out=&f->window;f->window.AddRef();return S_OK;}
};
static auto deadline(){return std::chrono::steady_clock::now()+std::chrono::seconds(5);}
int wmain() {
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount")return 2;
    if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
    int failed=0;
    {
        ComPtr<ITfThreadMgr> manager;TfClientId client=0;
        if(FAILED(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager))) || FAILED(manager->Activate(&client)))return 4;
        const auto library=LoadLibraryW(L"Msftedit.dll");
        HWND top=CreateWindowExW(0,L"STATIC",L"Grammar Word connection fixture",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,650,300,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
        HWND edit=CreateWindowExW(0,L"RICHEDIT50W",L"hello world suffix",WS_CHILD|WS_VISIBLE|ES_MULTILINE,10,10,600,200,top,nullptr,GetModuleHandleW(nullptr),nullptr);
        if(!library || !top || !edit)return 5;
        // Start-Process hides the probe initially; explicitly show its owned
        // synthetic editor before testing production foreground admission.
        ShowWindow(top,SW_SHOW);ShowWindow(top,SW_SHOW);
        SendMessageW(edit,EM_SETEDITSTYLE,SES_USECTF,SES_USECTF);SetForegroundWindow(top);SetFocus(edit);pump(300);
        ComPtr<ITfDocumentMgr> document;ComPtr<ITfContext> context;
        if(FAILED(manager->GetFocus(&document)) || !document || FAILED(document->GetTop(&context)) || !context)return 6;
        for(unsigned scenario=0;scenario<15;++scenario) {
            std::wstring expected=L"hello world suffix",selected;
            SetWindowTextW(edit,expected.c_str());SendMessageW(edit,EM_SETSEL,6,11);pump(20);
            const auto name=L"\\\\.\\pipe\\Grammar.WordConnection."+std::to_wstring(GetCurrentProcessId())+L"."+std::to_wstring(scenario);
            ChannelHandle server(CreateNamedPipeW(name.c_str(),PIPE_ACCESS_DUPLEX|FILE_FLAG_FIRST_PIPE_INSTANCE,PIPE_TYPE_BYTE|PIPE_READMODE_BYTE|PIPE_NOWAIT|PIPE_REJECT_REMOTE_CLIENTS,1,4096,4096,0,nullptr));
            ChannelHandle peer(CreateFileW(name.c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,0,nullptr));
            if(!server.get() || !peer.get())return 7;
            auto lease=std::make_shared<ApplyConnectionLease>(GetCurrentProcess(),server.get());
            if(!lease->alive())return 8;
            std::weak_ptr<ApplyConnectionLease> weakLease=lease;
            Fixture f;Events events;f.app.extension=static_cast<IConnectionPointContainer*>(&events);
            f.hwnd=top;f.window.number(L"Hwnd",static_cast<LONG>(reinterpret_cast<INT_PTR>(top)));f.window.object(L"Application",&f.app);
            f.range.number(L"Start",6);f.range.number(L"End",11);f.range.allowText=true;f.range.text=L"world";f.range.values[L"Text"]={VT_BSTR,0,nullptr};
            auto port=std::make_shared<Port>(&f,edit,top);auto token=std::make_shared<int>(0);std::weak_ptr<void> weakToken=token;
            auto actor=std::make_unique<DesktopTargetDispatcher>(client,top,port,token);token.reset();
            auto begin=[&]{return actor->begin_capture(context.Get(),edit,true,lease,deadline());};
            bool ok=true;
            if(scenario==7)ok=!actor->begin_capture(context.Get(),edit,true,{},deadline());
            else if(scenario==8)ok=!actor->begin_capture(context.Get(),edit,false,lease,deadline());
            else if(scenario==9)ok=!actor->begin_capture(context.Get(),edit,true,lease,std::chrono::steady_clock::now());
            else if(scenario==13){bool refused=false;events.onFind=[&]{refused=!actor->close();};ok=!begin() && refused;events.onFind={};}
            else {
                ok=begin();auto id=actor->capture_id();ok=ok && id && !begin();
                if(scenario==1)lease->revoke();
                drain(*actor);
                if(scenario==1)ok=ok && !actor->selected_text(id,selected) && !actor->begin_apply(id,L"changed",deadline());
                else {
                    const bool read=actor->selected_text(id,selected),can=actor->can_apply(id);
                    const bool wrong=actor->activate(id,top,GetCurrentProcessId()),active=actor->activate(id,edit,GetCurrentProcessId());
                    printf("read=%d can=%d wrong=%d active=%d foreground=%d focus=%d visible=%d\n",read,can,wrong,active,GetForegroundWindow()==top,GetFocus()==edit,IsWindowVisible(top));
                    ok=ok && read && selected==L"world" && can && !wrong && active;
                    ok=ok && !actor->begin_capture(context.Get(),edit,true,lease) && !actor->begin_apply(id,L"missing deadline");
                    if(scenario==2)lease->revoke();
                    if(scenario==3)peer.reset();
                    if(scenario==10)events.fire();
                    if(scenario==6 || scenario==14) {
                        events.closeResult=E_FAIL;ok=ok && !actor->close() && !begin();
                        lease.reset();ok=ok && !weakLease.expired();events.closeResult=S_OK;
                        ok=ok && actor->close() && weakLease.expired();
                    } else if(scenario==2 || scenario==3 || scenario==10)ok=ok && !actor->begin_apply(id,L"changed",deadline());
                    else {
                        if(scenario==11)ok=ok && !actor->begin_apply(id+1,L"wrong",deadline());
                        if(scenario==12){ok=ok && begin();auto newer=actor->capture_id();drain(*actor);ok=ok && newer>id && !actor->begin_apply(id,L"stale",deadline());id=newer;}
                        ok=ok && actor->begin_apply(id,L"reviewed",deadline());
                        if(scenario==4)lease->revoke();
                        if(scenario==5)ok=ok && actor->cancel_capture(id) && !actor->close();
                        drain(*actor);const bool positive=scenario==0 || scenario==11 || scenario==12;
                        if(positive)expected=L"hello reviewed suffix";
                        ok=ok && actor->apply_status()==(positive?DispatchStatus::applied:DispatchStatus::rejected);
                        ok=ok && !actor->begin_apply(id,L"replay",deadline());
                    }
                }
            }
            ok=ok && readFixture(edit)==expected && actor->close();actor.reset();lease.reset();
            ok=ok && weakLease.expired() && weakToken.expired() && f.balanced() && events.refs==1 && !events.sink;
            printf("case=%u pass=%d\n",scenario,ok);fflush(stdout);if(!ok)++failed;
        }
        context.Reset();document.Reset();DestroyWindow(top);manager->Deactivate();manager.Reset();FreeLibrary(library);
    }
    CoUninitialize();printf("result=%s scope=Desktop_Word_route_real_TSF_pipe_inert_OM Office=NOT_RUN\n",failed?"FAIL":"PASS");return failed?1:0;
}
