// Owned isolated RichEdit store, NOT Word/Office or installed-app qualification.
#include "../../src-tauri/native/tsf_selected_range.hpp"
#include "../../src-tauri/native/word_dispatcher.hpp"
#include "word-metadata-fixture.hpp"
#include <richedit.h>
#include <cstdio>
using namespace grammar;
static void pump(DWORD ms) {
    const auto end=GetTickCount64()+ms;
    do {MSG msg;while(PeekMessageW(&msg,nullptr,0,0,PM_REMOVE)){TranslateMessage(&msg);DispatchMessageW(&msg);}Sleep(5);}while(GetTickCount64()<end);
}
struct Session final:ITfEditSession {
    LONG refs=1;bool done=false;HRESULT result=E_PENDING;std::function<HRESULT(TfEditCookie)> work;
    explicit Session(std::function<HRESULT(TfEditCookie)> f):work(std::move(f)){}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(id==IID_IUnknown || id==IID_ITfEditSession){*out=static_cast<ITfEditSession*>(this);AddRef();return S_OK;}return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return InterlockedIncrement(&refs);}
    ULONG STDMETHODCALLTYPE Release() override{auto n=InterlockedDecrement(&refs);if(!n)delete this;return n;}
    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override {
        try{result=work(cookie);}catch(...){result=E_FAIL;}done=true;return result;
    }
};
static bool run(ITfContext* context,TfClientId client,DWORD flags,std::function<HRESULT(TfEditCookie)> work,HRESULT* disposition=nullptr) {
    auto session=new Session(std::move(work));HRESULT result=E_PENDING;
    const auto requested=context->RequestEditSession(client,session,TF_ES_ASYNC|flags,&result);
    if(disposition)*disposition=result;
    printf("request=%08lx disposition=%08lx flags=%lu done=%d\n",requested,result,flags,session->done);fflush(stdout);
    const bool queued=SUCCEEDED(requested) && result==TF_S_ASYNC;
    for(unsigned n=0;n<500 && !session->done && queued;++n)pump(10);
    const bool ok=SUCCEEDED(requested) && session->done && SUCCEEDED(session->result);
    // Do not continue and destroy callback captures if a store retained a late
    // callback. This isolated process terminates; it owns no external documents.
    if(!session->done && queued){puts("FAIL pending callback deadline");fflush(stdout);ExitProcess(9);}
    session->Release();return ok;
}
static std::wstring readFixture(HWND edit) {
    const int count=GetWindowTextLengthW(edit);std::vector<WCHAR> out(static_cast<size_t>(count)+1);
    GetWindowTextW(edit,out.data(),count+1);return std::wstring(out.data());
}
struct Port final:WordNativeWindowPort {
    Fixture* fixture;HWND field,root;bool live=true;
    Port(Fixture* f,HWND edit,HWND top):fixture(f),field(edit),root(top){}
    bool eligible(HWND edit,HWND top,DWORD thread) override{return live && edit==field && top==root && thread==GetCurrentThreadId();}
    HRESULT native_window(HWND edit,IDispatch** out) override {
        *out=nullptr;if(edit!=field)return E_INVALIDARG;*out=&fixture->window;fixture->window.AddRef();return S_OK;
    }
};
static auto deadline(){return std::chrono::steady_clock::now()+std::chrono::seconds(5);}
static bool drain(WordTargetDispatcher& actor) {
    for(unsigned n=0;n<500 && actor.pending();++n)pump(10);
    if(actor.pending()){puts("FAIL retained live callback deadline");fflush(stdout);ExitProcess(9);}return true;
}
int wmain() {
    WCHAR user[128]{};GetEnvironmentVariableW(L"USERNAME",user,128);
    if(std::wstring(user)!=L"WDAGUtilityAccount")return 2;
    if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
    int exitCode=0;
    {
        ComPtr<ITfThreadMgr> manager;TfClientId client=0;
        if(FAILED(CoCreateInstance(CLSID_TF_ThreadMgr,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&manager))) || FAILED(manager->Activate(&client)))return 4;
        const auto library=LoadLibraryW(L"Msftedit.dll");
        HWND top=CreateWindowExW(0,L"STATIC",L"Grammar Word actor fixture",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,650,300,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
        HWND edit=CreateWindowExW(0,L"RICHEDIT50W",L"hello world suffix",WS_CHILD|WS_VISIBLE|ES_MULTILINE|ES_AUTOVSCROLL,10,10,600,200,top,nullptr,GetModuleHandleW(nullptr),nullptr);
        if(!library || !top || !edit)return 5;
        SendMessageW(edit,EM_SETEDITSTYLE,SES_USECTF,SES_USECTF);SetForegroundWindow(top);SetFocus(edit);pump(300);
        ComPtr<ITfDocumentMgr> document;ComPtr<ITfContext> context;
        if(FAILED(manager->GetFocus(&document)) || !document || FAILED(document->GetTop(&context)) || !context)return 6;
        for(unsigned scenario=0;scenario<15;++scenario) {
            std::wstring expected=L"hello world suffix",selected;
            SetWindowTextW(edit,expected.c_str());SendMessageW(edit,EM_SETSEL,6,11);pump(20);
            Fixture f;f.hwnd=top;f.window.number(L"Hwnd",static_cast<LONG>(reinterpret_cast<INT_PTR>(top)));
            f.window.object(L"Application",&f.app);f.range.number(L"Start",6);f.range.number(L"End",11);
            f.range.allowText=true;f.range.text=L"world";f.range.values[L"Text"]={VT_BSTR,0,nullptr};
            auto port=std::make_shared<Port>(&f,edit,top);bool authorized=true;
            auto token=std::make_shared<int>(0);std::weak_ptr<void> lifetime=token;
            auto actor=std::make_unique<WordTargetDispatcher>(client,token);token.reset();
            auto begin=[&]{return actor->begin_capture(context.Get(),edit,top,port,true,[&]{return authorized;},deadline());};
            bool ok=true;unsigned mutations=0;
            if(scenario==8) {
                ok=!actor->begin_capture(context.Get(),edit,top,port,true,[&]{return authorized;},std::chrono::steady_clock::now()-std::chrono::seconds(1)) && !actor->pending();
            } else {
                ok=begin() && actor->pending();const auto oldId=actor->capture_id();auto id=oldId;
                ok=ok && id && !begin() && !actor->selected_text(id,selected) && selected.empty();
                if(scenario==1) {
                    ok=ok && actor->cancel_capture(id) && !actor->close() && !begin() && actor->pending();
                    drain(*actor);ok=ok && actor->capture_status()==WordDispatchStatus::rejected;
                } else {
                    drain(*actor);
                    ok=ok && actor->capture_status()==WordDispatchStatus::captured && actor->selected_text(id,selected) && selected==L"world";
                    ok=ok && !actor->selected_text(id+100,selected) && !actor->begin_accept(id+100,L"wrong",deadline());
                    if(scenario==5) {
                        ok=ok && begin();id=actor->capture_id();drain(*actor);
                        ok=ok && id>oldId && !actor->begin_accept(oldId,L"stale",deadline()) && actor->selected_text(id,selected);
                    }
                    if(scenario==9)ok=ok && !actor->cancel_capture(id+100);
                    if(scenario==10) {
                        bool refused=false;
                        std::thread other([&]{refused=!actor->begin_accept(id,L"wrong thread",deadline());});other.join();ok=ok && refused;
                    } else if(scenario==11) {
                        ok=ok && actor->close() && !actor->selected_text(id,selected) && !actor->begin_accept(id,L"closed",deadline());
                    } else if(scenario==12) {
                        authorized=false;ok=ok && !actor->selected_text(id,selected) && actor->capture_id()==0;
                    } else if(scenario==7) {
                        ok=ok && !actor->begin_accept(id,L"expired",std::chrono::steady_clock::now()-std::chrono::seconds(1));
                    } else {
                        if(scenario==6)SendMessageW(edit,EM_SETREADONLY,TRUE,0);
                        if(scenario==13)port->live=false;
                        const bool accepted=actor->begin_accept(id,L"reviewed",deadline());
                        if(scenario==6)ok=ok && !accepted && !actor->pending() && actor->accept_status()==WordDispatchStatus::rejected;
                        else {
                            ok=ok && accepted && actor->pending() && !actor->begin_accept(id,L"duplicate",deadline());
                            if(scenario==2)ok=ok && actor->cancel_capture(id) && !actor->close() && !begin();
                            if(scenario==3){expected=L"hello external change";SetWindowTextW(edit,expected.c_str());}
                            if(scenario==4)authorized=false;
                            drain(*actor);
                            const bool positive=scenario==0 || scenario==5 || scenario==9 || scenario==14;
                            if(positive)expected=L"hello reviewed suffix";
                            mutations=actor->mutations();
                            ok=ok && actor->accept_status()==(positive?WordDispatchStatus::applied:WordDispatchStatus::rejected) && mutations==(positive?1u:0u) &&
                                !actor->begin_accept(id,L"replay",deadline());
                        }
                        if(scenario==6)SendMessageW(edit,EM_SETREADONLY,FALSE,0);
                    }
                }
            }
            ok=ok && !actor->pending() && readFixture(edit)==expected && actor->close();
            actor.reset();ok=ok && lifetime.expired() && f.balanced();
            printf("case=%u pass=%d mutations=%u\n",scenario,ok,mutations);fflush(stdout);
            if(!ok)exitCode=1;
        }
        context.Reset();document.Reset();DestroyWindow(top);manager->Deactivate();manager.Reset();FreeLibrary(library);
    }
    CoUninitialize();printf("result=%s scope=Word_actor_real_TSF_inert_OM Office=NOT_RUN\n",exitCode?"FAIL":"PASS");return exitCode;
}
