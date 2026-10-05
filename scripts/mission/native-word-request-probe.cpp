// Owned isolated RichEdit store, NOT Word/Office or installed-app qualification.
#include "../../src-tauri/native/tsf_selected_range.hpp"
#include "../../src-tauri/native/word_edit_session.hpp"
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
static bool submit(ITfContext* context,TfClientId client,WordEditSession* session,DWORD flags,
    std::function<void()> beforePump={}) {
    HRESULT disposition=E_PENDING;
    const auto hr=context->RequestEditSession(client,session,TF_ES_ASYNC|flags,&disposition);
    if(FAILED(hr) || disposition!=TF_S_ASYNC || session->called)return false;
    if(beforePump)beforePump();
    for(unsigned n=0;n<500 && !session->completed;++n)pump(10);
    if(!session->completed){puts("FAIL live callback deadline");fflush(stdout);ExitProcess(9);}
    return true;
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
        HWND top=CreateWindowExW(0,L"STATIC",L"Grammar owned Word request fixture",WS_OVERLAPPEDWINDOW|WS_VISIBLE,30,30,650,300,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
        HWND edit=CreateWindowExW(0,L"RICHEDIT50W",L"hello world suffix world",WS_CHILD|WS_VISIBLE|ES_MULTILINE|ES_AUTOVSCROLL,10,10,600,200,top,nullptr,GetModuleHandleW(nullptr),nullptr);
        if(!library || !top || !edit)return 5;
        SendMessageW(edit,EM_SETEDITSTYLE,SES_USECTF,SES_USECTF);SetForegroundWindow(top);SetFocus(edit);pump(300);
        ComPtr<ITfDocumentMgr> document;ComPtr<ITfContext> context;
        if(FAILED(manager->GetFocus(&document)) || !document || FAILED(document->GetTop(&context)) || !context)return 6;
        for(unsigned scenario=0;scenario<17;++scenario) {
            std::wstring expected=L"hello world suffix world";
            SetWindowTextW(edit,expected.c_str());SendMessageW(edit,EM_SETSEL,6,11);pump(20);
            Fixture f;f.hwnd=top;f.window.number(L"Hwnd",static_cast<LONG>(reinterpret_cast<INT_PTR>(top)));
            f.window.object(L"Application",&f.app);f.range.number(L"Start",6);f.range.number(L"End",11);
            f.range.allowText=true;f.range.text=L"world";f.range.values[L"Text"]={VT_BSTR,0,nullptr};
            auto port=std::make_shared<Port>(&f,edit,top);bool authorized=scenario!=3;
            auto token=std::make_shared<int>(0);std::weak_ptr<void> lifetime=token;
            bool ok=true;unsigned mutations=0;
            {
                WordTsfRevisionBinding binding(context.Get(),edit,top,port,true,token,[&]{return authorized;});
                if(scenario==4)f.range.text=L"other";
                ComPtr<WordEditSession> capture;
                capture.Attach(new WordEditSession(binding.state,scenario==14?std::shared_ptr<void>{}:token,
                    std::chrono::steady_clock::now()+std::chrono::seconds(scenario==2?-1:5)));
                ok=submit(context.Get(),client,capture.Get(),TF_ES_READ,[&]{if(scenario==1)capture->cancel();});
                const bool captureRejected=scenario==1 || scenario==2 || scenario==3 || scenario==4 || scenario==14;
                ok=ok && static_cast<bool>(capture->captured)!=captureRejected;
                if(captureRejected) {
                    if(scenario!=4)ok=ok && capture->text_reads==0 && f.range.textReads==0;
                } else if(capture->captured) {
                    ok=ok && capture->captured->source()==L"world";
                    if(scenario==10) {
                        const auto before=binding.state->revision.load();
                        ok=ok && run(context.Get(),client,TF_ES_READWRITE,[&](TfEditCookie cookie){
                            ComPtr<ITfRange> range;
                            if(!selected_range_detail::selection(context.Get(),cookie,range))return E_FAIL;
                            auto hr=range->SetText(cookie,TF_ST_CORRECTION,L"other",5);
                            return FAILED(hr)?hr:range->SetText(cookie,TF_ST_CORRECTION,L"world",5);
                        }) && binding.state->revision.load()>before && readFixture(edit)==expected;
                    }
                    if(scenario==16)capture->cancel();
                    ComPtr<WordEditSession> apply;
                    apply.Attach(new WordEditSession(capture->captured,L"reviewed",scenario==15?std::shared_ptr<void>{}:token,
                        std::chrono::steady_clock::now()+std::chrono::seconds(scenario==6?-1:5)));
                    if(scenario==8)f.range.onInvoke=[&](const std::wstring& name){if(name==L"Text")apply->cancel();};
                    ok=ok && submit(context.Get(),client,apply.Get(),TF_ES_READWRITE,[&]{
                        if(scenario==5)apply->cancel();
                        if(scenario==7 || scenario==13)authorized=false;
                        if(scenario==9){expected=L"hello changed externally";SetWindowTextW(edit,expected.c_str());}
                        if(scenario==11)SendMessageW(edit,EM_SETSEL,19,24);
                    });
                    const bool positive=scenario==0 || scenario==12;
                    if(positive)expected=L"hello reviewed suffix world";
                    mutations=apply->mutations;
                    ok=ok && mutations==(positive?1u:0u) && apply->result==(positive?ApplyResult::applied:ApplyResult::rejected);
                    f.range.onInvoke={};
                    ComPtr<WordEditSession> replay;
                    replay.Attach(new WordEditSession(capture->captured,L"replay",token,std::chrono::steady_clock::now()+std::chrono::seconds(5)));
                    ok=ok && submit(context.Get(),client,replay.Get(),TF_ES_READWRITE) && replay->mutations==0 && replay->result==ApplyResult::rejected;
                }
                ok=ok && readFixture(edit)==expected;
                capture.Reset();token.reset();
                ok=ok && binding.close() && binding.disconnected() && lifetime.expired();
            }
            ok=ok && f.balanced();
            printf("case=%u pass=%d mutations=%u word_reads=%u\n",scenario,ok,mutations,f.range.textReads);fflush(stdout);
            if(!ok)exitCode=1;
        }
        context.Reset();document.Reset();DestroyWindow(top);manager->Deactivate();manager.Reset();FreeLibrary(library);
    }
    CoUninitialize();printf("result=%s scope=real_TSF_requests_inert_Word Office=NOT_RUN\n",exitCode?"FAIL":"PASS");return exitCode;
}
